use std::str::FromStr;

use anyhow::{Context, Result};
use raydium_library::amm::load_amm_keys;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use tracing::{debug, warn};

use crate::swap::Swap;

impl Swap {
    pub async fn get_pool(&self, pool_id: &str) -> Result<f64> {
        let client = self
            .client_blocking
            .clone()
            .context("failed to get rpc client")?;
        let owner = self.keypair.pubkey();
        let amm_program = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?;
        // let market_program = Pubkey::from_str("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX")?;
        let amm_pool_id =
            Pubkey::from_str(pool_id).inspect_err(|err| warn!("failed parse pool_id: {}", err))?;

        // let account = self.client.get_account(&amm_pool_id).await.unwrap();
        let amm_keys = load_amm_keys(&client, &amm_program, &amm_pool_id)
            .inspect_err(|err| warn!("failed load amm keys: {}", err))?;

        debug!("amm_keys result: {:#?}", amm_keys);

        // load market keys
        let market_keys = raydium_library::amm::openbook::get_keys_for_market(
            &client,
            &amm_keys.market_program,
            &amm_keys.market,
        )
        .inspect_err(|e| {
            warn!("failed to get market_keys: {}", e);
        })?;

        // calculate amm pool vault with load data at the same time or use simulate to calculate
        let result = raydium_library::amm::calculate_pool_vault_amounts(
            &client,
            &amm_program,
            &amm_pool_id,
            &amm_keys,
            &market_keys,
            raydium_library::amm::utils::CalculateMethod::Simulate(owner),
        )?;
        debug!("calculate_pool result: {:#?}", result);

        Ok(0.0)
    }
}
