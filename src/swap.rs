use std::{str::FromStr, sync::Arc};

use anyhow::{anyhow, Context, Result};
use clap::ValueEnum;
use raydium_library::amm;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};
use spl_token_client::{
    client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
    token::{Token, TokenError},
};

use tracing::{debug, error, info};

use crate::{get_rpc_client_blocking, raydium::get_pool_info};

pub struct Swap {
    client: Arc<RpcClient>,
    keypair: Keypair,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum SwapDirection {
    Buy,
    Sell,
}
impl From<SwapDirection> for u8 {
    fn from(value: SwapDirection) -> Self {
        match value {
            SwapDirection::Buy => 0,
            SwapDirection::Sell => 1,
        }
    }
}
#[derive(ValueEnum, Debug, Clone)]
pub enum SwapInType {
    /// Quantity
    Qty,
    /// Percentage
    Pct,
}

impl Swap {
    pub fn new(client: Arc<RpcClient>, keypair: Keypair) -> Self {
        Self { client, keypair }
    }

    fn program_client(&self) -> Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> {
        Arc::new(ProgramRpcClient::new(
            self.client.clone(),
            ProgramRpcClientSendTransaction,
        ))
    }
    fn keypair(&self) -> Keypair {
        Keypair::from_bytes(&self.keypair.to_bytes()).expect("failed to copy keypair")
    }

