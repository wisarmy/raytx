use std::{
    process::{Command, Output},
    str::FromStr,
};

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tracing::{error, info};

use crate::{raydium::get_pool_info, token};

pub struct Swap {
    client: RpcClient,
    pubkey: Pubkey,
}
impl Swap {
    pub fn new(client: RpcClient, pubkey: Pubkey) -> Self {
        Self { client, pubkey }
    }
    /// direction:
    /// * 0 buy
    /// * 1 sell
    /// * 11 sell all and close account
    pub async fn swap(&self, mint: &str, in_amount: f64, direction: u8) -> Result<bool> {
        let mint =
            Pubkey::from_str(mint).map_err(|e| anyhow!("failed to parse mint pubkey: {}", e))?;
        let mut ui_amount = in_amount;
        if direction == 11 {
            let token_account = token::token_account(&self.client, &self.pubkey, mint)?;
            info!("token_account: {:#?}", token_account);
            ui_amount = token_account.ui_amount;
        } else if direction == 1 {
            ui_amount = std::cmp::min_by(in_amount, ui_amount, |a, b| {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less)
            });
        }

        let pool_info =
            get_pool_info(&spl_token::native_mint::id().to_string(), &mint.to_string()).await?;
        let pool_id = pool_info.get_pool_id().unwrap();
        match direction {
            0 => {
                info!(
                    "buy command: npx ts-node raydium/swap_cli.ts {} {} {}",
                    pool_id, ui_amount, 0
                );
            }
            1 => {
                info!(
                    "sell command: npx ts-node raydium/swap_cli.ts {} {} {}",
                    pool_id, ui_amount, 1
                );
            }
            11 => {
                info!(
                    "sell command: npx ts-node raydium/swap_cli.ts {} {} {}",
                    pool_id, ui_amount, 11
                );
            }
            _ => {
                error!("direction not supported: {}", direction);
                return Err(anyhow!("direction not supported: {}", direction));
            }
        }
        let status = self.swap_cli(&mint.to_string(), ui_amount, direction)?;
        Ok(status)
    }

    fn swap_cli(&self, mint: &str, ui_amount: f64, direction: u8) -> Result<bool> {
        let output: Output = Command::new("npx")
            .arg("ts-node")
            .arg("raydium/swap_cli.ts")
            .arg(mint)
            .arg(ui_amount.to_string())
            .arg(direction.to_string())
            .output()?;

        if output.status.success() {
            info!("Output: {}", String::from_utf8_lossy(&output.stdout));
            Ok(true)
        } else {
            error!("Error: {}", String::from_utf8_lossy(&output.stderr));
            Ok(false)
        }
    }
}
