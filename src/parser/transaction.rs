use anchor_lang::{prelude::*, Event};
use anyhow::Result;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use tracing::{debug, info};

use super::{to_bytes, Encoding};

pub async fn parse(tx: EncodedConfirmedTransactionWithStatusMeta) {
    match tx.transaction.transaction {
        solana_transaction_status::EncodedTransaction::LegacyBinary(_) => {
            info!("EncodedTransaction::LegacyBinary");
        }
        solana_transaction_status::EncodedTransaction::Binary(_, _) => {
            info!("EncodedTransaction::Binary");
        }
        solana_transaction_status::EncodedTransaction::Json(uitx) => match uitx.message {
            solana_transaction_status::UiMessage::Parsed(parsed) => {
                debug!("parsed.account_keys: {:#?}", parsed.account_keys);
                debug!("parsed.instructions: {:#?}", parsed.instructions);

                let mut match_programs: Vec<(String, String)> = vec![];

                for ins in parsed.instructions {
                    match ins {
                        solana_transaction_status::UiInstruction::Compiled(_) => {
                            info!("UiInstruction::Compiled");
                        }
                        solana_transaction_status::UiInstruction::Parsed(ins_parsed) => {
                            match ins_parsed {
                                solana_transaction_status::UiParsedInstruction::Parsed(p) => {
                                    info!("UiParsedInstruction::Parsed, program_id: {}, data: {}", p.program_id, p.parsed);
                                },
                                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(
                                    pd,
                                ) => {
                                    // info!("{:?}", pd);
                                    info!("program_id: {}, data: {}", pd.program_id, pd.data);

                                    match_programs.push((pd.program_id, pd.data));
                                }
                            }
                        }
                    }
                }
                info!("match_programs: {:#?}", match_programs);
            }
            solana_transaction_status::UiMessage::Raw(_) => {
                info!("UiMessage::Raw");
            }
        },
        solana_transaction_status::EncodedTransaction::Accounts(_) => {
            info!("EncodedTransaction::Accounts");
        }
    }
}

pub fn parse_data<T: AnchorDeserialize>(data: &str) -> Result<T> {
    let bytes = to_bytes(data, Encoding::Base58)?;

    let parsed_data = T::deserialize(&mut bytes.as_slice())?;
    // let parsed_data = T::try_from_slice(&bytes)?;
    Ok(parsed_data)
}

pub fn parse_event<T: AnchorDeserialize + Event>(data: &str) -> Result<T> {
    let bytes = to_bytes(data, Encoding::Base58)?;
    // [228, 69, 165, 46, 81, 203, 154, 29]
    let _unknown = &bytes[0..8];
    let bytes = &bytes[8..];

    let mut ix_data: &[u8] = &bytes[..];
    let _disc: [u8; 8] = {
        let mut disc = [0; 8];
        disc.copy_from_slice(&bytes[..8]);
        ix_data = &ix_data[8..];
        disc
    };
    // println!("disc: {:?}", disc);

    let parsed_data = T::deserialize(&mut ix_data)?;
    Ok(parsed_data)
}
