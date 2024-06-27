use std::{str::FromStr, sync::Arc};

use anyhow::{anyhow, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use spl_token_client::{
    client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
    spl_token_2022::ui_amount_to_amount,
    token::{Token, TokenError},
};

use tracing::{debug, error, info};

use crate::{
    raydium::get_pool_info,
    swap::{SwapDirection, SwapInType},
};

pub struct Swap {
    client: Arc<RpcClient>,
    keypair: Keypair,
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
    ) -> Result<bool> {
        let owner = self.keypair.pubkey();
        let mint =
            Pubkey::from_str(mint).map_err(|e| anyhow!("failed to parse mint pubkey: {}", e))?;
        let program_client = self.program_client();
        let program_id = spl_token::ID;
        let wsol = spl_token::native_mint::ID;

        let (token_in, token_out) = match swap_direction {
            SwapDirection::Buy => (wsol, mint),
            SwapDirection::Sell => (mint, wsol),
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
        let in_amount = ui_amount_to_amount(amount_in, in_mint.base.decimals);
        if in_account.base.is_native() && in_balance < in_amount {
            let transfer_amt = in_amount - in_balance;
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
        match token_out_client.get_account_info(&out_ata).await {
            Ok(_) => debug!("out ata exists. skipping creation.."),
            Err(TokenError::AccountNotFound) | Err(TokenError::AccountInvalidOwner) => {
                info!("out ATA does not exist. Creating..");
                token_out_client
                    .create_associated_token_account(&owner)
                    .await?;
            }
            Err(error) => error!("error retrieving out ATA: {}", error),
        }

        // compute amount out

        todo!()
    }
}
