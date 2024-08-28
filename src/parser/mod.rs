use anyhow::Result;
use base64::Engine;
use tracing::warn;

pub mod discriminator;
pub mod jup;
pub mod pump;
pub mod raydium;
pub mod transaction;

pub enum Encoding {
    Base58,
    Base64,
}

pub fn to_hex(input: &str, encoding: Encoding) -> Result<String> {
    Ok(hex::encode(to_bytes(input, encoding)?))
}

pub fn to_bytes(input: &str, encoding: Encoding) -> Result<Vec<u8>> {
    let raw_bytes = match encoding {
        Encoding::Base58 => bs58::decode(input).into_vec().inspect_err(|err| {
            warn!("base58 failed to decode: {}", err);
        })?,
        Encoding::Base64 => base64::prelude::BASE64_STANDARD
            .decode(input)
            .inspect_err(|err| {
                warn!("base64 failed to decode: {}", err);
            })?,
    };
    Ok(raw_bytes)
}

pub fn sighash(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{}:{}", namespace, name);

    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(
        &anchor_lang::solana_program::hash::hash(preimage.as_bytes()).to_bytes()[..8],
    );
    sighash
}
