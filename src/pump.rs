use std::{str::FromStr, sync::Arc};

use anyhow::{anyhow, Context, Result};
use raydium_amm::math::U128;
use raydium_library::amm::TEN_THOUSAND;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};
use spl_token_client::token::TokenError;

use tracing::{debug, error, info, warn};

use crate::{
    get_client_build,
    swap::{SwapDirection, SwapInType},
    token, tx,
};

pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const RENT_PROGRAM: &str = "SysvarRent111111111111111111111111111111111";
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const PUMP_GLOBAL: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
pub const PUMP_FEE_RECIPIENT: &str = "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM";
pub const PUMP_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
// pub const PUMP_FUN_MINT_AUTHORITY: &str = "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM";
pub const PUMP_ACCOUNT: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
pub const PUMP_BUY_METHOD: u64 = 16927863322537952870;
pub const PUMP_SELL_METHOD: u64 = 12502976635542562355;

pub struct Pump {
    pub client: Arc<RpcClient>,
    pub keypair: Arc<Keypair>,
    pub client_blocking: Option<Arc<solana_client::rpc_client::RpcClient>>,
}

impl Pump {
    pub fn new(client: Arc<RpcClient>, keypair: Arc<Keypair>) -> Self {
        Self {
            client,
            keypair,
            client_blocking: None,
        }
    }

    pub fn with_blocking_client(
        &mut self,
        client: Arc<solana_client::rpc_client::RpcClient>,
    ) -> &mut Self {
        self.client_blocking = Some(client);
        self
    }

