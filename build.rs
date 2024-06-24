use std::env;
use std::process::Command;

fn main() {
    let project_root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let raydium_dir = format!("{}/raydium", project_root);

    // 执行 npm install
    let status = Command::new("npm")
        .arg("install")
        .current_dir(&raydium_dir)
        .status()
        .expect("Failed to execute npm install");

    if !status.success() {
        panic!("npm install failed with status: {}", status);
    }
}
