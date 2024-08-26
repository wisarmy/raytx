// use raydium_amm::instruction::{AmmInstruction, SwapInstructionBaseIn};
use borsh::BorshDeserialize;

#[derive(BorshDeserialize, Debug, PartialEq)]
pub struct BuyInsData {
    pub discriminator: u8,
    // SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
    pub amount_in: u64,
    /// Minimum amount of DESTINATION token to output, prevents excessive slippage
    pub minimum_amount_out: u64,
}

#[cfg(test)]
mod tests {
    use crate::parser::transaction::parse_data;

    use super::*;

    #[test]
    fn test_parse() {
        let data = "6EnjbYZvXyHw125hDaYb1r7";
        let buy_ins_data = parse_data::<BuyInsData>(data).unwrap();
        assert_eq!(
            buy_ins_data,
            BuyInsData {
                discriminator: 9,
                amount_in: 517289849594780,
                minimum_amount_out: 974059
            }
        )
    }
}
