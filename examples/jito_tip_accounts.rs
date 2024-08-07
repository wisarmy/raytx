use anyhow::Result;
use raytx::jito::api::{get_tip_accounts, TipAccountResult};
#[tokio::main]
async fn main() -> Result<()> {
    let accounts: TipAccountResult = get_tip_accounts().await?.try_into()?;
    println!("tip accounts: {:#?}", accounts);
    Ok(())
}
