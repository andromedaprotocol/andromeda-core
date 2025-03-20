use andromeda_fungible_tokens::cw20_redeem::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};
use andromeda_std::{
    amp::AndrAddr, error::ContractError, testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    attr, coins,
    testing::{mock_env, mock_info},
    to_json_binary, DepsMut, Response, Uint128,
};
use cw20::Cw20ReceiveMsg;
pub const MOCK_TOKEN_ADDRESS: &str = "cw20";

use crate::{
    contract::{execute, instantiate},
    state::TOKEN_ADDRESS,
    testing::mock_querier::mock_dependencies_custom,
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

const OWNER: &str = "owner";
const USER: &str = "user";
const REDEEMED_TOKEN_CONTRACT: &str = "redeemed_token_contract";
const CW20_CONTRACT: &str = "cw20_contract";
const NATIVE_DENOM: &str = "uusd";

#[test]
fn test_native_token_redemption() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info(OWNER, &[]);

    // First instantiate the contract
    let msg = InstantiateMsg {
        token_address: AndrAddr::from_string(REDEEMED_TOKEN_CONTRACT),
        kernel_address: "kernel".to_string(),
        owner: Some(OWNER.to_string()),
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Start redemption clause with native token
    let exchange_rate = Uint128::from(2u128); // 2:1 exchange rate
    let amount = Uint128::from(1000u128);
    let info = mock_info(OWNER, &coins(amount.into(), NATIVE_DENOM));

    let msg = ExecuteMsg::SetRedemptionClause {
        exchange_rate,
        start_time: None,
        duration: None,
    };

    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Now try to redeem tokens
    let redeem_amount = Uint128::from(100u128);
    let info = mock_info(USER, &coins(100, NATIVE_DENOM));

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER.to_string(),
        amount: redeem_amount,
        msg: to_json_binary(&Cw20HookMsg::Redeem {}).unwrap(),
    });

    let response = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Verify the response contains the correct attributes
    assert_eq!(
        response.attributes,
        vec![
            attr("action", "redeem"),
            attr("purchaser", USER),
            attr("amount", (redeem_amount * exchange_rate).to_string()),
            attr("purchase_asset", format!("native:{}", NATIVE_DENOM)),
            attr("purchase_asset_amount_send", redeem_amount.to_string()),
        ]
    );
}

#[test]
fn test_cw20_token_redemption() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info(OWNER, &[]);

    // First instantiate the contract
    let msg = InstantiateMsg {
        token_address: AndrAddr::from_string(REDEEMED_TOKEN_CONTRACT),
        kernel_address: "kernel".to_string(),
        owner: Some(OWNER.to_string()),
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Start redemption clause with CW20 token
    let exchange_rate = Uint128::from(2u128); // 2:1 exchange rate
    let amount = Uint128::from(1000u128);

    let msg = Cw20ReceiveMsg {
        sender: OWNER.to_string(),
        amount,
        msg: to_json_binary(&Cw20HookMsg::StartRedemptionClause {
            exchange_rate,
            start_time: None,
            duration: None,
        })
        .unwrap(),
    };

    let info = mock_info(REDEEMED_TOKEN_CONTRACT, &[]);

    let msg = ExecuteMsg::Receive(msg);
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Now try to redeem tokens
    let redeem_amount = Uint128::from(100u128);
    let info = mock_info(REDEEMED_TOKEN_CONTRACT, &[]);

    let msg = Cw20ReceiveMsg {
        sender: USER.to_string(),
        amount: redeem_amount,
        msg: to_json_binary(&Cw20HookMsg::Redeem {}).unwrap(),
    };
    let msg = ExecuteMsg::Receive(msg);

    let response = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Verify the response contains the correct attributes
    assert_eq!(
        response.attributes,
        vec![
            attr("action", "redeem"),
            attr("purchaser", USER),
            attr("amount", (redeem_amount * exchange_rate).to_string()),
            attr("purchase_asset", format!("cw20:{}", CW20_CONTRACT)),
            attr("purchase_asset_amount_send", redeem_amount.to_string()),
        ]
    );
}
