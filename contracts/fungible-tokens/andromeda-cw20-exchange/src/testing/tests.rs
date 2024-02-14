use andromeda_fungible_tokens::cw20_exchange::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, Sale, SaleAssetsResponse, SaleResponse,
    TokenAddressResponse,
};
use andromeda_std::{
    amp::AndrAddr, common::expiration::MILLISECONDS_TO_NANOSECONDS_RATIO, error::ContractError,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    attr, coin, coins, from_json,
    testing::{mock_env, mock_info},
    to_json_binary, wasm_execute, Addr, BankMsg, CosmosMsg, DepsMut, Empty, Response, SubMsg,
    Timestamp, Uint128,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::AssetInfo;
use cw_utils::Expiration;
pub const MOCK_TOKEN_ADDRESS: &str = "cw20";

use crate::{
    contract::{execute, instantiate, query},
    state::{SALE, TOKEN_ADDRESS},
    testing::mock_querier::mock_dependencies_custom,
};

fn init(deps: DepsMut) -> Result<Response, ContractError> {
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        modules: None,
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

#[test]
pub fn test_start_sale_invalid_token() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);
    let owner = Addr::unchecked("owner");
    let info = mock_info(owner.as_str(), &[]);
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate: Uint128::from(10u128),
        recipient: None,
        start_time: None,
        duration: None,
    };
    // Owner set as Cw20ReceiveMsg sender to ensure that this message will error even if a malicious user
    // sends the message directly with the owner address provided
    let receive_msg = Cw20ReceiveMsg {
        sender: owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: Uint128::from(100u128),
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Incorrect CW20 provided for sale".to_string()
        }
    )
}

#[test]
pub fn test_start_sale_unauthorised() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);
    let owner = Addr::unchecked("owner");
    let info = mock_info(owner.as_str(), &[]);
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate: Uint128::from(10u128),
        recipient: None,
        start_time: None,
        duration: None,
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: "not_owner".to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: Uint128::from(100u128),
    };
    let msg = ExecuteMsg::Receive(receive_msg);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {})
}

#[test]
pub fn test_start_sale_zero_amount() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let info = mock_info(owner.as_str(), &[]);
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate: Uint128::from(10u128),
        recipient: None,
        start_time: None,
        duration: None,
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: "not_owner".to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: Uint128::zero(),
    };
    let msg = ExecuteMsg::Receive(receive_msg);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Cannot send a 0 amount".to_string()
        }
    )
}

#[test]
pub fn test_start_sale() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));
    //     let info = mock_info(owner.as_str(), &[]);
    let token_info = mock_info(MOCK_TOKEN_ADDRESS, &[]);

    init(deps.as_mut()).unwrap();
    let current_time = env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset.clone(),
        exchange_rate,
        recipient: None,
        // A start time ahead of the current time
        start_time: Some(current_time + 10),
        duration: Some(1),
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: sale_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    execute(deps.as_mut(), env, token_info, msg).unwrap();

    let sale = SALE
        .load(deps.as_ref().storage, &exchange_asset.to_string())
        .unwrap();

    assert_eq!(sale.exchange_rate, exchange_rate);
    assert_eq!(sale.amount, sale_amount);

    let expected_start_time =
        Timestamp::from_nanos((current_time + 10) * MILLISECONDS_TO_NANOSECONDS_RATIO);
    assert_eq!(sale.start_time, Expiration::AtTime(expected_start_time));

    let expected_expiration_time =
        Timestamp::from_nanos((current_time + 11) * MILLISECONDS_TO_NANOSECONDS_RATIO);
    assert_eq!(sale.end_time, Expiration::AtTime(expected_expiration_time));
}

#[test]
pub fn test_start_sale_no_start_no_duration() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));
    //     let info = mock_info(owner.as_str(), &[]);
    let token_info = mock_info(MOCK_TOKEN_ADDRESS, &[]);

    init(deps.as_mut()).unwrap();
    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset.clone(),
        exchange_rate,
        recipient: None,
        // A start time ahead of the current time
        start_time: None,
        duration: None,
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: sale_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    execute(deps.as_mut(), env, token_info, msg).unwrap();

    let sale = SALE
        .load(deps.as_ref().storage, &exchange_asset.to_string())
        .unwrap();

    assert_eq!(sale.exchange_rate, exchange_rate);
    assert_eq!(sale.amount, sale_amount);

    assert_eq!(
        sale.start_time,
        // Current time + 1
        Expiration::AtTime(Timestamp::from_nanos(1571797419880000000))
    );

    assert_eq!(sale.end_time, Expiration::Never {});
}

