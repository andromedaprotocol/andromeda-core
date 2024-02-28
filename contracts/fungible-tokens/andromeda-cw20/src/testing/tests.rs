use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies_custom;
use andromeda_fungible_tokens::cw20::{ExecuteMsg, InstantiateMsg, QueryMsg};
// use andromeda_std::ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, PercentRate, Rate};
// use andromeda_std::ado_contract::ADOContract;
// use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;
// use cosmwasm_std::{coin, Decimal};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    to_json_binary, Addr, DepsMut, Response, Uint128,
};
use cw20::{Cw20Coin, Cw20ReceiveMsg};
use cw20_base::state::BALANCES;

use super::mock_querier::MOCK_CW20_CONTRACT;

fn init(deps: DepsMut) -> Response {
    let msg = InstantiateMsg {
        name: MOCK_CW20_CONTRACT.into(),
        symbol: "Symbol".into(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            amount: 1000u128.into(),
            address: "sender".to_string(),
        }],
        mint: None,
        marketing: None,

        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_andr_query() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let msg = QueryMsg::Owner {};
    let res = query(deps.as_ref(), mock_env(), msg);
    // Test that the query is hooked up correctly.
    assert!(res.is_ok())
}

#[test]
fn test_transfer() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut());
    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw20"),
        res
    );

    assert_eq!(
        Uint128::from(1000u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    let msg = ExecuteMsg::Transfer {
        recipient: "other".into(),
        amount: 100u128.into(),
    };

    // let not_whitelisted_info = mock_info("not_whitelisted", &[]);
    // let res = execute(deps.as_mut(), mock_env(), not_whitelisted_info, msg.clone());
    // assert_eq!(
    //     ContractError::Std(StdError::generic_err(
    //         "Querier contract error: InvalidAddress"
    //     )),
    //     res.unwrap_err()
    // );
    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            // .add_event(Event::new("Royalty"))
            // .add_event(Event::new("Tax"))
            .add_attribute("action", "transfer")
            .add_attribute("from", "sender")
            .add_attribute("to", "other")
            .add_attribute("amount", "100"),
        res
    );

    // Funds deducted from the sender (100 for send, 10 for tax).
    assert_eq!(
        Uint128::from(900u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    // Funds given to the receiver.
    assert_eq!(
        Uint128::from(100u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("other"))
            .unwrap()
    );

    // Royalty given to rates_recipient
    // assert_eq!(
    //     Uint128::from(0u128),
    //     BALANCES
    //         .load(deps.as_ref().storage, &Addr::unchecked("rates_recipient"))
    //         .unwrap()
    // );
}

#[test]
fn test_send() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    let res = init(deps.as_mut());

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw20"),
        res
    );

    assert_eq!(
        Uint128::from(1000u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    // let rate = Rate::Local(LocalRate {
    //     rate_type: LocalRateType::Additive,
    //     recipients: vec![Recipient {
    //         address: AndrAddr::from_string("sender".to_string()),
    //         msg: None,
    //         ibc_recovery_address: None,
    //     }],
    //     value: LocalRateValue::Flat(coin(10, "uusd")),
    //     description: None,
    // });

    // // Set rates
    // ADOContract::default()
    //     .set_rates(deps.as_mut().storage, "cw20", rate)
    //     .unwrap();

    let msg = ExecuteMsg::Send {
        contract: "contract".into(),
        amount: 100u128.into(),
        msg: to_json_binary(&"msg").unwrap(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "send")
            .add_attribute("from", "sender")
            .add_attribute("to", "contract")
            .add_attribute("amount", "100")
            .add_message(
                Cw20ReceiveMsg {
                    sender: "sender".into(),
                    amount: 100u128.into(),
                    msg: to_json_binary(&"msg").unwrap(),
                }
                .into_cosmos_msg("contract")
                .unwrap(),
            ),
        res
    );

    // Funds deducted from the sender (100 for send, 10 for tax).
    assert_eq!(
        Uint128::from(900u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    // Funds given to the receiver.
    assert_eq!(
        Uint128::from(100u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("contract"))
            .unwrap()
    );
}
