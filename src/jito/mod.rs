use std::{future::Future, str::FromStr, sync::LazyLock, time::Duration};

use anyhow::{anyhow, Result};
use api::{get_tip_accounts, TipAccountResult};
use indicatif::{ProgressBar, ProgressStyle};
use rand::{seq::IteratorRandom, thread_rng};
use serde::Deserialize;
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use tokio::{
    sync::RwLock,
    time::{sleep, Instant},
};
use tracing::{debug, error, info, warn};

use crate::get_env_var;

pub mod api;
pub mod ws;

pub static TIPS_PERCENTILE: LazyLock<RwLock<Option<TipPercentileData>>> =
    LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Deserialize, Clone)]
pub struct TipPercentileData {
    pub time: String,
    pub landed_tips_25th_percentile: f64,
    pub landed_tips_50th_percentile: f64,
    pub landed_tips_75th_percentile: f64,
    pub landed_tips_95th_percentile: f64,
    pub landed_tips_99th_percentile: f64,
    pub ema_landed_tips_50th_percentile: f64,
}

pub static BLOCK_ENGINE_URL: LazyLock<String> =
    LazyLock::new(|| get_env_var("JITO_BLOCK_ENGINE_URL"));
pub static TIP_STREAM_URL: LazyLock<String> = LazyLock::new(|| get_env_var("JITO_TIP_STREAM_URL"));
pub static TIP_PERCENTILE: LazyLock<String> = LazyLock::new(|| get_env_var("JITO_TIP_PERCENTILE"));

pub static TIP_ACCOUNTS: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| RwLock::new(vec![]));

pub async fn init_tip_accounts() -> Result<()> {
    let accounts: TipAccountResult = get_tip_accounts().await?.try_into()?;
    let mut tip_accounts = TIP_ACCOUNTS.write().await;

    accounts
        .accounts
        .iter()
        .for_each(|account| tip_accounts.push(account.to_string()));
    Ok(())
}

pub async fn get_tip_account() -> Result<Pubkey> {
    let accounts = TIP_ACCOUNTS.read().await;
    let mut rng = thread_rng();
    match accounts.iter().choose(&mut rng) {
        Some(acc) => Ok(Pubkey::from_str(acc).inspect_err(|err| {
            error!("jito: failed to parse Pubkey: {:?}", err);
        })?),
        None => Err(anyhow!("jito: no tip accounts available")),
    }
}

pub async fn init_tip_amounts() -> Result<()> {
    let tip_percentiles = api::get_tip_amounts().await?;
    *TIPS_PERCENTILE.write().await = tip_percentiles.first().cloned();

    Ok(())
}

// unit sol
pub async fn get_tip_value() -> Result<f64> {
    // If TIP_VALUE is set, use it
    if let Ok(tip_value) = std::env::var("JITO_TIP_VALUE") {
        if let Ok(value) = f64::from_str(&tip_value) {
            return Ok(value);
        } else {
            warn!(
                "Invalid TIP_VALUE in environment variable, falling back to percentile calculation"
            );
        }
    }

    let tips = TIPS_PERCENTILE.read().await;

    if let Some(ref data) = *tips {
        match TIP_PERCENTILE.as_str() {
            "25" => Ok(data.landed_tips_25th_percentile),
            "50" => Ok(data.landed_tips_50th_percentile),
            "75" => Ok(data.landed_tips_75th_percentile),
            "95" => Ok(data.landed_tips_95th_percentile),
            "99" => Ok(data.landed_tips_99th_percentile),
            _ => Err(anyhow!("jito: invalid TIP_PERCENTILE value")),
        }
    } else {
        Err(anyhow!("jito: failed get tip"))
    }
}

#[derive(Deserialize, Debug)]
pub struct BundleStatus {
    pub bundle_id: String,
    pub transactions: Vec<String>,
    pub slot: u64,
    pub confirmation_status: String,
    pub err: ErrorStatus,
}
#[derive(Deserialize, Debug)]
pub struct ErrorStatus {
    #[serde(rename = "Ok")]
    pub ok: Option<()>,
}

pub async fn wait_for_bundle_confirmation<F, Fut>(
    fetch_statuses: F,
    bundle_id: String,
    interval: Duration,
    timeout: Duration,
) -> Result<Vec<String>>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<Vec<Value>>>,
{
    let progress_bar = new_progress_bar();
    let start_time = Instant::now();

    loop {
        let statuses = fetch_statuses(bundle_id.clone()).await?;

        if let Some(status) = statuses.first() {
            let bundle_status: BundleStatus =
                serde_json::from_value(status.clone()).inspect_err(|err| {
                    error!(
                        "Failed to parse JSON when get_bundle_statuses, err: {}",
                        err,
                    );
                })?;

            debug!("{:?}", bundle_status);
            match bundle_status.confirmation_status.as_str() {
                "finalized" | "confirmed" => {
                    progress_bar.finish_and_clear();
                    info!(
                        "Finalized bundle {}: {}",
                        bundle_id, bundle_status.confirmation_status
                    );
                    // print tx
                    bundle_status
                        .transactions
                        .iter()
                        .for_each(|tx| info!("https://solscan.io/tx/{}", tx));
                    return Ok(bundle_status.transactions);
                }
                _ => {
                    progress_bar.set_message(format!(
                        "Finalizing bundle {}: {}",
                        bundle_id, bundle_status.confirmation_status
                    ));
                }
            }
        } else {
            progress_bar.set_message(format!("Finalizing bundle {}: {}", bundle_id, "None"));
        }

        // check loop exceeded 1 minute,
        if start_time.elapsed() > timeout {
            warn!("Loop exceeded {:?}, breaking out.", timeout);
            return Err(anyhow!("Bundle status get timeout"));
        }

        // Wait for a certain duration before retrying
        sleep(interval).await;
    }
}
pub fn new_progress_bar() -> ProgressBar {
    let progress_bar = ProgressBar::new(42);
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .expect("ProgressStyle::template direct input to be correct"),
    );
    progress_bar.enable_steady_tick(Duration::from_millis(100));
    progress_bar
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::{json, Value};

    use super::wait_for_bundle_confirmation;

    fn generate_statuses(bundle_id: String, confirmation_status: &str) -> Vec<Value> {
        vec![json!({
            "bundle_id": bundle_id,
            "transactions": ["tx1", "tx2"],
            "slot": 12345,
            "confirmation_status": confirmation_status,
            "err": {"Ok": null}
        })]
    }

    #[tokio::test]
    async fn test_success_confirmation() {
        for &status in &["finalized", "confirmed"] {
            let wait_result = wait_for_bundle_confirmation(
                |id| async { Ok(generate_statuses(id, status)) },
                "6e4b90284778a40633b56e4289202ea79e62d2296bb3d45398bb93f6c9ec083d".to_string(),
                Duration::from_secs(1),
                Duration::from_secs(1),
            )
            .await;
            assert!(wait_result.is_ok());
        }
    }
    #[tokio::test]
    async fn test_error_confirmation() {
        let wait_result = wait_for_bundle_confirmation(
            |id| async { Ok(generate_statuses(id, "processed")) },
            "6e4b90284778a40633b56e4289202ea79e62d2296bb3d45398bb93f6c9ec083d".to_string(),
            Duration::from_secs(1),
            Duration::from_secs(2),
        )
        .await;
        assert!(wait_result.is_err());
    }
}