#[test]
pub fn test_start_sale_invalid_start_time() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));
    let token_info = mock_info(MOCK_TOKEN_ADDRESS, &[]);

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate,
        recipient: None,
        start_time: Some(1),
        duration: None,
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: sale_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, token_info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::StartTimeInThePast {
            current_time: 1571797419879,
            current_block: 12345
        }
    );
}

#[test]
pub fn test_start_sale_ongoing() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));
    //     let info = mock_info(owner.as_str(), &[]);
    let token_info = mock_info(MOCK_TOKEN_ADDRESS, &[]);

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate,
        recipient: None,
        start_time: None,
        duration: None,
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: sale_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    execute(deps.as_mut(), env.clone(), token_info.clone(), msg.clone()).unwrap();

    let err = execute(deps.as_mut(), env, token_info, msg).unwrap_err();

    assert_eq!(err, ContractError::SaleNotEnded {})
}

#[test]
pub fn test_start_sale_zero_exchange_rate() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));
    let token_info = mock_info(MOCK_TOKEN_ADDRESS, &[]);

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::zero();
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate,
        recipient: None,
        start_time: None,
        duration: None,
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: sale_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, token_info, msg).unwrap_err();

    assert_eq!(err, ContractError::InvalidZeroAmount {})
}

#[test]
pub fn test_purchase_no_sale() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);
    let purchaser = Addr::unchecked("purchaser");
    //     let info = mock_info(owner.as_str(), &[]);
    let token_info = mock_info("invalid_token", &[]);

    init(deps.as_mut()).unwrap();

    // Purchase Tokens
    let purchase_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::Purchase { recipient: None };
    let receive_msg = Cw20ReceiveMsg {
        sender: purchaser.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: purchase_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, token_info, msg).unwrap_err();

    assert_eq!(err, ContractError::NoOngoingSale {});
}

#[test]
pub fn test_purchase_not_enough_sent() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let purchaser = Addr::unchecked("purchaser");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: Uint128::from(100u128),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),
            end_time: Expiration::Never {},
            start_amount: Uint128::from(100u128),
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = mock_info("exchanged_asset", &[]);
    let purchase_amount = Uint128::from(1u128);
    let hook = Cw20HookMsg::Purchase { recipient: None };
    let receive_msg = Cw20ReceiveMsg {
        sender: purchaser.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: purchase_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, exchange_info, msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to purchase a token".to_string()
        }
    );
}

#[test]
pub fn test_purchase_no_tokens_left() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let purchaser = Addr::unchecked("purchaser");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: Uint128::zero(),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),
            end_time: Expiration::Never {},
            start_amount: Uint128::zero(),
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = mock_info("exchanged_asset", &[]);
    let purchase_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::Purchase { recipient: None };
    let receive_msg = Cw20ReceiveMsg {
        sender: purchaser.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: purchase_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, exchange_info, msg).unwrap_err();

    assert_eq!(err, ContractError::NotEnoughTokens {});
}

#[test]
pub fn test_purchase_not_enough_tokens() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let purchaser = Addr::unchecked("purchaser");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: Uint128::one(),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: Uint128::one(),
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = mock_info("exchanged_asset", &[]);
    let purchase_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::Purchase { recipient: None };
    let receive_msg = Cw20ReceiveMsg {
        sender: purchaser.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: purchase_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, exchange_info, msg).unwrap_err();

    assert_eq!(err, ContractError::NotEnoughTokens {});
}

