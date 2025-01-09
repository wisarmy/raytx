use anyhow::Result;
use raytx::jito::{ws::tip_stream, TIPS_PERCENTILE};
#[tokio::main]
async fn main() -> Result<()> {
    tokio::spawn(async {
        if let Err(e) = tip_stream().await {
            println!("Error: {:?}", e);
        }
    });

    loop {
        {
            let state = TIPS_PERCENTILE.read().await;
            if let Some(ref msg) = *state {
                println!("Latest message: {:?}", msg);
            } else {
                println!("No message received yet");
            }
        }
        println!("Waiting next after 5s");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
