use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_account_decoder::UiAccountData;
use solana_client::{rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::pubkey::Pubkey;
use tracing::trace;

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

pub fn token_account(client: &RpcClient, owner: &Pubkey, mint: Pubkey) -> Result<TokenAccount> {
    let token_accounts = token_accounts_filter(client, owner, TokenAccountsFilter::Mint(mint))?;
    token_accounts
        .first()
        .cloned()
        .ok_or(anyhow!("no token account found"))
}

pub fn token_accounts(client: &RpcClient, owner: &Pubkey) -> Result<TokenAccounts> {
    token_accounts_filter(
        client,
        owner,
        TokenAccountsFilter::ProgramId(spl_token::id()),
    )
}
fn token_accounts_filter(
    client: &RpcClient,
    owner: &Pubkey,
    filter: TokenAccountsFilter,
) -> Result<TokenAccounts> {
    let token_accounts = client
        .get_token_accounts_by_owner(owner, filter)
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

#[cfg(test)]
mod tests {
    use crate::{get_rpc_client, token::token_account};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    #[test]
    pub fn test_token_account() {
        let client = get_rpc_client().unwrap();
        let owner = Pubkey::from_str("AAf6DN1Wkh4TKvqxVX1xLfEKRtZNSZKwrHsr3NL2Wphm")
            .expect("failed to parse owner pubkey");
        let mint = spl_token::native_mint::id();
        let token_account = token_account(&client, &owner, mint).unwrap();
        assert_eq!(
            token_account.pubkey,
            "C4rpfuopbU2q8kmn9panVsi2NkXW2uQaubmFSx9XCi1H"
        )
    }
}
