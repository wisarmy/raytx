use anyhow::Result;
use clap::{Parser, Subcommand};
use raytx::{get_rpc_client, get_wallet, logger, raydium::get_pool_info, swap, token};
use std::str::FromStr;
use tracing::info;

use solana_sdk::{pubkey::Pubkey, signature::Signer};

#[derive(Parser)]
#[command(name = "raytx", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Buy the mint token")]
    Buy {
        mint: String,
        #[arg(help = "wsol amount")]
        in_amount: f64,
    },
    #[command(about = "Sell the mint token")]
    Sell {
        mint: String,
        #[arg(help = "mint amount")]
        in_amount: f64,
    },
    #[command(about = "Sell all mint token and close the account")]
    SellAll { mint: String },
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
        Some(Command::Buy { mint, in_amount }) => {
            info!("buy {} {}", mint, in_amount);
            let swapx = swap::Swap::new(client, wallet.pubkey());
            swapx.swap(mint, *in_amount, 0).await?;
        }
        Some(Command::Sell { mint, in_amount }) => {
            info!("sell {} {}", mint, in_amount);
            let swapx = swap::Swap::new(client, wallet.pubkey());
            swapx.swap(mint, *in_amount, 1).await?;
        }
        Some(Command::SellAll { mint }) => {
            info!("sell_all {}", mint);
            let swapx = swap::Swap::new(client, wallet.pubkey());
            swapx.swap(mint, 0.0, 11).await?;
        }
        Some(Command::Token(token_command)) => match token_command {
            TokenCommand::List => {
                let token_accounts = token::token_accounts(&client, &wallet.pubkey());
                info!("token_accounts: {:#?}", token_accounts);
            }
            TokenCommand::Show { mint } => {
                let mint = Pubkey::from_str(mint).expect("failed to parse mint pubkey");
                let token_account = token::token_account(&client, &wallet.pubkey(), mint)?;
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
