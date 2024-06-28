use std::{
    process::{Command, Output},
    str::FromStr,
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use rust_decimal::prelude::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tracing::{error, info};

use crate::{
    raydium::get_pool_info,
    swap::{SwapDirection, SwapInType},
    token,
};

pub struct Swap {
    client: Arc<RpcClient>,
    pubkey: Pubkey,
    swap_addr: Option<String>,
}

impl Swap {
    pub fn new(client: Arc<RpcClient>, pubkey: Pubkey, swap_addr: Option<String>) -> Self {
        Self {
            client,
            pubkey,
            swap_addr,
        }
    }
    /// direction:
    /// * 0 buy
    /// * 1 sell
    /// * 11 sell all and close account
    pub async fn swap(
        &self,
        mint: &str,
        amount_in: f64,
        swap_direction: SwapDirection,
        in_type: SwapInType,
    ) -> Result<bool> {
        let mint =
            Pubkey::from_str(mint).map_err(|e| anyhow!("failed to parse mint pubkey: {}", e))?;
        let mut ui_amount = amount_in;
        let mut direction: u8 = swap_direction.clone().into();
        match swap_direction {
            SwapDirection::Sell => {
                let token_account = token::token_account(&self.client, &self.pubkey, mint).await?;
                info!("token_account: {:#?}", token_account);
                match in_type {
                    SwapInType::Qty => {
                        ui_amount = std::cmp::min_by(amount_in, token_account.ui_amount, |a, b| {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less)
                        });
                    }
                    SwapInType::Pct => {
                        let amount_in = amount_in.min(1.0);
                        if amount_in == 1.0 {
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
                            let amount_in_dec =
                                Decimal::from_f64(amount_in).with_context(|| {
                                    format!("failed to convert amount_in({}) to Decimal", amount_in)
                                })?;
                            ui_amount = (ui_amount_dec * amount_in_dec)
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
        let pool_id = pool_info.get_pool_id().expect("pool not found");
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
        if self.swap_addr.is_some() {
            let status = self.swap_api(&pool_id, ui_amount, direction).await?;
            Ok(status)
        } else {
            let status = self.swap_cli(&pool_id, ui_amount, direction)?;
            Ok(status)
        }
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

    async fn swap_api(&self, pool_id: &str, ui_amount: f64, direction: u8) -> Result<bool> {
        let client = reqwest::Client::new();

        let response = client
            .post(
                self.swap_addr
                    .clone()
                    .expect("not found raydium_server_addr"),
            )
            .json(&serde_json::json!({
                "poolId": pool_id,
                "amountIn": ui_amount,
                "dir": direction,
            }))
            .send()
            .await?;

        let status = response.status().is_success();
        let content = response.text().await?;
        info!("swap response: {content:?}");
        Ok(status)
    }
}
