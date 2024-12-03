use std::{env, str::FromStr, sync::Arc};

use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use tracing::{info, warn};

use crate::{
    get_rpc_client, get_rpc_client_blocking,
    helper::{api_error, api_ok},
    pump::{get_pump_info, RaydiumInfo},
    raydium::Raydium,
    swap::{self, SwapDirection, SwapInType},
    token,
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
        Ok(txs) => api_ok(txs),
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
    let client = match get_rpc_client() {
        Ok(client) => client,
        Err(err) => {
            return api_error(&format!("failed to get rpc client: {err}"));
        }
    };
    let client_blocking = match get_rpc_client_blocking() {
        Ok(client) => client,
        Err(err) => {
            return api_error(&format!("failed to get rpc client: {err}"));
        }
    };
    let wallet = state.wallet;
    let mut swapx = Raydium::new(client, wallet);
    swapx.with_blocking_client(client_blocking);
    match swapx.get_pool(pool_id.as_str()).await {
        Ok(data) => api_ok(json!({
            "base": data.0,
            "quote": data.1,
            "price": data.2,
            "usd_price": data.3,
            "sol_price": data.4,
        })),
        Err(err) => {
            warn!("get pool err: {:#?}", err);
            api_error(&err.to_string())
        }
    }
}

pub async fn coins(State(state): State<AppState>, Path(mint): Path<String>) -> impl IntoResponse {
    let client = match get_rpc_client() {
        Ok(client) => client,
        Err(err) => {
            return api_error(&format!("failed to get rpc client: {err}"));
        }
    };
    let client_blocking = match get_rpc_client_blocking() {
        Ok(client) => client,
        Err(err) => {
            return api_error(&format!("failed to get rpc client: {err}"));
        }
    };
    let wallet = state.wallet;
    // query from pump.fun
    let mut pump_info = match get_pump_info(client_blocking.clone(), &mint).await {
        Ok(info) => info,
        Err(err) => {
            return api_error(&err.to_string());
        }
    };
    if pump_info.complete {
        let mut swapx = Raydium::new(client, wallet);
        swapx.with_blocking_client(client_blocking);
        match swapx.get_pool_price(None, Some(mint.as_str())).await {
            Ok(data) => {
                pump_info.raydium_info = Some(RaydiumInfo {
                    base: data.0,
                    quote: data.1,
                    price: data.2,
                });
            }
            Err(err) => {
                warn!("get raydium pool price err: {:#?}", err);
            }
        }
    }

    return api_ok(pump_info);
}

#[debug_handler]
pub async fn token_accounts(State(state): State<AppState>) -> impl IntoResponse {
    let client = match get_rpc_client() {
        Ok(client) => client,
        Err(err) => {
            return api_error(&format!("failed to get rpc client: {err}"));
        }
    };
    let wallet = state.wallet;

    let token_accounts = token::token_accounts(&client, &wallet.pubkey()).await;

    match token_accounts {
        Ok(token_accounts) => api_ok(token_accounts),
        Err(err) => {
            warn!("get token_accounts err: {:#?}", err);
            api_error(&err.to_string())
        }
    }
}

#[debug_handler]
pub async fn token_account(
    State(state): State<AppState>,
    Path(mint): Path<String>,
) -> impl IntoResponse {
    let client = match get_rpc_client() {
        Ok(client) => client,
        Err(err) => {
            return api_error(&format!("failed to get rpc client: {err}"));
        }
    };
    let wallet = state.wallet;

    let mint = if let Ok(mint) = Pubkey::from_str(mint.as_str()) {
        mint
    } else {
        return api_error("invalid mint pubkey");
    };

    let token_account = token::token_account(&client, &wallet.pubkey(), mint).await;

    match token_account {
        Ok(token_account) => api_ok(token_account),
        Err(err) => {
            warn!("get token_account err: {:#?}", err);
            api_error(&err.to_string())
        }
    }
}
