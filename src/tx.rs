use std::{env, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use jito_json_rpc_client::jsonrpc_client::rpc_client::RpcClient as JitoRpcClient;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
    system_transaction,
    transaction::{Transaction, VersionedTransaction},
};
use spl_token::ui_amount_to_amount;

use tokio::time::Instant;
use tracing::{error, info};

use crate::jito::{self, get_tip_account, get_tip_value, wait_for_bundle_confirmation};
// prioritization fee = UNIT_PRICE * UNIT_BUDGET
pub const UNIT_PRICE: u64 = 1;
pub const UNIT_BUDGET: u32 = 300_000;

pub async fn new_signed_and_send(
    client: &RpcClient,
    keypair: &Keypair,
    mut instructions: Vec<Instruction>,
    use_jito: bool,
) -> Result<()> {
    // If not using Jito, manually set the compute unit price and limit
    if !use_jito {
        let modify_compute_units =
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(
                UNIT_PRICE,
            );
        let add_priority_fee =
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                UNIT_BUDGET,
            );
        instructions.insert(0, modify_compute_units);
        instructions.insert(1, add_priority_fee);
    }
    // send init tx
    let recent_blockhash = client.get_latest_blockhash()?;
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &vec![&*keypair],
        recent_blockhash,
    );

    if env::var("TX_SIMULATE").ok() == Some("true".to_string()) {
        let simulate_result = client.simulate_transaction(&txn)?;
        if let Some(logs) = simulate_result.value.logs {
            for log in logs {
                info!("{}", log);
            }
        }
        return match simulate_result.value.err {
            Some(err) => Err(anyhow!("{}", err)),
            None => Ok(()),
        };
    }

    let start_time = Instant::now();
    if use_jito {
        // jito
        let tip_account = get_tip_account().await?;
        let jito_client = Arc::new(JitoRpcClient::new(format!(
            "{}/api/v1/bundles",
            jito::BLOCK_ENGINE_URL.to_string()
        )));
        // jito tip, the upper limit is 0.1
        let mut tip = get_tip_value().await?;
        tip = tip.min(0.1);
        let tip_lamports = ui_amount_to_amount(tip, spl_token::native_mint::DECIMALS);
        info!(
            "tip account: {}, tip(sol): {}, lamports: {}",
            tip_account, tip, tip_lamports
        );
        // tip tx
        let mut bundle: Vec<VersionedTransaction> = vec![];
        bundle.push(VersionedTransaction::from(txn));
        bundle.push(VersionedTransaction::from(system_transaction::transfer(
            &keypair,
            &tip_account,
            tip_lamports,
            recent_blockhash,
        )));
        let bundle_id = jito_client.send_bundle(&bundle).await?;
        info!("bundle_id: {}", bundle_id);

        wait_for_bundle_confirmation(
            move |id: String| {
                let client = Arc::clone(&jito_client);
                async move {
                    let response = client.get_bundle_statuses(&[id]).await;
                    let statuses = response.inspect_err(|err| {
                        error!("Error fetching bundle status: {:?}", err);
                    })?;
                    Ok(statuses.value)
                }
            },
            bundle_id,
            Duration::from_millis(1000),
            Duration::from_secs(30),
        )
        .await?;
    } else {
        let sig = raydium_library::common::rpc::send_txn(&client, &txn, true)?;
        info!("signature: {:?}", sig);
    }

    info!("tx elapsed: {:?}", start_time.elapsed());
    Ok(())
}
