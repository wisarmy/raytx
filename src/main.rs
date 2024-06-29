use anyhow::Result;
use clap::{ArgGroup, Parser, Subcommand};
use raytx::{
    get_rpc_client, get_wallet, logger,
    raydium::get_pool_info,
    swap::{self, SwapDirection, SwapInType},
    swap_ts, token,
};
use std::str::FromStr;
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
    #[command(about = "swap the mint token by ts")]
    #[command(group(
        ArgGroup::new("amount")
            .required(true)
            .args(&["amount_in", "amount_in_pct"]),
    ))]
    SwapTs {
        mint: String,
        #[arg(value_enum)]
        direction: SwapDirection,
        #[arg(long, help = "amount in")]
        amount_in: Option<f64>,
        #[arg(long, help = "amount in percentage, only support sell")]
        amount_in_pct: Option<f64>,
    },
    #[command(about = "swap the mint token by rs")]
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
    let cli = Cli::parse();
    logger::init();
    let client = get_rpc_client()?;
    let wallet = get_wallet()?;

    match &cli.command {
        Some(Command::SwapTs {
            mint,
            direction,
            amount_in,
            amount_in_pct,
        }) => {
            let (amount_in, in_type) = if let Some(amount_in) = amount_in {
                (amount_in, SwapInType::Qty)
            } else if let Some(amount_in) = amount_in_pct {
                (amount_in, SwapInType::Pct)
            } else {
                panic!("either in_amount or in_amount_pct must be provided");
            };

            debug!("{} {:?} {:?} {:?}", mint, direction, amount_in, in_type);
            let swapx = swap_ts::Swap::new(client, wallet.pubkey(), dotenvy::var("SWAP_ADDR").ok());
            swapx
                .swap(mint, *amount_in, direction.clone(), in_type)
                .await?;
        }
        Some(Command::Swap {
            mint,
            direction,
            amount_in,
            amount_in_pct,
        }) => {
            let (amount_in, in_type) = if let Some(amount_in) = amount_in {
                (amount_in, SwapInType::Qty)
            } else if let Some(amount_in) = amount_in_pct {
                (amount_in, SwapInType::Pct)
            } else {
                panic!("either in_amount or in_amount_pct must be provided");
            };
            let slippage = dotenvy::var("SLIPPAGE").unwrap_or("5".to_string());
            let slippage = slippage.parse::<u64>().unwrap_or(5);
            debug!(
                "{} {:?} {:?} {:?} slippage: {}",
                mint, direction, amount_in, in_type, slippage
            );
            let swapx = swap::Swap::new(client, wallet);
            swapx
                .swap2(mint, *amount_in, direction.clone(), in_type, slippage)
                .await?;
        }

        Some(Command::Token(token_command)) => match token_command {
            TokenCommand::List => {
                let token_accounts = token::token_accounts(&client, &wallet.pubkey()).await;
                info!("token_accounts: {:#?}", token_accounts);
            }
            TokenCommand::Show { mint } => {
                let mint = Pubkey::from_str(mint).expect("failed to parse mint pubkey");
                let token_account = token::token_account(&client, &wallet.pubkey(), mint).await?;
                info!("token_account: {:#?}", token_account);
                let pool_info = get_pool_info(
                    &spl_token::native_mint::id().to_string(),
                    &token_account.mint,
                )
                .await?;
                let pool_id = pool_info.get_pool_id().unwrap();
                info!("pool id: {}", pool_id);
            }
        },
        _ => {}
    }
    Ok(())
}
