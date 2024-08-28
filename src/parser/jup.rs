use anchor_lang::prelude::*;

#[derive(AnchorDeserialize, Debug, PartialEq)]
pub struct Route {
    pub route_plan: Vec<RoutePlanStep>,
    pub in_amount: u64,
    pub quoted_out_amount: u64,
    pub slippage_bps: u16,
    pub platform_fee_bps: u8,
}

#[derive(AnchorDeserialize, Debug, PartialEq)]
pub struct RoutePlanStep {
    pub swap: Swap,
    pub percent: u8,
    pub input_index: u8,
    pub output_index: u8,
}

#[derive(AnchorDeserialize, Copy, Clone, Debug, PartialEq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(AnchorDeserialize, Clone, PartialEq, Debug)]
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

#[derive(AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
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

#[derive(AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct RemainingAccountsSlice {
    pub accounts_type: AccountsType,
    pub length: u8,
}

#[derive(AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct RemainingAccountsInfo {
    pub slices: Vec<RemainingAccountsSlice>,
}

#[derive(Clone, Debug, PartialEq)]
#[event]
pub struct SwapEvent {
    // pub unknown: [u8; 8],
    // pub discriminator: [u8; 8],
    pub amm: Pubkey,
    pub input_mint: Pubkey,
    pub input_amount: u64,
    pub output_mint: Pubkey,
    pub output_amount: u64,
}
#[derive(Clone, Debug, PartialEq)]
#[event]
pub struct FeeEvent {
    pub account: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use tracing::debug;

    use crate::parser::transaction::parse_data;

    use super::*;

    #[test]
    fn test_parse_route() {
        // Signature 2aBMaQvN8StxvedeExc8sGWfBUTC1co9ZnUXE7UE5UDboFWbrxrXvV7bXeWRSY6hhpU1RuNSVTHVFmoE6sfdcDDd
        let data = "PrpFmsY4d26dKbdKMAXs4neVE2yj1DtChrbeqP7wzojhcyUY";
        // let hex = crate::parser::to_hex(data, crate::parser::Encoding::Base58).unwrap();
        // tracing::info!("hex: {}", hex);
        let bytes = crate::parser::to_bytes(data, crate::parser::Encoding::Base58).unwrap();
        println!("bytes: {:?}", bytes.len());

        let ins_route = parse_data::<Route>(data).unwrap();
        assert_eq!(
            ins_route,
            Route {
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
    #[test]
    fn test_parse_swap_event() {
        // Signature 2aBMaQvN8StxvedeExc8sGWfBUTC1co9ZnUXE7UE5UDboFWbrxrXvV7bXeWRSY6hhpU1RuNSVTHVFmoE6sfdcDDd
        let data = "QMqFu4fYGGeUEysFnenhAvR83g86EDDNxzUskfkWKYCBPWe1hqgD6jgKAXr6aYoEQaxoqYMTvWgPVk2AHWGHjdbNiNtoaPfZA4znu6cRUSWSeJGEtRzSATxShVULX7AV7pkjEGEJC238f26YypQrekApRHctXJgbPUffrWstS1Qn9Ry";
        // let hex = crate::parser::to_hex(data, crate::parser::Encoding::Base58).unwrap();
        // tracing::info!("hex: {}", hex);
        // let bytes = crate::parser::to_bytes(data, crate::parser::Encoding::Base58).unwrap();
        // println!("bytes: {:?}", bytes);

        let swap_event = parse_data::<SwapEvent>(data).unwrap();
        debug!("swap_event: {:?}", swap_event);
        assert_eq!(
            swap_event,
            SwapEvent {
                // name: event_name,
                amm: Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap(),
                input_mint: Pubkey::from_str("So11111111111111111111111111111111111111112")
                    .unwrap(),
                input_amount: 100000000000,
                output_mint: Pubkey::from_str("26KMQVgDUoB6rEfnJ51yAABWWJND8uMtpnQgsHQ64Udr")
                    .unwrap(),
                output_amount: 1448740533191,
            }
        )
    }
    #[test]
    fn test_parse_fee_event() {
        // Signature 2aBMaQvN8StxvedeExc8sGWfBUTC1co9ZnUXE7UE5UDboFWbrxrXvV7bXeWRSY6hhpU1RuNSVTHVFmoE6sfdcDDd
        let data = "2qWhKzSZDTHhTkHUC1NYnTg1JTQB8w3LyKHBewbcWr6F74fYXwS2M6TgRGCBjD7Rsufm6sQzW62bEyqXQpxo9Rr4JYvWVPVyzFgxeF4DwmimGfDhkohmzDqZH";
        // let hex = crate::parser::to_hex(data, crate::parser::Encoding::Base58).unwrap();
        // tracing::info!("hex: {}", hex);
        // let bytes = crate::parser::to_bytes(data, crate::parser::Encoding::Base58).unwrap();
        // println!("bytes: {:?}", bytes);

        let fee_event = parse_data::<FeeEvent>(data).unwrap();
        // let mut event_name = [0u8; 8];
        // event_name.copy_from_slice("SwapEvent".as_bytes());

        debug!("fee_event: {:?}", fee_event);
        assert_eq!(
            fee_event,
            FeeEvent {
                account: Pubkey::from_str("KViqiev9hum4PsvQmYkNcqfHBjGQiuDqpv88RpAinh9").unwrap(),
                mint: Pubkey::from_str("26KMQVgDUoB6rEfnJ51yAABWWJND8uMtpnQgsHQ64Udr").unwrap(),
                amount: 12314294532,
            }
        )
    }
}
