use std::{str::FromStr, sync::LazyLock};

use anyhow::{anyhow, Result};
use api::{get_tip_accounts, TipAccountResult};
use rand::{seq::IteratorRandom, thread_rng};
use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::error;
use ws::TIPS_PERCENTILE;

use crate::get_env_var;

pub mod api;
pub mod ws;

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
    let mut rng = thread_rng();
    let accounts = TIP_ACCOUNTS.read().await;
    match accounts.iter().choose(&mut rng) {
        Some(acc) => Ok(Pubkey::from_str(acc).inspect_err(|err| {
            error!("jito: failed to parse Pubkey: {:?}", err);
        })?),
        None => Err(anyhow!("jito: no tip accounts available")),
    }
}
// unit sol
pub async fn get_tip_value() -> Result<f64> {
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
