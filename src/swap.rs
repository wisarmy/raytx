use anyhow::Result;
use clap::ValueEnum;
use serde::Deserialize;
use tracing::{info, warn};

use crate::{
    api::AppState,
    get_rpc_client, get_rpc_client_blocking,
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
    let client_blocking = get_rpc_client_blocking()?;
    let wallet = state.wallet;

    let swap_in_pump = get_pump_info(client_blocking.clone(), mint)
        .await
        .map_or_else(
            |err| {
                warn!("failed to get_pump_info: {}", err);
                false
            },
            |pump_info| !pump_info.complete,
        );

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
        swapx
            .swap(mint, amount_in, swap_direction, in_type, slippage, use_jito)
            .await
    }
}
