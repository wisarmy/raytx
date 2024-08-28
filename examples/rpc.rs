use std::{env, str::FromStr, sync::Arc, time::Duration};

use anyhow::{Context, Result};

use futures_util::{SinkExt, StreamExt};
use raytx::{get_rpc_client, get_rpc_client_blocking, logger, parser, pump::PUMP_PROGRAM};
use solana_client::{
    rpc_client::GetConfirmedSignaturesForAddress2Config, rpc_config::RpcTransactionConfig,
    rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use tokio::{sync::Mutex, time};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    logger::init();

    let signature = Signature::from_str(
        // "3hmLoihrntcQNDyuGY7NFZ4aCaGzsASehGD5UfGUhJVnWbFZfnnoWjZ4Dh1UATQXRsKokJEaAeSnzhszHnMo1PwN",
        // "2nqBdLFcmHBtSZyWaBiimsB4qDwoohWpiJdrQzaV7B2zUwmeJWnsEZ18uMzw8WB3UwXzWtCXXUibUioJAFTHmuok",
        "2aBMaQvN8StxvedeExc8sGWfBUTC1co9ZnUXE7UE5UDboFWbrxrXvV7bXeWRSY6hhpU1RuNSVTHVFmoE6sfdcDDd",
    )?;
    let tx = get_transaction(&signature).await?;
    info!("{:#?}", tx);
    parser::transaction::parse(tx).await;
    // connect_websocket().await?;
    Ok(())
}

pub async fn get_transaction(
    signature: &Signature,
) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
    let client = get_rpc_client()?;
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };
    let tx = client
        .get_transaction_with_config(&signature, config)
        .await?;
    Ok(tx)
}

pub async fn get_signatures() -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
    let client = get_rpc_client_blocking()?;
    let config = GetConfirmedSignaturesForAddress2Config {
        before: None,
        until: None,
        limit: Some(3),
        commitment: Some(CommitmentConfig::confirmed()),
    };

    // let address = Pubkey::from_str(PUMP_PROGRAM)?;
    let address = Pubkey::from_str("4Be9CvxqHW6BYiRAxW9Q3xu1ycTMWaL5z8NX4HR3ha7t")?;
    let signatures = client.get_signatures_for_address_with_config(&address, config)?;

    for signature in signatures.clone() {
        info!("{:#?}", signature);
    }
    Ok(signatures)
}

pub async fn connect_websocket() -> Result<()> {
    let (ws_stream, _) = connect_async(env::var("RPC_WEBSOCKET_ENDPOINT")?)
        .await
        .context("Failed to connect to WebSocket server")?;

    info!("Connected to WebSocket server: sol websocket");

    let (write, mut read) = ws_stream.split();

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
              // "mentions": [ PUMP_PROGRAM ]
              // "mentions": [ "4Be9CvxqHW6BYiRAxW9Q3xu1ycTMWaL5z8NX4HR3ha7t" ]
              "mentions": [ "4DdrfiDHpmx55i4SPssxVzS9ZaKLb8qr45NKY9Er9nNh" ]
            },
            {
              // "commitment": "processed"
              "commitment": "confirmed"
            }
          ]
    });
    let write = Arc::new(Mutex::new(write));
    let write_subscribe = Arc::clone(&write);
    tokio::spawn(async move {
        let msg = Message::text(logs_subscribe.to_string());
        let mut write = write_subscribe.lock().await;
        write.send(msg).await.expect("Failed to send message");
    });
    let write_ping = Arc::clone(&write);

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let mut write = write_ping.lock().await;
            if let Err(e) = write.send(Message::Ping(vec![])).await {
                error!("Failed to send Ping: {:?}", e);
                break;
            }
        }
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
            Ok(Message::Ping(ping)) => {
                info!("Received Ping, sending Pong: {:?}", ping);
                let mut write = write.lock().await;
                write
                    .send(Message::Pong(ping))
                    .await
                    .expect("Failed to send Pong");
            }
            Ok(Message::Pong(pong)) => {
                info!("Received Pong: {:?}", pong);
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
