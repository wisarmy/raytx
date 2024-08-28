use anchor_lang::prelude::*;

#[derive(AnchorDeserialize, Debug, PartialEq)]
pub struct BuyInsData {
    pub method: u64,
    pub amount: u64,
    pub max_sol_cost: u64,
}

#[cfg(test)]
mod tests {
    use crate::parser::transaction::parse_data;

    use super::*;

    #[test]
    fn test_parse() {
        let data = "AJTQ2h9DXrC1wh2xrky6BKsQFMZ7nYhJX";
        let buy_ins_data = parse_data::<BuyInsData>(data).unwrap();
        assert_eq!(
            buy_ins_data,
            BuyInsData {
                method: 16927863322537952870,
                amount: 3180982806712,
                max_sol_cost: 1262500000
            }
        )
    }
}
