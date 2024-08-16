use std::{collections::HashMap, env};

use anyhow::{anyhow, Result};
use reqwest::Proxy;
use serde::Deserialize;

// get pool info
// https://api-v3.raydium.io/pools/info/mint?mint1=So11111111111111111111111111111111111111112&mint2=EzM2d8JVpzfhV7km3tUsR1U1S4xwkrPnWkM4QFeTpump&poolType=standard&poolSortField=default&sortType=desc&pageSize=10&page=1
pub async fn get_pool_info(mint1: &str, mint2: &str) -> Result<PoolInfo> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = env::var("HTTP_PROXY") {
        let proxy = Proxy::all(http_proxy)?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build()?;

    let result = client
        .get("https://api-v3.raydium.io/pools/info/mint")
        .query(&[
            ("mint1", mint1),
            ("mint2", mint2),
            ("poolType", "standard"),
            ("poolSortField", "default"),
            ("sortType", "desc"),
            ("pageSize", "1"),
            ("page", "1"),
        ])
        .send()
        .await?
        .json::<PoolInfo>()
        .await?;
    Ok(result)
}
// get pool info by ids
// https://api-v3.raydium.io/pools/info/ids?ids=3RHg85W1JtKeqFQSxBfd2RX13aBFvvy6gcATkHU657mL
pub async fn get_pool_info_by_id(pool_id: &str) -> Result<PoolData> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = env::var("HTTP_PROXY") {
        let proxy = Proxy::all(http_proxy)?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build()?;

    let result = client
        .get("https://api-v3.raydium.io/pools/info/ids")
        .query(&[("ids", pool_id)])
        .send()
        .await?
        .json::<PoolData>()
        .await?;
    Ok(result)
}

#[derive(Debug, Deserialize)]
struct CurrencyData {
    usd: f64,
}
// get sol price
// https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd
pub async fn get_price(name: &str) -> Result<f64> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = env::var("HTTP_PROXY") {
        let proxy = Proxy::all(http_proxy)?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build()?;

    let result = client
        .get("https://api.coingecko.com/api/v3/simple/price")
        .query(&[("ids", name), ("vs_currencies", "usd")])
        .send()
        .await?
        .json::<HashMap<String, CurrencyData>>()
        .await?;
    Ok(result
        .get(name)
        .ok_or(anyhow!("failed get {} currency data", name))?
        .usd)
}

impl PoolInfo {
    pub fn get_pool_id(&self) -> Option<String> {
        if let Some(pool) = self.data.get_pool() {
            Some(pool.id.clone())
        } else {
            None
        }
    }
    pub fn get_market_id(&self) -> Option<String> {
        if let Some(pool) = self.data.get_pool() {
            Some(pool.market_id.clone())
        } else {
            None
        }
    }
    pub fn get_pool(&self) -> Option<Pool> {
        self.data.get_pool()
    }
}

#[derive(Debug, Deserialize)]
pub struct PoolInfo {
    pub success: bool,
    pub data: PoolData,
}

#[derive(Debug, Deserialize)]
pub struct PoolData {
    // pub count: u32,
    pub data: Vec<Pool>,
}

impl PoolData {
    pub fn get_pool(&self) -> Option<Pool> {
        self.data.first().cloned()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Pool {
    pub id: String,
    #[serde(rename = "programId")]
    pub program_id: String,
    #[serde(rename = "mintA")]
    pub mint_a: Mint,
    #[serde(rename = "mintB")]
    pub mint_b: Mint,
    #[serde(rename = "marketId")]
    pub market_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Mint {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}
