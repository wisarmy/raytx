use anyhow::Result;
use reqwest::Proxy;
use serde::Deserialize;

// 获取池子信息
// https://api-v3.raydium.io/pools/info/mint?mint1=So11111111111111111111111111111111111111112&mint2=EzM2d8JVpzfhV7km3tUsR1U1S4xwkrPnWkM4QFeTpump&poolType=standard&poolSortField=default&sortType=desc&pageSize=10&page=1
pub async fn get_pool_info(mint1: &str, mint2: &str) -> Result<PoolInfo> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = dotenvy::var("HTTP_PROXY") {
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

impl PoolInfo {
    pub fn get_pool_id(&self) -> Option<String> {
        if let Some(pool) = self.data.data.first() {
            Some(pool.id.clone())
        } else {
            None
        }
    }
    pub fn get_market_id(&self) -> Option<String> {
        if let Some(pool) = self.data.data.first() {
            Some(pool.market_id.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PoolInfo {
    pub success: bool,
    pub data: PoolData,
}

#[derive(Debug, Deserialize)]
pub struct PoolData {
    pub count: u32,
    pub data: Vec<Pool>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Mint {
    pub address: String,
    pub symbol: String,
    pub name: String,
}
