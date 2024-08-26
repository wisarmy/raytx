use anyhow::Result;
use borsh::BorshDeserialize;
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
                                    info!("UiParsedInstruction::Parsed, program_id: {}", p.program_id);
                                },
                                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(
                                    pd,
                                ) => {
                                    info!("{:?}", pd);
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

pub fn parse_data<T: BorshDeserialize>(data: &str) -> Result<T> {
    let bytes = to_bytes(data, Encoding::Base58)?;
    let parsed_data = T::try_from_slice(&bytes)?;
    Ok(parsed_data)
}
