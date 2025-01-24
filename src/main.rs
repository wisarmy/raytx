use anyhow::Result;
use axum::{
    http::{HeaderValue, Method},
    routing::{get, post},
    Router,
};
use clap::{ArgGroup, Parser, Subcommand};
use raytx::{
    api::{self, AppState},
    get_rpc_client, get_rpc_client_blocking, get_wallet, jito, logger,
    raydium::get_pool_info,
    swap::{self, SwapDirection, SwapInType},
    token,
};
use std::{env, net::SocketAddr, str::FromStr};
use tower_http::cors::CorsLayer;
use tracing::{debug, info};

use solana_sdk::{pubkey::Pubkey, signature::Signer};

#[derive(Parser)]
#[command(name = "raytx", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "swap the mint token")]
    #[command(group(
        ArgGroup::new("amount")
            .required(true)
            .args(&["amount_in", "amount_in_pct"]),
    ))]
    Swap {
        mint: String,
        #[arg(value_enum)]
        direction: SwapDirection,
        #[arg(long, help = "amount in")]
        amount_in: Option<f64>,
        #[arg(long, help = "amount in percentage, only support sell")]
        amount_in_pct: Option<f64>,
        #[arg(long, help = "use jito to swap", default_value_t = false)]
        jito: bool,
    },
    Daemon {
        #[arg(
            long,
            help = "Start a long-running daemon process for swap",
            default_value = "127.0.0.1:7235"
        )]
        addr: String,
    },
    #[command(about = "Wrap sol -> wsol")]
    Wrap {},
    #[command(about = "Unwrap wsol -> sol")]
    Unwrap {},
    #[command(subcommand)]
    Token(TokenCommand),
}

#[derive(Subcommand, Debug)]
enum TokenCommand {
    #[command(about = "List your wallet token accounts")]
    List,
    #[command(about = "Show token account from arg mint")]
    Show {
        #[arg(help = "The mint address of the token")]
        mint: String,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    if let Ok(env_path) = env::var("DOTENV_PATH") {
        println!("Using env_path: {}", env_path);
        dotenvy::from_path(env_path).ok();
    } else {
        dotenvy::dotenv().ok();
    }
    let cli = Cli::parse();
    logger::init();
    let client = get_rpc_client()?;
    let client_blocking = get_rpc_client_blocking()?;
    let wallet = get_wallet()?;
    let app_state = AppState {
        client,
        client_blocking,
        wallet,
    };

    match &cli.command {
        Some(Command::Swap {
            mint,
            direction,
            amount_in,
            amount_in_pct,
            jito,
        }) => {
            let (amount_in, in_type) = if let Some(amount_in) = amount_in {
                (amount_in, SwapInType::Qty)
            } else if let Some(amount_in) = amount_in_pct {
                (amount_in, SwapInType::Pct)
            } else {
                panic!("either in_amount or in_amount_pct must be provided");
            };
            let slippage = env::var("SLIPPAGE").unwrap_or("5".to_string());
            let slippage = slippage.parse::<u64>().unwrap_or(5);
            debug!(
                "{} {:?} {:?} {:?} slippage: {}",
                mint, direction, amount_in, in_type, slippage
            );
            // jito
            if *jito {
                jito::init_tip_accounts()
                    .await
                    .map_err(|err| {
                        info!("failed to get tip accounts: {:?}", err);
                        err
                    })
                    .unwrap();
                jito::init_tip_amounts()
                    .await
                    .map_err(|err| {
                        info!("failed to init tip amounts: {:?}", err);
                        err
                    })
                    .unwrap();
            }

            swap::swap(
                app_state,
                mint,
                *amount_in,
                direction.clone(),
                in_type,
                slippage,
                *jito,
            )
            .await?;
        }
        Some(Command::Daemon { addr }) => {
            jito::init_tip_accounts().await.unwrap();
            tokio::spawn(async {
                jito::ws::tip_stream()
                    .await
                    .expect("Failed to get tip percentiles data");
            });

            let app = Router::new()
                .nest(
                    "/api",
                    Router::new()
                        .route("/swap", post(api::swap))
                        .route("/pool/:pool_id", get(api::get_pool))
                        .route("/coins/:mint", get(api::coins))
                        .route("/token_accounts", get(api::token_accounts))
                        .route("/token_accounts/:mint", get(api::token_account))
                        .with_state(app_state),
                )
                .layer(
                    CorsLayer::new()
                        .allow_origin("*".parse::<HeaderValue>().unwrap())
                        .allow_methods([
                            Method::GET,
                            Method::POST,
                            Method::PUT,
                            Method::OPTIONS,
                            Method::DELETE,
                        ]),
                );

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            info!("listening on {}", listener.local_addr().unwrap());
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        }
        Some(Command::Token(token_command)) => match token_command {
            TokenCommand::List => {
                let token_accounts =
                    token::token_accounts(&app_state.client, &app_state.wallet.pubkey()).await;
                info!("token_accounts: {:#?}", token_accounts);
            }
            TokenCommand::Show { mint } => {
                let mint = Pubkey::from_str(mint).expect("failed to parse mint pubkey");
                let token_account =
                    token::token_account(&app_state.client, &app_state.wallet.pubkey(), mint)
                        .await?;
                info!("token_account: {:#?}", token_account);
                let pool_info = get_pool_info(
                    &spl_token::native_mint::id().to_string(),
                    &token_account.mint,
                )
                .await?;
                let pool_id = pool_info.get_pool().unwrap().id;
                info!("pool id: {}", pool_id);
            }
        },
        _ => {}
    }
    Ok(())
}
