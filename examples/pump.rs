use std::str::FromStr;

use anyhow::Result;
use raytx::{
    get_rpc_client_blocking,
    pump::{get_bonding_curve_account, get_pda, PUMP_PROGRAM},
};
use solana_sdk::pubkey::Pubkey;
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // let pump_info = get_pump_info("8zSLdDzM1XsqnfrHmHvA9ir6pvYDjs8UXz6B2Tydd6b2").await?;
    // println!("pump info: {:#?}", pump_info);
    get_bonding_curve_by_mint().await?;

    Ok(())
}

pub async fn get_bonding_curve_by_mint() -> Result<()> {
    let client = get_rpc_client_blocking()?;
    let program_id = Pubkey::from_str(PUMP_PROGRAM)?;
    let mint = Pubkey::from_str("8oAK7mKMSnsVgrBgFS6A4uPqL8dh5NHAc7ohsq71pump")?;
    let bonding_curve = get_pda(&mint, &program_id)?;
    println!("bonding_curve: {bonding_curve}");

    let bonding_curve_account =
        get_bonding_curve_account(client, &mint, &Pubkey::from_str(PUMP_PROGRAM)?).await;

    println!("bonding_curve_account: {:#?}", bonding_curve_account);
    Ok(())
}
