use std::{env, sync::Arc};

use anyhow::{anyhow, Result};
use reqwest::Proxy;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;

pub mod api;
pub mod constants;
pub mod helper;
pub mod jito;
pub mod logger;
pub mod parser;
pub mod pool;
pub mod pump;
pub mod raydium;
pub mod swap;
#[cfg(feature = "swap_ts")]
pub mod swap_ts;
pub mod token;
pub mod tx;

fn get_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("Environment variable {} is not set", key))
}

pub fn get_client_build() -> Result<reqwest::Client> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = env::var("HTTP_PROXY") {
        let proxy = Proxy::all(http_proxy)?;
        client_builder = client_builder.proxy(proxy);
    }
    match client_builder.build() {
        Ok(client) => Ok(client),
        Err(err) => Err(anyhow!("failed create client: {}", err)),
    }
}

pub fn get_rpc_client() -> Result<Arc<RpcClient>> {
    let cluster_url = env::var("RPC_ENDPOINT")?;
    let client = RpcClient::new(cluster_url.to_string());
    return Ok(Arc::new(client));
}

pub fn get_rpc_client_blocking() -> Result<Arc<solana_client::rpc_client::RpcClient>> {
    let cluster_url = env::var("RPC_ENDPOINT")?;
    let client = solana_client::rpc_client::RpcClient::new(cluster_url.to_string());
    return Ok(Arc::new(client));
}

pub fn get_wallet() -> Result<Arc<Keypair>> {
    let wallet = Keypair::from_base58_string(&env::var("PRIVATE_KEY")?);
    return Ok(Arc::new(wallet));
}

#[cfg(test)]
mod tests {
    #[ctor::ctor]
    fn init() {
        crate::logger::init();
        dotenvy::dotenv().ok();
    }
}
