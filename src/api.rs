use std::{env, sync::Arc};

use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use tracing::{info, warn};

use crate::{
    helper::{api_error, api_ok},
    raydium::Raydium,
    swap::{self, SwapDirection, SwapInType},
};

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<RpcClient>,
    pub client_blocking: Arc<solana_client::rpc_client::RpcClient>,
    pub wallet: Arc<Keypair>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSwap {
    mint: String,
    direction: SwapDirection,
    amount_in: f64,
    in_type: Option<SwapInType>,
    slippage: Option<u64>,
    jito: Option<bool>,
}

#[debug_handler]
pub async fn swap(
    State(state): State<AppState>,
    Json(input): Json<CreateSwap>,
) -> impl IntoResponse {
    let slippage = match input.slippage {
        Some(v) => v,
        None => {
            let slippage = env::var("SLIPPAGE").unwrap_or("5".to_string());
            let slippage = slippage.parse::<u64>().unwrap_or(5);
            slippage
        }
    };

    info!("{:?}, slippage: {}", input, slippage);

    let result = swap::swap(
        state,
        input.mint.as_str(),
        input.amount_in,
        input.direction.clone(),
        input.in_type.unwrap_or(SwapInType::Qty),
        slippage,
        input.jito.unwrap_or(false),
    )
    .await;
    match result {
        Ok(_) => api_ok(()),
        Err(err) => {
            warn!("swap err: {:#?}", err);
            api_error(&err.to_string())
        }
    }
}

#[debug_handler]
pub async fn get_pool(
    State(state): State<AppState>,
    Path(pool_id): Path<String>,
) -> impl IntoResponse {
    let client = state.client;
    let wallet = state.wallet;
    let mut swapx = Raydium::new(client, wallet);
    swapx.with_blocking_client(state.client_blocking);
    match swapx.get_pool(pool_id.as_str()).await {
        Ok(data) => api_ok(json!({
            "base": data.0,
            "quote": data.1,
            "price": data.2,
            "sol_price": data.3
        })),
        Err(err) => {
            warn!("get pool err: {:#?}", err);
            api_error(&err.to_string())
        }
    }
}
