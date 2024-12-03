use std::{env, str::FromStr};

use amm_cli::load_amm_keys;
use anyhow::{Context, Result};
use common::common_utils;
use futures_util::{SinkExt, StreamExt};
use raytx::{get_rpc_client_blocking, logger, pump::PUMP_PROGRAM, raydium::get_pool_state_by_mint};
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info};
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    logger::init();

    // get_signatures().await?;
    // connect_websocket().await?;
    // get_amm_info().await?;
    get_amm_info_by_mint().await?;
    Ok(())
}

pub async fn get_amm_info_by_mint() -> Result<()> {
    let client = get_rpc_client_blocking()?;
    let mint = "DrEMQaQqGN2fQwiUgJi6NStLtmni8m3uSkUP678Apump";

    let pool_state = get_pool_state_by_mint(client, mint).await?;

    println!("pool_state: {:#?}", pool_state);

    Ok(())
}

pub async fn get_amm_info() -> Result<()> {
    let client = get_rpc_client_blocking()?;
    // let amm_pool_id = Pubkey::from_str("3vehHGc8J9doSo6gJoWYG23JG54hc2i7wjdFReX3Rcah")?;
    let amm_pool_id = Pubkey::from_str("7Sp76Pv48RaL4he2BfGUhvjqCtvjjfTSnXDXNvk845yL")?;

    let pool_state =
        common::rpc::get_account::<raydium_amm::state::AmmInfo>(&client, &amm_pool_id)?.unwrap();

    println!("pool_state : {:#?}", pool_state);
    let amm_program = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?;
    let native_mint = spl_token::native_mint::ID;
    let amount_specified = 100_000_000;
    let slippage_bps = 10;
    let swap_base_in = true;
    let user_input_token = if pool_state.coin_vault_mint == native_mint {
        pool_state.pc_vault
    } else {
        pool_state.coin_vault
    };

    let amm_keys = load_amm_keys(&client, &amm_program, &amm_pool_id).unwrap();
    // reload accounts data to calculate amm pool vault amount
    // get multiple accounts at the same time to ensure data consistency
    let load_pubkeys = vec![
        amm_pool_id,
        amm_keys.amm_pc_vault,
        amm_keys.amm_coin_vault,
        user_input_token,
    ];
    let rsps = common::rpc::get_multiple_accounts(&client, &load_pubkeys).unwrap();

    println!("rsps: {:#?}", rsps);

    let amm_pc_vault_account = rsps[1].clone();
    let amm_coin_vault_account = rsps[2].clone();
    let _token_in_account = rsps[3].clone();

    let amm_pc_vault =
        common_utils::unpack_token(&amm_pc_vault_account.as_ref().unwrap().data).unwrap();
    let amm_coin_vault =
        common_utils::unpack_token(&amm_coin_vault_account.as_ref().unwrap().data).unwrap();

    println!("amm_pc_vault: {:#?}", amm_pc_vault.base.amount);
    println!("amm_coin_vault: {:#?}", amm_coin_vault.base.amount);

    let swap_info_result = amm_cli::calculate_swap_info(
        &client,
        amm_program,
        amm_pool_id,
        user_input_token,
        amount_specified,
        slippage_bps,
        swap_base_in,
    )
    .unwrap();

    println!("swap_info_result : {:#?}", swap_info_result);

    Ok(())
}

pub async fn get_signatures() -> Result<()> {
    let client = get_rpc_client_blocking()?;
    let config = GetConfirmedSignaturesForAddress2Config {
        before: None,
        until: None,
        limit: Some(3),
        commitment: Some(CommitmentConfig::confirmed()),
    };

    let address = Pubkey::from_str(PUMP_PROGRAM)?;
    let signatures = client.get_signatures_for_address_with_config(&address, config)?;

    for signature in signatures {
        info!("{:#?}", signature);
    }
    Ok(())
}

pub async fn connect_websocket() -> Result<()> {
    let (ws_stream, _) = connect_async(env::var("RPC_WEBSOCKET_ENDPOINT")?)
        .await
        .context("Failed to connect to WebSocket server")?;

    info!("Connected to WebSocket server: sol websocket");

    let (mut write, mut read) = ws_stream.split();

    let _program_subscribe = serde_json::json!({
      "jsonrpc": "2.0",
      "id": 1,
      "method": "programSubscribe",
      "params": [
        PUMP_PROGRAM,
        {
          "encoding": "jsonParsed",
          "commitment": "processed"
        }
      ]
    });
    let logs_subscribe = serde_json::json!({
          "jsonrpc": "2.0",
          "id": 1,
          "method": "logsSubscribe",
          "params": [
            {
              "mentions": [ PUMP_PROGRAM ]
            },
            {
              "commitment": "processed"
            }
          ]
    });
    tokio::spawn(async move {
        let msg = Message::text(logs_subscribe.to_string());
        write.send(msg).await.expect("Failed to send message");
    });

    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                let response: serde_json::Value = serde_json::from_str(&text).unwrap();
                info!("Received text message: {:#?}", response);
            }
            Ok(Message::Close(close)) => {
                info!("Connection closed: {:?}", close);
                break;
            }
            Err(e) => {
                error!("Error receiving message: {:?}", e);
                break;
            }
            _ => {
                info!("unkown message");
            }
        }
    }

    Ok(())
}
