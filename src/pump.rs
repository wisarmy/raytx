use std::{str::FromStr, sync::Arc};

use anyhow::{anyhow, Result};
use borsh::from_slice;
use borsh_derive::{BorshDeserialize, BorshSerialize};
use raydium_amm::math::U128;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};
use spl_token_client::token::TokenError;

use tracing::{debug, error, info, warn};

use crate::{
    swap::{SwapDirection, SwapInType},
    token, tx,
};
pub const TEN_THOUSAND: u64 = 10000;
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const RENT_PROGRAM: &str = "SysvarRent111111111111111111111111111111111";
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const PUMP_GLOBAL: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
pub const PUMP_FEE_RECIPIENT: &str = "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV";
pub const PUMP_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
// pub const PUMP_FUN_MINT_AUTHORITY: &str = "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM";
pub const PUMP_ACCOUNT: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
pub const PUMP_BUY_METHOD: u64 = 16927863322537952870;
pub const PUMP_SELL_METHOD: u64 = 12502976635542562355;

pub struct Pump {
    pub client: Arc<RpcClient>,
    pub keypair: Arc<Keypair>,
}

impl Pump {
    pub fn new(client: Arc<RpcClient>, keypair: Arc<Keypair>) -> Self {
        Self { client, keypair }
    }

    pub async fn swap(
        &self,
        mint: &str,
        amount_in: f64,
        swap_direction: SwapDirection,
        in_type: SwapInType,
        slippage: u64,
        use_jito: bool,
    ) -> Result<Vec<String>> {
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
        let pump_program = Pubkey::from_str(PUMP_PROGRAM)?;
        let (bonding_curve, associated_bonding_curve, bonding_curve_account) =
            get_bonding_curve_account(self.client.clone(), &mint, &pump_program).await?;
        let in_ata = get_associated_token_address(&owner, &token_in);
        let out_ata = get_associated_token_address(&owner, &token_out);

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
                let in_account = token::get_account_info(
                    self.client.clone(),
                    self.keypair.clone(),
                    &token_in,
                    &in_ata,
                )
                .await?;
                let in_mint =
                    token::get_mint_info(self.client.clone(), self.keypair.clone(), &token_in)
                        .await?;
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

        // Calculate tokens out
        let virtual_sol_reserves = U128::from(bonding_curve_account.virtual_sol_reserves);
        let virtual_token_reserves = U128::from(bonding_curve_account.virtual_token_reserves);
        let unit_price = (bonding_curve_account.virtual_sol_reserves as f64
            / bonding_curve_account.virtual_token_reserves as f64)
            / 1000.0;

        let creator = Pubkey::new_from_array(bonding_curve_account.creator);
        let creator_vault = get_creator_vault_pda(&creator, &pump_program)?;

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
                        AccountMeta::new(creator_vault, false),
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
                        AccountMeta::new_readonly(creator_vault, false),
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

        tx::new_signed_and_send(&self.client, &self.keypair, instructions, use_jito).await
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
pub struct RaydiumInfo {
    pub base: f64,
    pub quote: f64,
    pub price: f64,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PumpInfo {
    pub mint: String,
    pub bonding_curve: String,
    pub associated_bonding_curve: String,
    pub raydium_pool: Option<String>,
    pub raydium_info: Option<RaydiumInfo>,
    pub complete: bool,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub total_supply: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct BondingCurveAccount {
    pub discriminator: u64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
    pub creator: [u8; 32],
}

pub async fn get_bonding_curve_account(
    rpc_client: Arc<solana_client::rpc_client::RpcClient>,
    mint: &Pubkey,
    program_id: &Pubkey,
) -> Result<(Pubkey, Pubkey, BondingCurveAccount)> {
    let bonding_curve = get_pda(mint, program_id)?;
    let associated_bonding_curve = get_associated_token_address(&bonding_curve, &mint);
    let bonding_curve_data = rpc_client
        .get_account_data(&bonding_curve)
        .inspect_err(|err| {
            warn!(
                "Failed to get bonding curve account data: {}, err: {}",
                bonding_curve, err
            );
        })?;

    let bonding_curve_account = from_slice::<BondingCurveAccount>(&bonding_curve_data[..81])
        .map_err(|e| {
            anyhow!(
                "Failed to deserialize bonding curve account: {}",
                e.to_string()
            )
        })?;

    // println!("{:?}", bonding_curve_account);
    Ok((
        bonding_curve,
        associated_bonding_curve,
        bonding_curve_account,
    ))
}

pub fn get_pda(mint: &Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
    let seeds = [b"bonding-curve".as_ref(), mint.as_ref()];
    let (bonding_curve, _bump) = Pubkey::find_program_address(&seeds, program_id);
    Ok(bonding_curve)
}

pub fn get_creator_vault_pda(creator: &Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
    let seeds = [b"creator-vault".as_ref(), creator.as_ref()];
    let (creator_vault, _bump) = Pubkey::find_program_address(&seeds, program_id);
    Ok(creator_vault)
}

// https://frontend-api.pump.fun/coins/8zSLdDzM1XsqnfrHmHvA9ir6pvYDjs8UXz6B2Tydd6b2
pub async fn get_pump_info(
    rpc_client: Arc<solana_client::rpc_client::RpcClient>,
    mint: &str,
) -> Result<PumpInfo> {
    let mint = Pubkey::from_str(mint)?;
    let program_id = Pubkey::from_str(PUMP_PROGRAM)?;
    let (bonding_curve, associated_bonding_curve, bonding_curve_account) =
        get_bonding_curve_account(rpc_client, &mint, &program_id).await?;

    let pump_info = PumpInfo {
        mint: mint.to_string(),
        bonding_curve: bonding_curve.to_string(),
        associated_bonding_curve: associated_bonding_curve.to_string(),
        raydium_pool: None,
        raydium_info: None,
        complete: bonding_curve_account.complete,
        virtual_sol_reserves: bonding_curve_account.virtual_sol_reserves,
        virtual_token_reserves: bonding_curve_account.virtual_token_reserves,
        total_supply: bonding_curve_account.token_total_supply,
    };
    Ok(pump_info)
}
