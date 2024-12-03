use anyhow::{Context, Result};
use common::common_utils;
use spl_token_2022::amount_to_ui_amount;
use tracing::{debug, warn};

use crate::{
    helper::get_solana_price,
    raydium::{get_pool_state, Raydium},
};

impl Raydium {
    pub async fn get_pool(&self, pool_id: &str) -> Result<(f64, f64, f64, f64, f64)> {
        let (base, quote, price) = self.get_pool_price(Some(pool_id), None).await?;
        let sol_price = get_solana_price()
            .await
            .inspect_err(|err| warn!("failed get solana price: {}", err))?;
        let usd_price = ((price * sol_price) * 1_000_000_000.0).round() / 1_000_000_000.0;

        debug!("sol price: {}, usd_price: {} ", sol_price, usd_price);

        Ok((base, quote, price, usd_price, sol_price))
    }

    pub async fn get_pool_price(
        &self,
        pool_id: Option<&str>,
        mint: Option<&str>,
    ) -> Result<(f64, f64, f64)> {
        let client = self
            .client_blocking
            .clone()
            .context("failed to get rpc client")?;

        let (amm_pool_id, pool_state) = get_pool_state(client.clone(), pool_id, mint).await?;

        // debug!("pool_state : {:#?}", pool_state);

        let load_pubkeys = vec![pool_state.pc_vault, pool_state.coin_vault];
        let rsps = common::rpc::get_multiple_accounts(&client, &load_pubkeys).unwrap();

        let amm_pc_vault_account = rsps[0].clone();
        let amm_coin_vault_account = rsps[1].clone();

        let amm_pc_vault =
            common_utils::unpack_token(&amm_pc_vault_account.as_ref().unwrap().data).unwrap();
        let amm_coin_vault =
            common_utils::unpack_token(&amm_coin_vault_account.as_ref().unwrap().data).unwrap();

        let (base_account, quote_account) = if amm_coin_vault.base.is_native() {
            (
                (
                    pool_state.pc_vault_mint,
                    amount_to_ui_amount(amm_pc_vault.base.amount, pool_state.pc_decimals as u8),
                ),
                (
                    pool_state.coin_vault_mint,
                    amount_to_ui_amount(amm_coin_vault.base.amount, pool_state.coin_decimals as u8),
                ),
            )
        } else {
            (
                (
                    pool_state.coin_vault_mint,
                    amount_to_ui_amount(amm_coin_vault.base.amount, pool_state.coin_decimals as u8),
                ),
                (
                    pool_state.pc_vault_mint,
                    amount_to_ui_amount(amm_pc_vault.base.amount, pool_state.pc_decimals as u8),
                ),
            )
        };

        let price = quote_account.1 / base_account.1;

        debug!(
            "calculate pool[{}]: {}: {}, {}: {}, price: {} sol",
            amm_pool_id, base_account.0, base_account.1, quote_account.0, quote_account.1, price
        );

        Ok((base_account.1, quote_account.1, price))
    }
}