    pub async fn swap(
        &self,
        mint: &str,
        amount_in: f64,
        swap_direction: SwapDirection,
        in_type: SwapInType,
        slippage: u64,
    ) -> Result<bool> {
        // slippage_bps = 50u64; // 0.5%
        let slippage_bps = slippage * 100;
        let owner = self.keypair.pubkey();
        let mint =
            Pubkey::from_str(mint).map_err(|e| anyhow!("failed to parse mint pubkey: {}", e))?;
        let program_client = self.program_client();
        let program_id = spl_token::ID;
        let native_mint = spl_token::native_mint::ID;

        let (token_in, token_out) = match swap_direction {
            SwapDirection::Buy => (native_mint, mint),
            SwapDirection::Sell => (mint, native_mint),
        };

        let token_in_client = Token::new(
            program_client.clone(),
            &program_id,
            &token_in,
            None,
            Arc::new(self.keypair()),
        );
        let token_out_client = Token::new(
            program_client.clone(),
            &program_id,
            &token_out,
            None,
            Arc::new(self.keypair()),
        );

        let pool_info = get_pool_info(&token_in.to_string(), &token_out.to_string()).await?;

        let in_ata = token_in_client.get_associated_token_address(&owner);
        let in_account = token_in_client.get_account_info(&in_ata).await?;
        let in_mint = token_in_client.get_mint_info().await?;
        // If input token is the native mint(wSOL) and the balance is inadequate, attempt to
        // convert SOL to wSOL.
        let in_balance = in_account.base.amount;
        // ui amount -> amount
        let mut amount_specified = ui_amount_to_amount(amount_in, in_mint.base.decimals);
        if in_account.base.is_native() && in_balance < amount_specified {
            let transfer_amt = amount_specified - in_balance;
            let blockhash = self.client.get_latest_blockhash().await?;
            let transfer_instruction =
                solana_sdk::system_instruction::transfer(&owner, &in_ata, transfer_amt);
            let sync_instruction = spl_token::instruction::sync_native(&program_id, &in_ata)?;
            let tx = Transaction::new_signed_with_payer(
                &[transfer_instruction, sync_instruction],
                Some(&owner),
                &[&self.keypair],
                blockhash,
            );
            self.client.send_and_confirm_transaction(&tx).await.unwrap();
        }

        // Create the out ATA if it doesn't exist.
        let out_ata = token_out_client.get_associated_token_address(&owner);
        debug!("out ATA={}", out_ata);
        let mut create_instruction = None;
        match token_out_client.get_account_info(&out_ata).await {
            Ok(_) => debug!("out ata exists. skipping creation.."),
            Err(TokenError::AccountNotFound) | Err(TokenError::AccountInvalidOwner) => {
                info!("out ATA does not exist. will be create");
                // token_out_client
                //     .create_associated_token_account(&owner)
                //     .await?;
                create_instruction = Some(create_associated_token_account(
                    &owner,
                    &owner,
                    &token_out,
                    &program_id,
                ));
            }
            Err(error) => error!("error retrieving out ATA: {}", error),
        }

        let amm_program = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?;
        let market_program = Pubkey::from_str("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX")?;
        let market_id = Pubkey::from_str(
            &pool_info
                .get_market_id()
                .with_context(|| "failed to get pool market id")?,
        )?;
        let amm_pool_id = Pubkey::from_str(
            &pool_info
                .get_pool_id()
                .with_context(|| "failed to get pool id")?,
        )?;
        debug!("amm pool id: {amm_pool_id}");
        let client = get_rpc_client_blocking()?;
        let mut swap_base_in = false;

        // load amm keys
        let amm_keys_pda = raydium_library::amm::utils::get_amm_pda_keys(
            &amm_program,
            &market_program,
            &market_id,
            &mint,
            &native_mint,
        )?;
        // debug!("amm_keys_pda: {amm_keys_pda:#?}");
        let amm_keys = amm_keys_pda;
        // let amm_keys =
        //     raydium_library::amm::utils::load_amm_keys(&client, &amm_program, &amm_pool_id)
        //         .inspect_err(|e| {
        //             error!("failed to get amm_keys: {}", e);
        //         })?;
        debug!("amm_keys: {amm_keys:#?}");
        // load market keys
        let market_keys = raydium_library::amm::openbook::get_keys_for_market(
            &client,
            &amm_keys.market_program,
            &amm_keys.market,
        )
        .inspect_err(|e| {
            error!("failed to get market_keys: {}", e);
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
        let close_instruction = None;
        let direction = if token_in == amm_keys.amm_coin_mint && token_out == amm_keys.amm_pc_mint {
            info!("sell {}", token_in);
            // if sell, use swap in type
            match in_type {
                SwapInType::Qty => {
                    amount_specified = amount_specified.min(in_account.base.amount);
                }
                SwapInType::Pct => {
                    let amount_in_pct = amount_in.min(1.0);
                    info!("amount_in_pct: {}, amount_in: {}", amount_in_pct, amount_in);
                    if amount_in_pct == 1.0 {
                        // sell all, close ata
                        info!("sell all. will be close ATA");
                        amount_specified = in_account.base.amount;
                        // close_instruction = Some(spl_token::instruction::close_account(
                        //     &owner,
                        //     &in_ata,
                        //     &owner,
                        //     &owner,
                        //     &vec![&owner],
                        // )?);
                    } else {
                        amount_specified =
                            (amount_in_pct * 100.0) as u64 * in_account.base.amount / 100;
                    }
                }
            }
            amm::utils::SwapDirection::Coin2PC
        } else {
            info!("buy {}", token_out);
            // if buy, use swap_base_in
            swap_base_in = true;
            amm::utils::SwapDirection::PC2Coin
        };

        info!(
            "swap[{}]: {} -> {} -> {}",
            swap_base_in,
            token_in,
            amount_to_ui_amount(amount_specified, in_mint.base.decimals),
            token_out
        );
        let other_amount_threshold = raydium_library::amm::swap_with_slippage(
            result.pool_pc_vault_amount,
            result.pool_coin_vault_amount,
            result.swap_fee_numerator,
            result.swap_fee_denominator,
            direction,
            amount_specified,
            swap_base_in,
            slippage_bps,
        )?;
        // build swap instruction
        let build_swap_instruction = raydium_library::amm::swap(
            &amm_program,
            &amm_keys,
            &market_keys,
            &owner,
            &in_ata,
            &out_ata,
            amount_specified,
            other_amount_threshold,
            swap_base_in,
        )?;
        // build instructions
        let mut instructions = vec![build_swap_instruction];
        if let Some(create_instruction) = create_instruction {
            instructions.push(create_instruction);
        }
        if let Some(close_instruction) = close_instruction {
            instructions.push(close_instruction);
        }

        // send init tx
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&owner),
            &vec![&self.keypair],
            client.get_latest_blockhash()?,
        );
        let sig = raydium_library::common::rpc::send_txn(&client, &txn, true)?;
        info!("signature: {:?}", sig);
        Ok(true)
    }
}