#[test]
pub fn test_purchase() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let purchaser = Addr::unchecked("purchaser");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: sale_amount,
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: sale_amount,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = mock_info("exchanged_asset", &[]);
    let purchase_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::Purchase { recipient: None };
    let receive_msg = Cw20ReceiveMsg {
        sender: purchaser.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: purchase_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let res = execute(deps.as_mut(), env, exchange_info, msg).unwrap();

    // Check transfer
    let msg = res.messages.first().unwrap();
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            MOCK_TOKEN_ADDRESS.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: purchaser.to_string(),
                amount: Uint128::from(10u128),
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::reply_on_error(expected_wasm, 2);
    assert_eq!(msg, &expected);

    // Check sale amount updated
    let sale = SALE
        .load(deps.as_mut().storage, &exchange_asset.to_string())
        .unwrap();

    assert_eq!(
        sale.amount,
        sale_amount.checked_sub(Uint128::from(10u128)).unwrap()
    );

    // Check recipient received funds
    let msg = &res.messages[1];
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            "exchanged_asset".to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: purchase_amount,
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::reply_on_error(expected_wasm, 3);

    assert_eq!(msg, &expected);
}

#[test]
pub fn test_purchase_with_start_and_duration() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let purchaser = Addr::unchecked("purchaser");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: sale_amount,
            exchange_rate,
            recipient: owner.to_string(),
            // start time in the past
            start_time: Expiration::AtTime(env.block.time.minus_nanos(1)),
            // end time in the future
            end_time: Expiration::AtTime(env.block.time.plus_nanos(1)),
            start_amount: sale_amount,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = mock_info("exchanged_asset", &[]);
    let purchase_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::Purchase { recipient: None };
    let receive_msg = Cw20ReceiveMsg {
        sender: purchaser.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: purchase_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let res = execute(deps.as_mut(), env, exchange_info, msg).unwrap();

    // Check transfer
    let msg = res.messages.first().unwrap();
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            MOCK_TOKEN_ADDRESS.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: purchaser.to_string(),
                amount: Uint128::from(10u128),
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::reply_on_error(expected_wasm, 2);
    assert_eq!(msg, &expected);

    // Check sale amount updated
    let sale = SALE
        .load(deps.as_mut().storage, &exchange_asset.to_string())
        .unwrap();

    assert_eq!(
        sale.amount,
        sale_amount.checked_sub(Uint128::from(10u128)).unwrap()
    );

    // Check recipient received funds
    let msg = &res.messages[1];
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            "exchanged_asset".to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: purchase_amount,
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::reply_on_error(expected_wasm, 3);

    assert_eq!(msg, &expected);
}

#[test]
pub fn test_purchase_sale_not_started() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let purchaser = Addr::unchecked("purchaser");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: sale_amount,
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time.plus_nanos(1)),
            end_time: Expiration::Never {},
            start_amount: sale_amount,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = mock_info("exchanged_asset", &[]);
    let purchase_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::Purchase { recipient: None };
    let receive_msg = Cw20ReceiveMsg {
        sender: purchaser.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: purchase_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, exchange_info, msg).unwrap_err();
    assert_eq!(err, ContractError::SaleNotStarted {})
}

#[test]
pub fn test_purchase_sale_duration_ended() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let purchaser = Addr::unchecked("purchaser");
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: sale_amount,
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::AtTime(env.block.time.minus_nanos(1)),
            start_amount: sale_amount,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = mock_info("exchanged_asset", &[]);
    let purchase_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::Purchase { recipient: None };
    let receive_msg = Cw20ReceiveMsg {
        sender: purchaser.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: purchase_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let err = execute(deps.as_mut(), env, exchange_info, msg).unwrap_err();
    assert_eq!(err, ContractError::SaleEnded {})
}

#[test]
pub fn test_purchase_no_sale_native() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut()).unwrap();

    // Purchase Tokens
    let purchase_amount = coins(100, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let info = mock_info("purchaser", &purchase_amount);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::NoOngoingSale {});
}

#[test]
pub fn test_purchase_not_enough_sent_native() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "native:test",
        &Sale {
            amount: Uint128::from(100u128),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: Uint128::from(100u128),
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(1, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let info = mock_info("purchaser", &purchase_amount);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to purchase a token".to_string()
        }
    );
}

