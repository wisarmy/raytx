use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::get_client_build;

pub fn api_ok<T: Serialize>(data: T) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "data": data
    }))
}
pub fn api_error(msg: &str) -> Json<Value> {
    Json(json!({
        "status": "error",
        "message": msg
    }))
}

#[derive(Debug, Deserialize)]
struct CurrencyData {
    usd: f64,
}
// get sol price from coingecko
// https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd
pub async fn get_price(name: &str) -> Result<f64> {
    let client = get_client_build()?;

    let result = client
        .get("https://api.coingecko.com/api/v3/simple/price")
        .query(&[("ids", name), ("vs_currencies", "usd")])
        .send()
        .await?
        .json::<HashMap<String, CurrencyData>>()
        .await
        .context("Failed to parse price JSON")?;
    Ok(result
        .get(name)
        .ok_or(anyhow!("failed get {} currency data", name))?
        .usd)
}
// get sol price from pump.fun
// https://frontend-api.pump.fun/sol-price
pub async fn get_solana_price() -> Result<f64> {
    let client = get_client_build()?;

    let result = client
        .get("https://frontend-api.pump.fun/sol-price")
        .send()
        .await?
        .json::<HashMap<String, f64>>()
        .await
        .context("Failed to parse price JSON")?;
    let sol_price = result
        .get("solPrice")
        .ok_or(anyhow!("failed get sol price"))?;
    Ok(*sol_price)
}

#[cfg(test)]
mod tests {
    use tracing::debug;

    use super::*;

    #[tokio::test]
    async fn test_get_solana_price() {
        let price = get_solana_price().await.unwrap();
        debug!("sol price: {}", price);
        assert!(price > 0.0)
    }
}
