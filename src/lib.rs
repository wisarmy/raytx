use std::sync::Arc;

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;

pub mod logger;
pub mod raydium;
pub mod swap;
pub mod swap_rs;
pub mod token;

pub fn get_rpc_client() -> Result<Arc<RpcClient>> {
    let cluster_url = dotenvy::var("RPC_ENDPOINT")?;
    let client = RpcClient::new(cluster_url.to_string());
    return Ok(Arc::new(client));
}

pub fn get_wallet() -> Result<Keypair> {
    let wallet = Keypair::from_base58_string(&dotenvy::var("PRIVATE_KEY")?);
    return Ok(wallet);
}

#[cfg(test)]
mod tests {
    #[ctor::ctor]
    fn init() {
        crate::logger::init();
    }
}
