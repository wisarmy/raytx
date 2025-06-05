use anyhow::Result;
use clap::ValueEnum;
use serde::Deserialize;
use tracing::info;

use crate::{
    api::AppState,
    get_rpc_client,
    pump::{self, get_pump_info},
    raydium,
};

#[derive(ValueEnum, Debug, Clone, Deserialize)]
pub enum SwapDirection {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
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
#[derive(ValueEnum, Debug, Clone, Deserialize)]
pub enum SwapInType {
    /// Quantity
    #[serde(rename = "qty")]
    Qty,
    /// Percentage
    #[serde(rename = "pct")]
    Pct,
}

pub async fn swap(
    state: AppState,
    mint: &str,
    amount_in: f64,
    swap_direction: SwapDirection,
    in_type: SwapInType,
    slippage: u64,
    use_jito: bool,
) -> Result<Vec<String>> {
    let client = get_rpc_client()?;
    let wallet = state.wallet;

    let pump_info_result = get_pump_info(client.clone(), mint).await;

    println!("pump_info_result: {:#?}", pump_info_result);

    match pump_info_result {
        Ok(pump_info) => {
            if !pump_info.complete {
                // Pump token not completed, use original pump trading
                info!("swap in pump fun");
                let swapx = pump::Pump::new(client, wallet);
                swapx
                    .swap(mint, amount_in, swap_direction, in_type, slippage, use_jito)
                    .await
            } else {
                // Pump token completed, use pump amm trading
                // info!("swap in pump amm");
                Err(anyhow::anyhow!(
                    "Pump token {} is completed, not support swap in pump amm yet",
                    mint
                ))
            }
        }
        Err(_err) => {
            // Not a pump token or failed to get pump info, use raydium
            info!("swap in raydium");
            let swapx = raydium::Raydium::new(client, wallet);
            swapx
                .swap(mint, amount_in, swap_direction, in_type, slippage, use_jito)
                .await
        }
    }
}
