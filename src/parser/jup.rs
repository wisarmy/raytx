use borsh::BorshDeserialize;

#[derive(BorshDeserialize, Debug, PartialEq)]
pub struct BuyInsData {
    pub id: u8,
    pub route_plan: Vec<RoutePlanStep>,
    pub in_amount: u64,
    pub quoted_out_amount: u64,
    pub slippage_bps: u16,
    pub platform_fee_bps: u8,
}

#[derive(BorshDeserialize, Debug, PartialEq)]
pub struct RoutePlanStep {
    pub swap: Swap,
    pub percent: u8,
    pub input_index: u8,
    pub output_index: u8,
}

#[derive(BorshDeserialize, Copy, Clone, Debug, PartialEq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(BorshDeserialize, Clone, PartialEq, Debug)]
pub enum Swap {
    Saber,
    SaberAddDecimalsDeposit,
    SaberAddDecimalsWithdraw,
    TokenSwap,
    Sencha,
    Step,
    Cropper,
    Raydium,
    Crema {
        a_to_b: bool,
    },
    Lifinity,
    Mercurial,
    Cykura,
    Serum {
        side: Side,
    },
    MarinadeDeposit,
    MarinadeUnstake,
    Aldrin {
        side: Side,
    },
    AldrinV2 {
        side: Side,
    },
    Whirlpool {
        a_to_b: bool,
    },
    Invariant {
        x_to_y: bool,
    },
    Meteora,
    GooseFX,
    DeltaFi {
        stable: bool,
    },
    Balansol,
    MarcoPolo {
        x_to_y: bool,
    },
    Dradex {
        side: Side,
    },
    LifinityV2,
    RaydiumClmm,
    Openbook {
        side: Side,
    },
    Phoenix {
        side: Side,
    },
    Symmetry {
        from_token_id: u64,
        to_token_id: u64,
    },
    TokenSwapV2,
    HeliumTreasuryManagementRedeemV0,
    StakeDexStakeWrappedSol,
    StakeDexSwapViaStake {
        bridge_stake_seed: u32,
    },
    GooseFXV2,
    Perps,
    PerpsAddLiquidity,
    PerpsRemoveLiquidity,
    MeteoraDlmm,
    OpenBookV2 {
        side: Side,
    },
    RaydiumClmmV2,
    StakeDexPrefundWithdrawStakeAndDepositStake {
        bridge_stake_seed: u32,
    },
    Clone {
        pool_index: u8,
        quantity_is_input: bool,
        quantity_is_collateral: bool,
    },
    SanctumS {
        src_lst_value_calc_accs: u8,
        dst_lst_value_calc_accs: u8,
        src_lst_index: u32,
        dst_lst_index: u32,
    },
    SanctumSAddLiquidity {
        lst_value_calc_accs: u8,
        lst_index: u32,
    },
    SanctumSRemoveLiquidity {
        lst_value_calc_accs: u8,
        lst_index: u32,
    },
    RaydiumCP,
    WhirlpoolSwapV2 {
        a_to_b: bool,
        remaining_accounts_info: Option<RemainingAccountsInfo>,
    },
    OneIntro,
    Obric {
        x_to_y: bool,
    },
}

#[derive(BorshDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum AccountsType {
    TransferHookA,
    TransferHookB,
    // TransferHookReward,
    // TransferHookInput,
    // TransferHookIntermediate,
    // TransferHookOutput,
    //TickArray,
    //TickArrayOne,
    //TickArrayTwo,
}

#[derive(BorshDeserialize, Clone, Debug, PartialEq)]
pub struct RemainingAccountsSlice {
    pub accounts_type: AccountsType,
    pub length: u8,
}

#[derive(BorshDeserialize, Clone, Debug, PartialEq)]
pub struct RemainingAccountsInfo {
    pub slices: Vec<RemainingAccountsSlice>,
}

#[cfg(test)]
mod tests {
    use crate::parser::transaction::parse_data;

    use super::*;

    #[test]
    fn test_parse() {
        // Signature 2aBMaQvN8StxvedeExc8sGWfBUTC1co9ZnUXE7UE5UDboFWbrxrXvV7bXeWRSY6hhpU1RuNSVTHVFmoE6sfdcDDd
        let data = "PrpFmsY4d26dKbdKMAXs4neVE2yj1DtChrbeqP7wzojhcyUY";
        // let hex = crate::parser::to_hex(data, crate::parser::Encoding::Base58).unwrap();
        // tracing::info!("hex: {}", hex);
        let buy_ins_data = parse_data::<BuyInsData>(data).unwrap();
        assert_eq!(
            buy_ins_data,
            BuyInsData {
                id: 1,
                route_plan: vec![RoutePlanStep {
                    swap: Swap::Raydium,
                    percent: 100,
                    input_index: 0,
                    output_index: 1
                }],
                in_amount: 100000000000,
                quoted_out_amount: 1440710470524,
                slippage_bps: 30,
                platform_fee_bps: 85,
            }
        )
    }
}