    pub async fn swap(
        &self,
        mint: &str,
        amount_in: f64,
        swap_direction: SwapDirection,
        in_type: SwapInType,
        slippage: u64,
        use_jito: bool,
    ) -> Result<()> {
        // slippage_bps = 50u64; // 0.5%
        let slippage_bps = slippage * 100;
        let owner = self.keypair.pubkey();
        let mint =
            Pubkey::from_str(mint).map_err(|e| anyhow!("failed to parse mint pubkey: {}", e))?;
        let program_id = spl_token::ID;
        let native_mint = spl_token::native_mint::ID;

        let (token_in, token_out, pump_method) = match swap_direction {
            SwapDirection::Buy => (native_mint, mint, PUMP_BUY_METHOD),
            SwapDirection::Sell => (mint, native_mint, PUMP_SELL_METHOD),
        };

        let pump_info = get_pump_info(&mint.to_string()).await?;

        let in_ata = token::get_associated_token_address(
            self.client.clone(),
            self.keypair.clone(),
            &token_in,
            &owner,
        );
        let in_account = token::get_account_info(
            self.client.clone(),
            self.keypair.clone(),
            &token_in,
            &in_ata,
        )
        .await?;
        let in_mint =
            token::get_mint_info(self.client.clone(), self.keypair.clone(), &token_in).await?;
        let out_ata = token::get_associated_token_address(
            self.client.clone(),
            self.keypair.clone(),
            &token_out,
            &owner,
        );

        let mut create_instruction = None;
        let mut close_instruction = None;

        let (amount_specified, amount_ui_pretty) = match swap_direction {
            SwapDirection::Buy => {
                // Create base ATA if it doesn't exist.
                match token::get_account_info(
                    self.client.clone(),
                    self.keypair.clone(),
                    &token_out,
                    &out_ata,
                )
                .await
                {
                    Ok(_) => debug!("base ata exists. skipping creation.."),
                    Err(TokenError::AccountNotFound) | Err(TokenError::AccountInvalidOwner) => {
                        info!(
                            "base ATA for mint {} does not exist. will be create",
                            token_out
                        );
                        create_instruction = Some(create_associated_token_account(
                            &owner,
                            &owner,
                            &token_out,
                            &program_id,
                        ));
                    }
                    Err(error) => error!("error retrieving out ATA: {}", error),
                }

                (
                    ui_amount_to_amount(amount_in, spl_token::native_mint::DECIMALS),
                    (amount_in, spl_token::native_mint::DECIMALS),
                )
            }
            SwapDirection::Sell => {
                let amount = match in_type {
                    SwapInType::Qty => ui_amount_to_amount(amount_in, in_mint.base.decimals),
                    SwapInType::Pct => {
                        let amount_in_pct = amount_in.min(1.0);
                        if amount_in_pct == 1.0 {
                            // sell all, close ata
                            info!("sell all. will be close ATA for mint {}", token_in);
                            close_instruction = Some(spl_token::instruction::close_account(
                                &program_id,
                                &in_ata,
                                &owner,
                                &owner,
                                &vec![&owner],
                            )?);
                            in_account.base.amount
                        } else {
                            (amount_in_pct * 100.0) as u64 * in_account.base.amount / 100
                        }
                    }
                };
                (
                    amount,
                    (
                        amount_to_ui_amount(amount, in_mint.base.decimals),
                        in_mint.base.decimals,
                    ),
                )
            }
        };

        info!(
            "swap: {}, value: {:?} -> {}",
            token_in, amount_ui_pretty, token_out
        );

        let client = self
            .client_blocking
            .clone()
            .context("failed to get rpc client")?;

        let bonding_curve = Pubkey::from_str(&pump_info.bonding_curve)
            .map_err(|e| anyhow!("failed to parse associated_bonding_curve pubkey: {}", e))?;
        let associated_bonding_curve = Pubkey::from_str(&pump_info.associated_bonding_curve)
            .map_err(|e| anyhow!("failed to parse associated_bonding_curve pubkey: {}", e))?;
        let pump_program = Pubkey::from_str(PUMP_PROGRAM)?;

        // Calculate tokens out
        let virtual_sol_reserves = U128::from(pump_info.virtual_sol_reserves);
        let virtual_token_reserves = U128::from(pump_info.virtual_token_reserves);
        let unit_price = (pump_info.virtual_sol_reserves as f64
            / pump_info.virtual_token_reserves as f64)
            / 1000.0;

        let (token_amount, sol_amount_threshold, input_accouts) = match swap_direction {
            SwapDirection::Buy => {
                let max_sol_cost = max_amount_with_slippage(amount_specified, slippage_bps);

                (
                    U128::from(amount_specified)
                        .checked_mul(virtual_token_reserves)
                        .unwrap()
                        .checked_div(virtual_sol_reserves)
                        .unwrap()
                        .as_u64(),
                    max_sol_cost,
                    vec![
                        AccountMeta::new_readonly(Pubkey::from_str(PUMP_GLOBAL)?, false),
                        AccountMeta::new(Pubkey::from_str(PUMP_FEE_RECIPIENT)?, false),
                        AccountMeta::new_readonly(mint, false),
                        AccountMeta::new(bonding_curve, false),
                        AccountMeta::new(associated_bonding_curve, false),
                        AccountMeta::new(out_ata, false),
                        AccountMeta::new(owner, true),
                        AccountMeta::new_readonly(system_program::id(), false),
                        AccountMeta::new_readonly(program_id, false),
                        AccountMeta::new_readonly(Pubkey::from_str(RENT_PROGRAM)?, false),
                        AccountMeta::new_readonly(Pubkey::from_str(PUMP_ACCOUNT)?, false),
                        AccountMeta::new_readonly(pump_program, false),
                    ],
                )
            }
            SwapDirection::Sell => {
                let sol_output = U128::from(amount_specified)
                    .checked_mul(virtual_sol_reserves)
                    .unwrap()
                    .checked_div(virtual_token_reserves)
                    .unwrap()
                    .as_u64();
                let min_sol_output = min_amount_with_slippage(sol_output, slippage_bps);

                (
                    amount_specified,
                    min_sol_output,
                    vec![
                        AccountMeta::new_readonly(Pubkey::from_str(PUMP_GLOBAL)?, false),
                        AccountMeta::new(Pubkey::from_str(PUMP_FEE_RECIPIENT)?, false),
                        AccountMeta::new_readonly(mint, false),
                        AccountMeta::new(bonding_curve, false),
                        AccountMeta::new(associated_bonding_curve, false),
                        AccountMeta::new(in_ata, false),
                        AccountMeta::new(owner, true),
                        AccountMeta::new_readonly(system_program::id(), false),
                        AccountMeta::new_readonly(
                            Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM)?,
                            false,
                        ),
                        AccountMeta::new_readonly(program_id, false),
                        AccountMeta::new_readonly(Pubkey::from_str(PUMP_ACCOUNT)?, false),
                        AccountMeta::new_readonly(pump_program, false),
                    ],
                )
            }
        };

