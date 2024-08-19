use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use spl_token_2022::amount_to_ui_amount;
use tracing::{debug, info, warn};

use crate::{
    constants::Symbol,
    raydium::{get_pool_info_by_id, get_price},
    swap::Swap,
};

impl Swap {
    pub async fn get_pool(&self, pool_id: &str) -> Result<(f64, f64, f64, f64)> {
        let amm_pool_id =
            Pubkey::from_str(pool_id).inspect_err(|err| warn!("failed parse pool_id: {}", err))?;
        let pool_info = get_pool_info_by_id(pool_id)
            .await?
            .get_pool()
            .ok_or(anyhow!("pool is empty"))?;
        debug!("amm pool id: {:?}", amm_pool_id);

        let client = self
            .client_blocking
            .clone()
            .context("failed to get rpc client")?;
        let owner = self.keypair.pubkey();
        let amm_program = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?;
        let market_program = Pubkey::from_str("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX")?;
        let market_id = Pubkey::from_str(&pool_info.market_id)?;
        let mint_a = (
            Pubkey::from_str(&pool_info.mint_a.address)?,
            pool_info.mint_a.decimals,
            pool_info.mint_a.name,
        );
        let mint_b = (
            Pubkey::from_str(&pool_info.mint_b.address)?,
            pool_info.mint_b.decimals,
            pool_info.mint_b.name,
        );
        debug!("{mint_a:#?}, {mint_b:#?}");

        let amm_keys = raydium_library::amm::utils::get_amm_pda_keys(
            &amm_program,
            &market_program,
            &market_id,
            &mint_a.0,
            &mint_b.0,
        )?;
        debug!("amm_keys: {amm_keys:#?}");
        if amm_keys.amm_pool != amm_pool_id {
            warn!("amm_keys's amm_pool not match input pool_id");
            return Err(anyhow!("internal error"));
        }

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
        let calculate_result = raydium_library::amm::calculate_pool_vault_amounts(
            &client,
            &amm_program,
            &amm_pool_id,
            &amm_keys,
            &market_keys,
            raydium_library::amm::utils::CalculateMethod::Simulate(owner),
        )?;
        debug!("calculate_pool result: {:#?}", calculate_result);

        let mut pool_pc = (mint_b.clone(), calculate_result.pool_pc_vault_amount);
        let mut pool_coin = (mint_a.clone(), calculate_result.pool_coin_vault_amount);

        if amm_keys.amm_pc_mint != spl_token::native_mint::ID {
            pool_pc = (mint_a, calculate_result.pool_coin_vault_amount);
            pool_coin = (mint_b, calculate_result.pool_pc_vault_amount)
        }

        let pool_pc_ui_amount = amount_to_ui_amount(pool_pc.1, pool_pc.0 .1);
        let pool_coin_ui_amount = amount_to_ui_amount(pool_coin.1, pool_coin.0 .1);
        let unit_price = pool_pc_ui_amount / pool_coin_ui_amount;
        info!(
            "calculate pool: {}: {}, {}: {}, unit_price: {} wsol",
            pool_pc.0 .2, pool_pc_ui_amount, pool_coin.0 .2, pool_coin_ui_amount, unit_price
        );
        let sol_price = get_price(Symbol::SOLANA)
            .await
            .inspect_err(|err| warn!("failed get solana price: {}", err))?;
        let coin_price = ((unit_price * sol_price) * 1_000_000_000.0).round() / 1_000_000_000.0;

        info!(
            "sol price: {}, {} price: {} ",
            sol_price, pool_coin.0 .2, coin_price
        );

        Ok((
            pool_coin_ui_amount,
            pool_pc_ui_amount,
            coin_price,
            sol_price,
        ))
    }
}
