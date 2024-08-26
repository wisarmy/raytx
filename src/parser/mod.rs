use anyhow::Result;
use base64::Engine;
use tracing::warn;

pub mod jup;
pub mod pump;
pub mod raydium;
pub mod transaction;

pub enum Encoding {
    Base58,
    Base64,
}

pub fn to_hex(input: &str, encoding: Encoding) -> Result<String> {
    bytes_to_hex(to_bytes(input, encoding)?)
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

pub fn bytes_to_hex<T>(data: T) -> Result<String>
where
    T: AsRef<[u8]>,
{
    let hex_data = hex::encode(data);
    Ok(hex_data)
}
