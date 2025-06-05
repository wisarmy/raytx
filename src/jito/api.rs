use std::env;

use anyhow::{Context, Result};
use reqwest::Proxy;
use serde::{Deserialize, Serialize};

use super::{TipPercentileData, BLOCK_ENGINE_URL};

#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u32,
    method: String,
    params: Vec<()>,
}

#[derive(Deserialize, Debug)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: Option<u32>,
    pub result: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}

pub async fn get_tip_accounts() -> Result<RpcResponse> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = env::var("HTTP_PROXY") {
        let proxy = Proxy::all(http_proxy)?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build()?;
    let request_body = RpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "getTipAccounts".to_string(),
        params: vec![],
    };
    let result = client
        .post(format!("{}/api/v1/bundles", *BLOCK_ENGINE_URL))
        .json(&request_body)
        .send()
        .await?
        .json::<RpcResponse>()
        .await?;
    Ok(result)
}
/// tip accounts
#[derive(Debug)]
pub struct TipAccountResult {
    pub accounts: Vec<String>,
}

impl TryFrom<RpcResponse> for TipAccountResult {
    type Error = anyhow::Error;
    fn try_from(value: RpcResponse) -> Result<Self, Self::Error> {
        if let Some(error) = value.error {
            return Err(anyhow::anyhow!("RPC error: {}", error));
        }
        
        let result = value.result.context("missing 'result' field in response")?;
        let accounts = result
            .as_array()
            .context("expected 'result' to be an array")?
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        Ok(TipAccountResult { accounts })
    }
}

pub async fn get_tip_amounts() -> Result<Vec<TipPercentileData>> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = env::var("HTTP_PROXY") {
        let proxy = Proxy::all(http_proxy)?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build()?;

    let result = client
        .get("https://bundles.jito.wtf/api/v1/bundles/tip_floor")
        .send()
        .await?
        .json::<Vec<TipPercentileData>>()
        .await?;
    Ok(result)
}