#[test]
pub fn test_purchase_no_tokens_left_native() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "native:test",
        &Sale {
            amount: Uint128::zero(),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: Uint128::zero(),
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(100, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let info = mock_info("purchaser", &purchase_amount);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::NotEnoughTokens {});
}

#[test]
pub fn test_purchase_not_enough_tokens_native() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    //     let info = mock_info(owner.as_str(), &[]);

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "native:test",
        &Sale {
            amount: Uint128::from(1u128),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: Uint128::from(1u128),
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(100, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let info = mock_info("purchaser", &purchase_amount);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::NotEnoughTokens {});
}

#[test]
pub fn test_purchase_native() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let purchaser = Addr::unchecked("purchaser");
    let exchange_asset = AssetInfo::Native("test".to_string());

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: sale_amount,
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: sale_amount,
        },
    )
    .unwrap();

    // Purchase Tokens
    // Purchase Tokens
    let purchase_amount = coins(100, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let info = mock_info("purchaser", &purchase_amount);

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // Check transfer
    let msg = res.messages.first().unwrap();
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            MOCK_TOKEN_ADDRESS.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: purchaser.to_string(),
                amount: Uint128::from(10u128),
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::reply_on_error(expected_wasm, 2);
    assert_eq!(msg, &expected);

    // Check sale amount updated
    let sale = SALE
        .load(deps.as_mut().storage, &exchange_asset.to_string())
        .unwrap();

    assert_eq!(
        sale.amount,
        sale_amount.checked_sub(Uint128::from(10u128)).unwrap()
    );

    // Check recipient received funds
    let msg = &res.messages[1];
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Bank(BankMsg::Send {
        to_address: owner.to_string(),
        amount: purchase_amount.to_vec(),
    });
    let expected = SubMsg::reply_on_error(expected_wasm, 3);

    assert_eq!(msg, &expected);
}

#[test]
pub fn test_purchase_refund() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "native:test",
        &Sale {
            amount: Uint128::from(100u128),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: Uint128::from(100u128),
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(105, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let info = mock_info("purchaser", &purchase_amount);

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let refund_attribute = res.attributes.first().unwrap();
    let refund_message = res.messages.first().unwrap();

    assert_eq!(refund_attribute, attr("refunded_amount", "5"));
    assert_eq!(
        refund_message,
        &SubMsg::reply_on_error(
            CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: coins(5u128, "test")
            }),
            1
        )
    )
}

#[test]
pub fn test_cancel_sale_unauthorised() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    //     let info = mock_info(owner.as_str(), &[]);
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: sale_amount,
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: sale_amount,
        },
    )
    .unwrap();

    let msg = ExecuteMsg::CancelSale {
        asset: exchange_asset,
    };
    let unauthorised_info = mock_info("anyone", &[]);

    let err = execute(deps.as_mut(), env, unauthorised_info, msg).unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {})
}

#[test]
pub fn test_cancel_sale_no_sale() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let info = mock_info(owner.as_str(), &[]);
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::CancelSale {
        asset: exchange_asset,
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::NoOngoingSale {})
}

#[test]
pub fn test_cancel_sale() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");
    let info = mock_info(owner.as_str(), &[]);
    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset.to_string(),
        &Sale {
            amount: sale_amount,
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),
            end_time: Expiration::Never {},
            start_amount: sale_amount,
        },
    )
    .unwrap();

    let msg = ExecuteMsg::CancelSale {
        asset: exchange_asset.clone(),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // Ensure sale has been removed
    let sale_opt = SALE
        .may_load(deps.as_mut().storage, &exchange_asset.to_string())
        .unwrap();
    assert!(sale_opt.is_none());

    // Ensure any remaining funds are returned
    let message = res.messages.first().unwrap();
    let expected_message = SubMsg::reply_on_error(
        CosmosMsg::Wasm(
            wasm_execute(
                "cw20",
                &Cw20ExecuteMsg::Transfer {
                    recipient: owner.to_string(),
                    amount: sale_amount,
                },
                vec![],
            )
            .unwrap(),
        ),
        1,
    );
    assert_eq!(message, &expected_message)
}