        info!(
            "token_amount: {}, sol_amount_threshold: {}, unit_price: {} sol",
            token_amount, sol_amount_threshold, unit_price
        );

        let build_swap_instruction = Instruction::new_with_bincode(
            pump_program,
            &(pump_method, token_amount, sol_amount_threshold),
            input_accouts,
        );
        // build instructions
        let mut instructions = vec![];
        if let Some(create_instruction) = create_instruction {
            instructions.push(create_instruction);
        }
        if amount_specified > 0 {
            instructions.push(build_swap_instruction)
        }
        if let Some(close_instruction) = close_instruction {
            instructions.push(close_instruction);
        }
        if instructions.len() == 0 {
            return Err(anyhow!("instructions is empty, no tx required"));
        }

        tx::new_signed_and_send(&client, &self.keypair, instructions, use_jito).await
    }
}

fn min_amount_with_slippage(input_amount: u64, slippage_bps: u64) -> u64 {
    input_amount
        .checked_mul(TEN_THOUSAND.checked_sub(slippage_bps).unwrap())
        .unwrap()
        .checked_div(TEN_THOUSAND)
        .unwrap()
}
fn max_amount_with_slippage(input_amount: u64, slippage_bps: u64) -> u64 {
    input_amount
        .checked_mul(slippage_bps.checked_add(TEN_THOUSAND).unwrap())
        .unwrap()
        .checked_div(TEN_THOUSAND)
        .unwrap()
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PumpInfo {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub description: Value,
    pub image_uri: Value,
    pub metadata_uri: Value,
    pub twitter: Value,
    pub telegram: Value,
    pub bonding_curve: String,
    pub associated_bonding_curve: String,
    pub creator: String,
    pub created_timestamp: u64,
    pub raydium_pool: Value,
    pub complete: bool,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub total_supply: u64,
    pub website: Value,
    pub show_name: bool,
    pub king_of_the_hill_timestamp: Value,
    pub market_cap: f64,
    pub reply_count: u64,
    pub last_reply: Value,
    pub nsfw: bool,
    pub market_id: Value,
    pub inverted: Value,
    pub is_currently_live: bool,
    pub username: Value,
    pub profile_image: Value,
    pub usd_market_cap: f64,
}

// https://frontend-api.pump.fun/coins/8zSLdDzM1XsqnfrHmHvA9ir6pvYDjs8UXz6B2Tydd6b2
pub async fn get_pump_info(mint: &str) -> Result<PumpInfo> {
    let client = get_client_build()?;
    let result = client
        .get(format!("https://frontend-api.pump.fun/coins/{}", mint))
        .send()
        .await?
        .json::<PumpInfo>()
        .await
        .context("Failed to parse pump info JSON")?;
    Ok(result)
}

pub async fn is_pump_funning(mint: &str) -> Result<bool> {
    match get_pump_info(mint).await {
        Ok(pump_info) => Ok(pump_info.raydium_pool.is_null()),
        Err(err) => {
            warn!("is_pump_funning: {}", err);
            Ok(false)
        }
    }
}
