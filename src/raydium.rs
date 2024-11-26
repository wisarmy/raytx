use std::env;

use anyhow::{anyhow, Context, Result};
use raydium_library::amm;
use reqwest::Proxy;
use serde::Deserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    // native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};
use spl_token_client::token::TokenError;
use std::{str::FromStr, sync::Arc};

use crate::{
    get_rpc_client_blocking,
    swap::{SwapDirection, SwapInType},
    token, tx,
};
use spl_token::state::Account;

use tracing::{debug, error, info};

pub struct Raydium {
    pub client: Arc<RpcClient>,
    pub keypair: Arc<Keypair>,
    pub client_blocking: Option<Arc<solana_client::rpc_client::RpcClient>>,
}

impl Raydium {
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
    ) -> Result<Vec<String>> {
        // slippage_bps = 50u64; // 0.5%
        let slippage_bps = slippage * 100;
        let owner = self.keypair.pubkey();
        let mint =
            Pubkey::from_str(mint).map_err(|e| anyhow!("failed to parse mint pubkey: {}", e))?;
        let program_id = spl_token::ID;
        let native_mint = spl_token::native_mint::ID;

        let (token_in, token_out) = match swap_direction {
            SwapDirection::Buy => (native_mint, mint),
            SwapDirection::Sell => (mint, native_mint),
        };
        let pool_info = get_pool_info(&token_in.to_string(), &token_out.to_string()).await?;

        let in_ata = token::get_associated_token_address(
            self.client.clone(),
            self.keypair.clone(),
            &token_in,
            &owner,
        );
        let out_ata = token::get_associated_token_address(
            self.client.clone(),
            self.keypair.clone(),
            &token_out,
            &owner,
        );

        let mut create_instruction = None;
        let mut close_instruction = None;
        let swap_base_in = true;

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
                        // token::create_associated_token_account(
                        //     self.client.clone(),
                        //     self.keypair.clone(),
                        //     &token_out,
                        //     &owner,
                        // )
                        // .await?;
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

        // load amm keys
        // since load_amm_keys is not available, get_amm_pda_keys is used here,
        // and the parameters(coin_mint, pc_mint) look a little strange.
        let amm_keys = raydium_library::amm::utils::get_amm_pda_keys(
            &amm_program,
            &market_program,
            &market_id,
            &mint,
            &native_mint,
        )?;
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
        // setting direction
        let (mut direction, mut direction_str) = match swap_direction {
            SwapDirection::Buy => (amm::utils::SwapDirection::PC2Coin, "PC2Coin"),
            SwapDirection::Sell => (amm::utils::SwapDirection::Coin2PC, "Coin2PC"),
        };
        // if mint_a is native mint, reverse direction
        if pool_info
            .get_pool()
            .ok_or(anyhow!("failed to get pool"))?
            .mint_a
            .address
            == native_mint.to_string()
        {
            (direction, direction_str) = match direction {
                amm::utils::SwapDirection::PC2Coin => {
                    (amm::utils::SwapDirection::Coin2PC, "Coin2PC")
                }
                amm::utils::SwapDirection::Coin2PC => {
                    (amm::utils::SwapDirection::PC2Coin, "PC2Coin")
                }
            };
        }
        debug!("direction: {}", direction_str);

        info!(
            "swap: {}, value: {:?} -> {}",
            token_in, amount_ui_pretty, token_out
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
        // build instructions
        let mut instructions = vec![];
        // sol <-> wsol support
        let mut wsol_account = None;
        if token_in == native_mint || token_out == native_mint {
            // create wsol account
            let seed = &format!("{}", Keypair::new().pubkey())[..32];
            let wsol_pubkey = Pubkey::create_with_seed(&owner, seed, &spl_token::id())?;
            wsol_account = Some(wsol_pubkey);

            // LAMPORTS_PER_SOL / 100 // 0.01 SOL as rent
            // get rent
            let rent = self
                .client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .await?;
            // if buy add amount_specified
            let total_amount = if token_in == native_mint {
                rent + amount_specified
            } else {
                rent
            };
            // create tmp wsol account
            instructions.push(system_instruction::create_account_with_seed(
                &owner,
                &wsol_pubkey,
                &owner,
                seed,
                total_amount,
                Account::LEN as u64, // 165, // Token account size
                &spl_token::id(),
            ));

            // initialize account
            instructions.push(spl_token::instruction::initialize_account(
                &spl_token::id(),
                &wsol_pubkey,
                &native_mint,
                &owner,
            )?);
        }

        if let Some(create_instruction) = create_instruction {
            instructions.push(create_instruction);
        }
        if amount_specified > 0 {
            let mut close_wsol_account_instruction = None;
            // replace native mint with tmp wsol account
            let mut final_in_ata = in_ata;
            let mut final_out_ata = out_ata;

            if let Some(wsol_account) = wsol_account {
                match swap_direction {
                    SwapDirection::Buy => {
                        final_in_ata = wsol_account;
                    }
                    SwapDirection::Sell => {
                        final_out_ata = wsol_account;
                    }
                }
                close_wsol_account_instruction = Some(spl_token::instruction::close_account(
                    &program_id,
                    &wsol_account,
                    &owner,
                    &owner,
                    &vec![&owner],
                )?);
            }

            // build swap instruction
            let build_swap_instruction = raydium_library::amm::swap(
                &amm_program,
                &amm_keys,
                &market_keys,
                &owner,
                &final_in_ata,
                &final_out_ata,
                amount_specified,
                other_amount_threshold,
                swap_base_in,
            )?;
            info!(
                "amount_specified: {}, other_amount_threshold: {}, wsol_account: {:?}",
                amount_specified, other_amount_threshold, wsol_account
            );
            instructions.push(build_swap_instruction);
            // close wsol account
            if let Some(close_wsol_account_instruction) = close_wsol_account_instruction {
                instructions.push(close_wsol_account_instruction);
            }
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

// get pool info
// https://api-v3.raydium.io/pools/info/mint?mint1=So11111111111111111111111111111111111111112&mint2=EzM2d8JVpzfhV7km3tUsR1U1S4xwkrPnWkM4QFeTpump&poolType=standard&poolSortField=default&sortType=desc&pageSize=10&page=1
pub async fn get_pool_info(mint1: &str, mint2: &str) -> Result<PoolInfo> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = env::var("HTTP_PROXY") {
        let proxy = Proxy::all(http_proxy)?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build()?;

    let result = client
        .get("https://api-v3.raydium.io/pools/info/mint")
        .query(&[
            ("mint1", mint1),
            ("mint2", mint2),
            ("poolType", "standard"),
            ("poolSortField", "default"),
            ("sortType", "desc"),
            ("pageSize", "1"),
            ("page", "1"),
        ])
        .send()
        .await?
        .json::<PoolInfo>()
        .await
        .context("Failed to parse pool info JSON")?;
    Ok(result)
}
// get pool info by ids
// https://api-v3.raydium.io/pools/info/ids?ids=3RHg85W1JtKeqFQSxBfd2RX13aBFvvy6gcATkHU657mL
pub async fn get_pool_info_by_id(pool_id: &str) -> Result<PoolData> {
    let mut client_builder = reqwest::Client::builder();
    if let Ok(http_proxy) = env::var("HTTP_PROXY") {
        let proxy = Proxy::all(http_proxy)?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build()?;

    let result = client
        .get("https://api-v3.raydium.io/pools/info/ids")
        .query(&[("ids", pool_id)])
        .send()
        .await?
        .json::<PoolData>()
        .await
        .context("Failed to parse pool info JSON")?;
    Ok(result)
}

impl PoolInfo {
    pub fn get_pool_id(&self) -> Option<String> {
        if let Some(pool) = self.data.get_pool() {
            Some(pool.id.clone())
        } else {
            None
        }
    }
    pub fn get_market_id(&self) -> Option<String> {
        if let Some(pool) = self.data.get_pool() {
            Some(pool.market_id.clone())
        } else {
            None
        }
    }
    pub fn get_pool(&self) -> Option<Pool> {
        self.data.get_pool()
    }
}

#[derive(Debug, Deserialize)]
pub struct PoolInfo {
    pub success: bool,
    pub data: PoolData,
}

#[derive(Debug, Deserialize)]
pub struct PoolData {
    // pub count: u32,
    pub data: Vec<Pool>,
}

impl PoolData {
    pub fn get_pool(&self) -> Option<Pool> {
        self.data.first().cloned()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Pool {
    pub id: String,
    #[serde(rename = "programId")]
    pub program_id: String,
    #[serde(rename = "mintA")]
    pub mint_a: Mint,
    #[serde(rename = "mintB")]
    pub mint_b: Mint,
    #[serde(rename = "marketId")]
    pub market_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Mint {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}
