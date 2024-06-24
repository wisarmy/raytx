use anyhow::Result;
use clap::{Parser, Subcommand};
use raytx::{get_rpc_client, get_wallet, logger, raydium::get_pool_info, swap, token};
use std::str::FromStr;
use tracing::info;

use solana_sdk::{pubkey::Pubkey, signature::Signer};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Swap {
        mint: String,
        in_amount: f64,
        #[arg(value_parser = clap::value_parser!(u8).range(0..=11))]
        direction: u8,
    },
    Wrap {},
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
            in_amount,
            direction,
        }) => {
            info!("{} {} {}", mint, in_amount, direction);
            let swapx = swap::Swap::new(client, wallet.pubkey());
            swapx.swap(mint, *in_amount, *direction).await?;
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