#[test]
fn test_query_sale() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    let msg = QueryMsg::Sale {
        asset: exchange_asset.clone(),
    };
    let not_found_response: SaleResponse =
        from_json(query(deps.as_ref(), env.clone(), msg.clone()).unwrap()).unwrap();

    assert!(not_found_response.sale.is_none());

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let sale = Sale {
        amount: sale_amount,
        exchange_rate,
        recipient: "owner".to_string(),
        start_time: Expiration::AtTime(env.block.time),

        end_time: Expiration::Never {},
        start_amount: sale_amount,
    };
    SALE.save(deps.as_mut().storage, &exchange_asset.to_string(), &sale)
        .unwrap();

    let found_response: SaleResponse = from_json(query(deps.as_ref(), env, msg).unwrap()).unwrap();

    assert_eq!(found_response.sale, Some(sale));
}

#[test]
fn test_query_token_address() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut()).unwrap();

    let msg = QueryMsg::TokenAddress {};
    let resp: TokenAddressResponse = from_json(query(deps.as_ref(), env, msg).unwrap()).unwrap();

    assert_eq!(resp.address, MOCK_TOKEN_ADDRESS.to_string())
}

#[test]
fn test_andr_query() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let exchange_asset = AssetInfo::Cw20(Addr::unchecked("exchanged_asset"));

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let sale = Sale {
        amount: sale_amount,
        exchange_rate,
        recipient: "owner".to_string(),
        start_time: Expiration::AtTime(env.block.time),

        end_time: Expiration::Never {},
        start_amount: sale_amount,
    };
    SALE.save(deps.as_mut().storage, &exchange_asset.to_string(), &sale)
        .unwrap();

    let msg = QueryMsg::Sale {
        asset: exchange_asset,
    };
    let query_msg_response: SaleResponse =
        from_json(query(deps.as_ref(), env, msg).unwrap()).unwrap();

    assert_eq!(query_msg_response.sale, Some(sale));

    // let key_msg = QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
    //     to_json_binary(&exchange_asset.to_string()).unwrap(),
    // )));
    // let key_response: SaleResponse =
    //     from_json(&query(deps.as_ref(), env, key_msg).unwrap()).unwrap();

    // assert_eq!(key_response.sale, Some(sale));
}

#[test]
fn test_purchase_native_invalid_coins() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = Addr::unchecked("owner");

    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "native:test",
        &Sale {
            amount: Uint128::from(100u128),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: Uint128::from(100u128),
        },
    )
    .unwrap();

    let purchaser = Addr::unchecked("purchaser");
    let msg = ExecuteMsg::Purchase { recipient: None };

    let empty_coin_info = mock_info(purchaser.as_str(), &coins(0u128, "test"));
    let err = execute(deps.as_mut(), env.clone(), empty_coin_info, msg.clone()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );

    let two_coin_info = mock_info(
        purchaser.as_str(),
        &[coin(100u128, "test"), coin(10u128, "testtwo")],
    );
    let err = execute(deps.as_mut(), env.clone(), two_coin_info, msg.clone()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::MultipleDenoms {})
    );

    let no_coin_info = mock_info(purchaser.as_str(), &[]);
    let err = execute(deps.as_mut(), env, no_coin_info, msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    )
}

#[test]
fn test_query_sale_assets() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);
    let owner = Addr::unchecked("owner");
    init(deps.as_mut()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "native:test",
        &Sale {
            amount: Uint128::from(100u128),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: Uint128::from(100u128),
        },
    )
    .unwrap();
    SALE.save(
        deps.as_mut().storage,
        "cw20:testaddress",
        &Sale {
            amount: Uint128::from(100u128),
            exchange_rate,
            recipient: owner.to_string(),
            start_time: Expiration::AtTime(env.block.time),

            end_time: Expiration::Never {},
            start_amount: Uint128::from(100u128),
        },
    )
    .unwrap();

    let query_msg = QueryMsg::SaleAssets {
        limit: None,
        start_after: None,
    };
    let resp: SaleAssetsResponse =
        from_json(query(deps.as_ref(), env, query_msg).unwrap()).unwrap();

    assert_eq!(resp.assets.len(), 2);
    assert_eq!(resp.assets[0], "cw20:testaddress");
    assert_eq!(resp.assets[1], "native:test");
}
