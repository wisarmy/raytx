use std::{
    process::{Command, Output},
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use clap::ValueEnum;
use rust_decimal::prelude::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tracing::{error, info};

use crate::{raydium::get_pool_info, token};

pub struct Swap {
    client: RpcClient,
    pubkey: Pubkey,
}
#[derive(ValueEnum, Debug, Clone)]
pub enum SwapDirection {
    Buy,
    Sell,
}
impl From<SwapDirection> for u8 {
    fn from(value: SwapDirection) -> Self {
        match value {
            SwapDirection::Buy => 0,
            SwapDirection::Sell => 1,
        }
    }
}
#[derive(ValueEnum, Debug, Clone)]
pub enum SwapInType {
    /// Quantity
    Qty,
    /// Percentage
    Pct,
}
impl Swap {
    pub fn new(client: RpcClient, pubkey: Pubkey) -> Self {
        Self { client, pubkey }
    }
    /// direction:
    /// * 0 buy
    /// * 1 sell
    /// * 11 sell all and close account
    pub async fn swap(
        &self,
        mint: &str,
        in_amount: f64,
        swap_direction: SwapDirection,
        in_type: SwapInType,
    ) -> Result<bool> {
        let mint =
            Pubkey::from_str(mint).map_err(|e| anyhow!("failed to parse mint pubkey: {}", e))?;
        let mut ui_amount = in_amount;
        let mut direction: u8 = swap_direction.clone().into();
        match swap_direction {
            SwapDirection::Sell => {
                let token_account = token::token_account(&self.client, &self.pubkey, mint)?;
                info!("token_account: {:#?}", token_account);
                match in_type {
                    SwapInType::Qty => {
                        ui_amount = std::cmp::min_by(in_amount, token_account.ui_amount, |a, b| {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less)
                        });
                    }
                    SwapInType::Pct => {
                        let in_amount = in_amount.min(1.0);
                        if in_amount == 1.0 {
                            // sell all, set diretion 11 to close ata
                            ui_amount = token_account.ui_amount;
                            direction = 11;
                        } else {
                            let ui_amount_dec = Decimal::from_f64(token_account.ui_amount)
                                .with_context(|| {
                                    format!(
                                        "failed to convert token_account.ui_amount ({}) to Decimal",
                                        token_account.ui_amount
                                    )
                                })?;
                            let in_amount_dec =
                                Decimal::from_f64(in_amount).with_context(|| {
                                    format!("failed to convert in_amount({}) to Decimal", in_amount)
                                })?;
                            ui_amount = (ui_amount_dec * in_amount_dec)
                                .to_f64()
                                .with_context(|| format!("failed to convert Decimal to f64"))?;
                        }
                    }
                }
            }
            SwapDirection::Buy => {}
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
        let status = self.swap_cli(&pool_id, ui_amount, direction)?;
        Ok(status)
    }

    fn swap_cli(&self, pool_id: &str, ui_amount: f64, direction: u8) -> Result<bool> {
        let output: Output = Command::new("npx")
            .arg("ts-node")
            .arg("raydium/swap_cli.ts")
            .arg(pool_id)
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
