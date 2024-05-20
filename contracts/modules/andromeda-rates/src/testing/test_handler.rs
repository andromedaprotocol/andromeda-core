use andromeda_std::{
    ado_base::rates::{calculate_fee, LocalRateValue, PercentRate},
    error::ContractError,
};
use cosmwasm_std::{coin, Coin, Decimal};

struct TestHandleLocalCase {
    name: &'static str,
    fee_rate: LocalRateValue,
    payment: Coin,
    expected_result: Coin,
    expected_error: Option<ContractError>,
}

#[test]
fn test_handle_local() {
    let test_cases = vec![
        TestHandleLocalCase {
            name: "Payment is greater than flat rate",
            fee_rate: LocalRateValue::Flat(coin(1, "uandr")),
            payment: coin(20, "uandr"),
            expected_result: coin(1, "uandr"),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Payment is less than flat rate",
            fee_rate: LocalRateValue::Flat(coin(100, "uandr")),
            payment: coin(20, "uandr"),
            expected_result: coin(1, "uandr"),
            expected_error: Some(ContractError::InsufficientFunds {}),
        },
        TestHandleLocalCase {
            name: "Payment is equal to flat rate",
            fee_rate: LocalRateValue::Flat(coin(100, "uandr")),
            payment: coin(100, "uandr"),
            expected_result: coin(100, "uandr"),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Percent rate without remainder",
            fee_rate: LocalRateValue::Percent(PercentRate {
                percent: Decimal::percent(4),
            }),
            payment: coin(100, "uandr"),
            expected_result: coin(4, "uandr"),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Percent rate with small remainder",
            fee_rate: LocalRateValue::Percent(PercentRate {
                percent: Decimal::percent(4),
            }),
            payment: coin(101, "uandr"),
            // If there's a remainder (it's 0.04) it rounds up
            expected_result: coin(5, "uandr"),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Percent rate with large remainder",
            fee_rate: LocalRateValue::Percent(PercentRate {
                percent: Decimal::percent(7),
            }),
            payment: coin(114, "uandr"),
            // 7.98, should return 8
            expected_result: coin(8, "uandr"),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Payment of 1 coin",
            fee_rate: LocalRateValue::Percent(PercentRate {
                percent: Decimal::percent(1),
            }),
            payment: coin(1, "uandr"),
            // The fee takes up the entire payment
            expected_result: coin(1, "uandr"),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "0 percent rate",
            fee_rate: LocalRateValue::Percent(PercentRate {
                percent: Decimal::percent(0),
            }),
            payment: coin(101, "uandr"),
            expected_result: coin(5, "uandr"),
            expected_error: Some(ContractError::InvalidRate {}),
        },
        TestHandleLocalCase {
            name: "101 percent rate",
            fee_rate: LocalRateValue::Percent(PercentRate {
                percent: Decimal::percent(101),
            }),
            payment: coin(101, "uandr"),
            expected_result: coin(5, "uandr"),
            expected_error: Some(ContractError::InvalidRate {}),
        },
    ];

    for test in test_cases {
        let res = calculate_fee(test.fee_rate, &test.payment);
        if let Some(err) = test.expected_error {
            assert_eq!(res.unwrap_err(), err, "{}", test.name);
            continue;
        }

        let response = res.unwrap();

        assert_eq!(response, test.expected_result, "{}", test.name);
    }
}
