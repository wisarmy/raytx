use anyhow::Result;
use clap::ValueEnum;
use serde::Deserialize;
use tracing::{info, warn};

use crate::{
    api::AppState,
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
    let client = state.client;
    let wallet = state.wallet;
    let client_blocking = state.client_blocking.clone();
    let mut swap_in_pump = true;

    let raydium_pool = match get_pump_info(mint).await {
        Ok(pump_info) => {
            if let Some(pool) = pump_info.raydium_pool.as_str() {
                swap_in_pump = false;
                Some(pool.to_string())
            } else {
                None
            }
        }
        Err(err) => {
            warn!("failed to get_pump_info: {}", err);
            swap_in_pump = false;
            None
        }
    };

    if swap_in_pump {
        info!("swap in pump fun");
        let mut swapx = pump::Pump::new(client, wallet);
        swapx.with_blocking_client(client_blocking);
        swapx
            .swap(mint, amount_in, swap_direction, in_type, slippage, use_jito)
            .await
    } else {
        info!("swap in raydium");
        let mut swapx = raydium::Raydium::new(client, wallet);
        swapx.with_blocking_client(client_blocking);
        swapx.with_pool_id(raydium_pool);
        swapx
            .swap(mint, amount_in, swap_direction, in_type, slippage, use_jito)
            .await
    }
}
