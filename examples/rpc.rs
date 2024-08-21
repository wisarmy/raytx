use std::{env, str::FromStr};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use raytx::{get_rpc_client_blocking, logger, pump::PUMP_PROGRAM};
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info};
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    logger::init();

    // get_signatures().await?;
    connect_websocket().await?;
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
