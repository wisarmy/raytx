use anyhow::Result;
use clap::{ArgGroup, Parser, Subcommand};
use raytx::{
    get_rpc_client, get_wallet, logger,
    raydium::get_pool_info,
    swap::{self, SwapDirection, SwapInType},
    swap_rs, token,
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
    #[command(about = "swap the mint token")]
    #[command(group(
        ArgGroup::new("amount")
            .required(true)
            .args(&["in_amount", "in_amount_pct"]),
    ))]
    Swap {
        mint: String,
        #[arg(value_enum)]
        direction: SwapDirection,
        #[arg(long, help = "in amount")]
        in_amount: Option<f64>,
        #[arg(long, help = "in amount percentage, only support sell")]
        in_amount_pct: Option<f64>,
    },
    #[command(about = "swap the mint token")]
    #[command(group(
        ArgGroup::new("amount")
            .required(true)
            .args(&["in_amount", "in_amount_pct"]),
    ))]
    SwapByRust {
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
        Some(Command::Swap {
            mint,
            direction,
            in_amount,
            in_amount_pct,
        }) => {
            let (in_amount, in_type) = if let Some(in_amount) = in_amount {
                (in_amount, SwapInType::Qty)
            } else if let Some(in_amount) = in_amount_pct {
                (in_amount, SwapInType::Pct)
            } else {
                panic!("either in_amount or in_amount_pct must be provided");
            };

            debug!("{} {:?} {:?} {:?}", mint, direction, in_amount, in_type);
            let swapx = swap::Swap::new(client, wallet.pubkey(), dotenvy::var("SWAP_ADDR").ok());
            swapx
                .swap(mint, *in_amount, direction.clone(), in_type)
                .await?;
        }
        Some(Command::SwapByRust {
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

            let swapx = swap_rs::Swap::new(client, wallet);
            swapx
                .swap(mint, *amount_in, direction.clone(), in_type)
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
