use anyhow::Result;
use raytx::pump::get_pump_info;
#[tokio::main]
async fn main() -> Result<()> {
    let pump_info = get_pump_info("8zSLdDzM1XsqnfrHmHvA9ir6pvYDjs8UXz6B2Tydd6b2").await?;
    println!("pump info: {:#?}", pump_info);
    Ok(())
}
