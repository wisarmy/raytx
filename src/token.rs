use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_account_decoder::UiAccountData;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_2022::{
    extension::StateWithExtensionsOwned,
    state::{Account, Mint},
};
use spl_token_client::{
    client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
    token::{TokenError, TokenResult},
};
use tracing::{trace, warn};

pub type TokenAccounts = Vec<TokenAccount>;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenAccount {
    pub pubkey: String,
    pub mint: String,
    pub amount: String,
    pub ui_amount: f64,
}
#[derive(Debug, Serialize, Deserialize)]
struct ParsedAccount {
    program: String,
    parsed: Parsed,
    space: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Parsed {
    info: TokenInfo,
    #[serde(rename = "type")]
    account_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenInfo {
    is_native: bool,
    mint: String,
    owner: String,
    state: String,
    token_amount: Amount,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Amount {
    amount: String,
    decimals: u8,
    ui_amount: f64,
    ui_amount_string: String,
}

pub async fn token_account(
    client: &RpcClient,
    owner: &Pubkey,
    mint: Pubkey,
) -> Result<TokenAccount> {
    let token_accounts =
        token_accounts_filter(client, owner, TokenAccountsFilter::Mint(mint)).await?;
    token_accounts
        .first()
        .cloned()
        .ok_or(anyhow!("NotFound: token account not found"))
}

pub async fn token_accounts(client: &RpcClient, owner: &Pubkey) -> Result<TokenAccounts> {
    token_accounts_filter(
        client,
        owner,
        TokenAccountsFilter::ProgramId(spl_token::id()),
    )
    .await
}
async fn token_accounts_filter(
    client: &RpcClient,
    owner: &Pubkey,
    filter: TokenAccountsFilter,
) -> Result<TokenAccounts> {
    let token_accounts = client
        .get_token_accounts_by_owner(owner, filter)
        .await
        .expect("Failed to get token accounts");

    trace!("token_accounts: {:#?}", token_accounts);

    let mut tas: TokenAccounts = vec![];
    for token_account in token_accounts.into_iter() {
        let account_data = token_account.account.data;
        match account_data {
            UiAccountData::Json(parsed_account) => {
                let parsed: Parsed = serde_json::from_value(parsed_account.parsed)?;
                tas.push(TokenAccount {
                    pubkey: token_account.pubkey,
                    mint: parsed.info.mint,
                    amount: parsed.info.token_amount.amount,
                    ui_amount: parsed.info.token_amount.ui_amount,
                });
            }
            UiAccountData::LegacyBinary(_) | UiAccountData::Binary(_, _) => {
                continue;
            }
        }
    }

    Ok(tas)
}

pub async fn get_account_info(
    client: Arc<RpcClient>,
    _keypair: Arc<Keypair>,
    address: &Pubkey,
    account: &Pubkey,
) -> TokenResult<StateWithExtensionsOwned<Account>> {
    let program_client = Arc::new(ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction,
    ));
    let account = program_client
        .get_account(*account)
        .await
        .map_err(TokenError::Client)?
        .ok_or(TokenError::AccountNotFound)
        .inspect_err(|err| warn!("{} {}: mint {}", account, err, address))?;

    if account.owner != spl_token::ID {
        return Err(TokenError::AccountInvalidOwner);
    }
    let account = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
    if account.base.mint != *address {
        return Err(TokenError::AccountInvalidMint);
    }

    Ok(account)
}

// pub async fn get_account_info(
//     client: Arc<RpcClient>,
//     keypair: Arc<Keypair>,
//     address: &Pubkey,
//     account: &Pubkey,
// ) -> TokenResult<StateWithExtensionsOwned<Account>> {
//     let token_client = Token::new(
//         Arc::new(ProgramRpcClient::new(
//             client.clone(),
//             ProgramRpcClientSendTransaction,
//         )),
//         &spl_token::ID,
//         address,
//         None,
//         Arc::new(Keypair::from_bytes(&keypair.to_bytes()).expect("failed to copy keypair")),
//     );
//     token_client.get_account_info(account).await
// }

pub async fn get_mint_info(
    client: Arc<RpcClient>,
    _keypair: Arc<Keypair>,
    address: &Pubkey,
) -> TokenResult<StateWithExtensionsOwned<Mint>> {
    let program_client = Arc::new(ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction,
    ));
    let account = program_client
        .get_account(*address)
        .await
        .map_err(TokenError::Client)?
        .ok_or(TokenError::AccountNotFound)
        .inspect_err(|err| warn!("{} {}: mint {}", address, err, address))?;

    if account.owner != spl_token::ID {
        return Err(TokenError::AccountInvalidOwner);
    }

    let mint_result = StateWithExtensionsOwned::<Mint>::unpack(account.data).map_err(Into::into);
    let decimals: Option<u8> = None;
    if let (Ok(mint), Some(decimals)) = (&mint_result, decimals) {
        if decimals != mint.base.decimals {
            return Err(TokenError::InvalidDecimals);
        }
    }

    mint_result
}

// pub async fn get_mint_info(
//     client: Arc<RpcClient>,
//     keypair: Arc<Keypair>,
//     address: &Pubkey,
// ) -> TokenResult<StateWithExtensionsOwned<Mint>> {
//     let token_client = Token::new(
//         Arc::new(ProgramRpcClient::new(
//             client.clone(),
//             ProgramRpcClientSendTransaction,
//         )),
//         &spl_token::ID,
//         address,
//         None,
//         Arc::new(Keypair::from_bytes(&keypair.to_bytes()).expect("failed to copy keypair")),
//     );
//     token_client.get_mint_info().await
// }

#[cfg(test)]
mod tests {
    #[cfg(feature = "slow_tests")]
    mod slow_tests {
        use crate::{get_rpc_client, token::token_account};
        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;

        #[tokio::test]
        pub async fn test_token_account() {
            let client = get_rpc_client().unwrap();
            let owner = Pubkey::from_str("AAf6DN1Wkh4TKvqxVX1xLfEKRtZNSZKwrHsr3NL2Wphm")
                .expect("failed to parse owner pubkey");
            let mint = spl_token::native_mint::id();
            let token_account = token_account(&client, &owner, mint).await.unwrap();
            assert_eq!(
                token_account.pubkey,
                "C4rpfuopbU2q8kmn9panVsi2NkXW2uQaubmFSx9XCi1H"
            )
        }
    }
}
