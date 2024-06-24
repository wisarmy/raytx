use anyhow::Result;
use raytx::raydium::get_pool_info;
#[tokio::main]
async fn main() -> Result<()> {
    let pool_info = get_pool_info(
        "So11111111111111111111111111111111111111112",
        "6FVyLVhQsShWVUsCq2FJRr1MrECGShc3QxBwWtgiVFwK",
    )
    .await?;
    println!("pool info: {:#?}", pool_info);
    Ok(())
}
