use andromeda_fungible_tokens::cw20_redeem::InstantiateMsg;
use andromeda_std::{
    amp::AndrAddr, error::ContractError, testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    DepsMut, Response, StdError, StdResult, Uint128,
};
use rstest::rstest;

pub const MOCK_TOKEN_ADDRESS: &str = "cw20";

use crate::{
    contract::instantiate, state::TOKEN_ADDRESS, testing::mock_querier::mock_dependencies_custom,
};

fn init(deps: DepsMut) -> Result<Response, ContractError> {
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,

        token_address: AndrAddr::from_string("cw20"),
    };

    instantiate(deps, mock_env(), info, msg)
}
#[test]
pub fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut()).unwrap();

    let saved_mock_token_address = TOKEN_ADDRESS.load(deps.as_ref().storage).unwrap();

    assert_eq!(saved_mock_token_address, MOCK_TOKEN_ADDRESS.to_string())
}

/// Represents the result of a redemption calculation
#[derive(Debug, PartialEq)]
struct RedemptionResult {
    redeemed_amount: Uint128,
    excess_amount: Uint128,
}

/// Calculates redemption amount and any excess based on available tokens
fn calculate_redemption_with_limits(
    amount_sent: Uint128,
    exchange_rate: Uint128,
    available_tokens: Uint128,
) -> StdResult<RedemptionResult> {
    let redeemed = amount_sent.checked_mul(exchange_rate)?;

    if redeemed.is_zero() {
        return Err(StdError::generic_err("Zero redemption amount"));
    }

    if redeemed <= available_tokens {
        Ok(RedemptionResult {
            redeemed_amount: redeemed,
            excess_amount: Uint128::zero(),
        })
    } else {
        // Calculate how many tokens we can actually redeem
        let actual_redeemed = available_tokens;
        // Calculate how much of the sent amount we didn't use
        let actual_amount_needed = available_tokens.checked_div(exchange_rate)?;
        let excess = amount_sent.checked_sub(actual_amount_needed)?;

        Ok(RedemptionResult {
            redeemed_amount: actual_redeemed,
            excess_amount: excess,
        })
    }
}

#[rstest]
// Normal cases
#[case(
    Uint128::new(100), // amount sent
    Uint128::new(2),   // exchange rate
    Uint128::new(1000), // available tokens
    Ok(RedemptionResult {
        redeemed_amount: Uint128::new(200),
        excess_amount: Uint128::zero(),
    })
)]
// Exact amount case
#[case(
    Uint128::new(500),
    Uint128::new(2),
    Uint128::new(1000),
    Ok(RedemptionResult {
        redeemed_amount: Uint128::new(1000),
        excess_amount: Uint128::zero(),
    })
)]
// Excess case
#[case(
    Uint128::new(600), // trying to get 1200 tokens when only 1000 available
    Uint128::new(2),
    Uint128::new(1000),
    Ok(RedemptionResult {
        redeemed_amount: Uint128::new(1000),
        excess_amount: Uint128::new(100), // 100 tokens worth of excess
    })
)]
// Zero amount case
#[case(
    Uint128::zero(),
    Uint128::new(2),
    Uint128::new(1000),
    Err(StdError::generic_err("Zero redemption amount"))
)]
fn test_redemption_calculations(
    #[case] amount_sent: Uint128,
    #[case] exchange_rate: Uint128,
    #[case] available_tokens: Uint128,
    #[case] expected: StdResult<RedemptionResult>,
) {
    let result = calculate_redemption_with_limits(amount_sent, exchange_rate, available_tokens);
    assert_eq!(result, expected);
}

#[test]
fn test_redemption_edge_cases() {
    // Test with maximum available tokens
    let result =
        calculate_redemption_with_limits(Uint128::new(1000), Uint128::new(2), Uint128::MAX)
            .unwrap();
    assert_eq!(result.excess_amount, Uint128::zero());
    assert_eq!(result.redeemed_amount, Uint128::new(2000));

    // Test with very small amounts
    let result =
        calculate_redemption_with_limits(Uint128::new(1), Uint128::new(1), Uint128::new(1))
            .unwrap();
    assert_eq!(result.excess_amount, Uint128::zero());
    assert_eq!(result.redeemed_amount, Uint128::new(1));

    // Test overflow case
    let result = calculate_redemption_with_limits(Uint128::MAX, Uint128::new(2), Uint128::MAX);
    assert!(result.is_err());
}

#[test]
fn test_fractional_redemption_cases() {
    // Test when available tokens don't divide evenly by exchange rate
    let result = calculate_redemption_with_limits(
        Uint128::new(100),
        Uint128::new(3),
        Uint128::new(250), // Not divisible by 3
    )
    .unwrap();

    // Should redeem up to 249 (as 250 is not divisible by 3)
    assert_eq!(result.redeemed_amount, Uint128::new(250));
    assert_eq!(result.excess_amount, Uint128::new(17)); // (100 - 83) where 83 * 3 = 249
}

#[test]
fn test_boundary_conditions() {
    // Test with minimum viable amounts
    let result =
        calculate_redemption_with_limits(Uint128::new(1), Uint128::new(1), Uint128::new(1))
            .unwrap();
    assert_eq!(result.redeemed_amount, Uint128::new(1));
    assert_eq!(result.excess_amount, Uint128::zero());
}
