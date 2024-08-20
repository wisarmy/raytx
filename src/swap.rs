use anyhow::Result;
use clap::ValueEnum;
use serde::Deserialize;
use tracing::info;

use crate::{api::AppState, pump, raydium};

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
) -> Result<()> {
    let client = state.client;
    let wallet = state.wallet;
    let client_blocking = state.client_blocking.clone();

    if pump::is_pump_funning(mint).await? {
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
