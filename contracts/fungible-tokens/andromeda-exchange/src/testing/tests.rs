use andromeda_fungible_tokens::exchange::{
    Cw20HookMsg, ExchangeRate, ExecuteMsg, InstantiateMsg, QueryMsg, Redeem, RedeemResponse, Sale,
    SaleAssetsResponse, SaleResponse, TokenAddressResponse,
};
use andromeda_std::{
    ado_base::permissioning::PermissioningMessage,
    amp::{AndrAddr, Recipient},
    common::{
        denom::Asset,
        expiration::{Expiry, MILLISECONDS_TO_NANOSECONDS_RATIO},
        schedule::Schedule,
        Milliseconds,
    },
    error::ContractError,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    attr, coin, coins, from_json,
    testing::{message_info, mock_env},
    to_json_binary, wasm_execute, Addr, BankMsg, Coin, CosmosMsg, Decimal256, Empty, Response,
    SubMsg, Timestamp, Uint128,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use rstest::*;

pub const MOCK_TOKEN_ADDRESS: &str = "cw20";

use crate::{
    contract::{execute, instantiate, query},
    state::{REDEEM, SALE, TOKEN_ADDRESS},
    testing::mock_querier::mock_dependencies_custom,
};

use super::mock_querier::TestDeps;

fn init(deps: &mut TestDeps) -> Result<Response, ContractError> {
    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let mock_token_address = deps.api.addr_make(MOCK_TOKEN_ADDRESS);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,

        token_address: AndrAddr::from_string(mock_token_address.to_string()),
    };

    instantiate(deps.as_mut(), mock_env(), info, msg)
}
#[test]
pub fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps).unwrap();

    let saved_mock_token_address = TOKEN_ADDRESS.load(deps.as_ref().storage).unwrap();

    let mock_token_address = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    assert_eq!(saved_mock_token_address, mock_token_address.to_string())
}

#[test]
pub fn test_start_sale_invalid_token() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);
    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let exchange_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset_addr.to_string()));

    init(&mut deps).unwrap();

    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate: Uint128::from(10u128),
        recipient: None,
        schedule: Schedule::new(None, None),
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
    let owner = deps.api.addr_make("owner");
    let not_owner = deps.api.addr_make("not_owner");
    let info = message_info(&owner, &[]);
    let exchange_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset_addr.to_string()));

    init(&mut deps).unwrap();

    // Set permission for start sale

    let permission_msg = ExecuteMsg::Permissioning(PermissioningMessage::PermissionAction {
        action: "Receive".to_string(),
    });
    execute(deps.as_mut(), env.clone(), info.clone(), permission_msg).unwrap();

    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate: Uint128::from(10u128),
        recipient: None,
        schedule: Schedule::new(None, None),
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: not_owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: Uint128::from(100u128),
    };
    let msg = ExecuteMsg::Receive(receive_msg);
    let info = message_info(&not_owner, &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {})
}

#[test]
pub fn test_start_sale_zero_amount() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let exchange_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset_addr.to_string()));

    init(&mut deps).unwrap();

    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate: Uint128::from(10u128),
        recipient: None,
        schedule: Schedule::new(None, None),
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

    let owner = deps.api.addr_make("owner");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    //     let info = message_info(owner.as_str(), &[]);
    let mock_cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let token_info = message_info(&mock_cw20_addr, &[]);

    init(&mut deps).unwrap();
    let current_time = env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset.clone(),
        exchange_rate,
        recipient: None,
        // A start time ahead of the current time
        schedule: Schedule::new(
            Some(Expiry::AtTime(Milliseconds(current_time + 10))),
            Some(Expiry::FromNow(Milliseconds(60_000))),
        ),
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: sale_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    execute(deps.as_mut(), env, token_info, msg).unwrap();

    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();
    let sale = SALE
        .load(deps.as_ref().storage, &exchange_asset_str)
        .unwrap();

    assert_eq!(sale.exchange_rate, exchange_rate);
    assert_eq!(sale.remaining_amount, sale_amount);
    assert_eq!(sale.start_amount, sale_amount);

    let expected_start_time =
        Timestamp::from_nanos((current_time + 10) * MILLISECONDS_TO_NANOSECONDS_RATIO);
    assert_eq!(
        sale.start_time,
        Milliseconds::from_nanos(expected_start_time.nanos())
    );

    let expected_expiration_time =
        Timestamp::from_nanos((current_time + 60_000 + 10) * MILLISECONDS_TO_NANOSECONDS_RATIO);
    assert_eq!(
        sale.end_time,
        Some(Milliseconds::from_nanos(expected_expiration_time.nanos()))
    );
}

#[test]
pub fn test_start_sale_no_start_no_duration() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    //     let info = message_info(owner.as_str(), &[]);
    let mock_cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let token_info = message_info(&mock_cw20_addr, &[]);

    init(&mut deps).unwrap();
    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset.clone(),
        exchange_rate,
        recipient: None,
        // A start time ahead of the current time
        schedule: Schedule::new(None, None),
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: owner.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: sale_amount,
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    execute(deps.as_mut(), env, token_info, msg).unwrap();

    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();
    let sale = SALE
        .load(deps.as_ref().storage, &exchange_asset_str)
        .unwrap();

    assert_eq!(sale.exchange_rate, exchange_rate);
    assert_eq!(sale.remaining_amount, sale_amount);
    assert_eq!(sale.start_amount, sale_amount);

    assert_eq!(
        sale.start_time,
        // Current time
        Milliseconds::from_nanos(Timestamp::from_nanos(1571797419879000000).nanos())
    );

    assert_eq!(sale.end_time, None);
}

#[test]
pub fn test_start_sale_invalid_start_time() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let exchange_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset_addr.to_string()));
    let mock_cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let token_info = message_info(&mock_cw20_addr, &[]);

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate,
        recipient: None,
        schedule: Schedule::new(Some(Expiry::AtTime(Milliseconds(1))), None),
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

    let owner = deps.api.addr_make("owner");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    //     let info = message_info(owner.as_str(), &[]);
    let mock_cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let token_info = message_info(&mock_cw20_addr, &[]);

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate,
        recipient: None,
        schedule: Schedule::new(None, None),
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

    let owner = deps.api.addr_make("owner");
    let exchange_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset_addr.to_string()));
    let token_info = message_info(&Addr::unchecked(MOCK_TOKEN_ADDRESS), &[]);

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::zero();
    let sale_amount = Uint128::from(100u128);
    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset,
        exchange_rate,
        recipient: None,
        schedule: Schedule::new(None, None),
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
    let purchaser = deps.api.addr_make("purchaser");
    let invalid_token = deps.api.addr_make("invalid_token");
    let token_info = message_info(&invalid_token, &[]);

    init(&mut deps).unwrap();

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

    let owner = deps.api.addr_make("owner");
    let purchaser = deps.api.addr_make("purchaser");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: Uint128::from(100u128),
            remaining_amount: Uint128::from(100u128),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_info = message_info(&exchanged_asset_addr, &[]);
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

    let owner = deps.api.addr_make("owner");
    let purchaser = deps.api.addr_make("purchaser");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: Uint128::zero(),
            remaining_amount: Uint128::zero(),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = message_info(&exchanged_asset_addr, &[]);
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

    let owner = deps.api.addr_make("owner");
    let purchaser = deps.api.addr_make("purchaser");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: Uint128::one(),
            remaining_amount: Uint128::one(),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = message_info(&exchanged_asset_addr, &[]);
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

    let owner = deps.api.addr_make("owner");
    let purchaser = deps.api.addr_make("purchaser");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: sale_amount,
            remaining_amount: sale_amount,
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchange_info = message_info(&exchanged_asset_addr, &[]);
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
    let cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let msg = res.messages.first().unwrap();
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            cw20_addr.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: purchaser.to_string(),
                amount: Uint128::from(10u128),
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::new(expected_wasm);
    assert_eq!(msg, &expected);

    // Check sale amount updated
    let sale = SALE
        .load(deps.as_mut().storage, &exchange_asset_str)
        .unwrap();

    assert_eq!(
        sale.remaining_amount,
        sale_amount.checked_sub(Uint128::from(10u128)).unwrap()
    );

    // Check recipient received funds
    let msg = &res.messages[1];
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            exchanged_asset_addr.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: purchase_amount,
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::new(expected_wasm);

    assert_eq!(msg, &expected);
}

#[test]
pub fn test_purchase_with_start_and_duration() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let purchaser = deps.api.addr_make("purchaser");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: sale_amount,
            remaining_amount: sale_amount,
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            // start time in the past
            start_time: Milliseconds::from_nanos(env.block.time.minus_nanos(1).nanos()),
            // end time in the future
            end_time: Some(Milliseconds::from_nanos(
                env.block.time.plus_days(1).nanos(),
            )),
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchanged_asset = deps.api.addr_make("exchanged_asset");
    let exchange_info = message_info(&exchanged_asset, &[]);
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
    let mock_cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            mock_cw20_addr.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: purchaser.to_string(),
                amount: Uint128::from(10u128),
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::new(expected_wasm);
    assert_eq!(msg, &expected);

    // Check sale amount updated
    let sale = SALE
        .load(deps.as_mut().storage, &exchange_asset_str)
        .unwrap();

    assert_eq!(
        sale.remaining_amount,
        sale_amount.checked_sub(Uint128::from(10u128)).unwrap()
    );

    // Check recipient received funds
    let msg = &res.messages[1];
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            exchanged_asset_addr.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: purchase_amount,
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::new(expected_wasm);

    assert_eq!(msg, &expected);
}

#[test]
pub fn test_purchase_sale_not_started() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let purchaser = deps.api.addr_make("purchaser");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: sale_amount,
            remaining_amount: sale_amount,
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.plus_days(1).nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchanged_asset = deps.api.addr_make("exchanged_asset");
    let exchange_info = message_info(&exchanged_asset, &[]);
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

    let owner = deps.api.addr_make("owner");
    let purchaser = deps.api.addr_make("purchaser");
    let exchanged_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchanged_asset_addr.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: sale_amount,
            remaining_amount: sale_amount,
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: Some(Milliseconds::from_nanos(
                env.block.time.minus_nanos(1).nanos(),
            )),
        },
    )
    .unwrap();

    // Purchase Tokens
    let exchanged_asset = deps.api.addr_make("exchanged_asset");
    let exchange_info = message_info(&exchanged_asset, &[]);
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

    init(&mut deps).unwrap();

    // Purchase Tokens
    let purchaser = deps.api.addr_make("purchaser");
    let purchase_amount = coins(100, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let info = message_info(&purchaser, &purchase_amount);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::NoOngoingSale {});
}

#[test]
pub fn test_purchase_not_enough_sent_native() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "test",
        &Sale {
            start_amount: Uint128::from(100u128),
            remaining_amount: Uint128::from(100u128),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(1, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let purchaser = deps.api.addr_make("purchaser");
    let info = message_info(&purchaser, &purchase_amount);

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

    let owner = deps.api.addr_make("owner");

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "test",
        &Sale {
            start_amount: Uint128::zero(),
            remaining_amount: Uint128::zero(),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(100, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let purchaser = deps.api.addr_make("purchaser");
    let info = message_info(&purchaser, &purchase_amount);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::NotEnoughTokens {});
}

#[test]
pub fn test_purchase_not_enough_tokens_native() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    //     let info = message_info(owner.as_str(), &[]);

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "test",
        &Sale {
            start_amount: Uint128::from(1u128),
            remaining_amount: Uint128::from(1u128),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(100, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let purchaser = deps.api.addr_make("purchaser");
    let info = message_info(&purchaser, &purchase_amount);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::NotEnoughTokens {});
}

#[test]
pub fn test_purchase_native() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let test_addr = deps.api.addr_make("test");
    let exchange_asset = Asset::NativeToken(test_addr.to_string());
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(9u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: sale_amount,
            remaining_amount: sale_amount,
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(100, test_addr.to_string());
    let msg = ExecuteMsg::Purchase { recipient: None };
    let purchaser = deps.api.addr_make("purchaser");
    let info = message_info(&purchaser, &purchase_amount);

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // Check refund
    let msg = res.messages[0].clone();
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Bank(BankMsg::Send {
        to_address: purchaser.to_string(),
        amount: vec![Coin::new(1_u128, test_addr.to_string())],
    });
    let expected = SubMsg::new(expected_wasm);
    assert_eq!(msg, expected);

    // Check transfer
    let mock_token_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let msg = res.messages[1].clone();
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Wasm(
        wasm_execute(
            mock_token_addr.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: purchaser.to_string(),
                amount: Uint128::from(11u128),
            },
            vec![],
        )
        .unwrap(),
    );
    let expected = SubMsg::new(expected_wasm);
    assert_eq!(msg, expected);

    // Check sale amount updated
    let sale = SALE
        .load(deps.as_mut().storage, &exchange_asset_str)
        .unwrap();

    assert_eq!(
        sale.remaining_amount,
        sale_amount.checked_sub(Uint128::from(11u128)).unwrap()
    );

    // Check recipient received funds
    let msg = &res.messages[2];
    let expected_wasm: CosmosMsg<Empty> = CosmosMsg::Bank(BankMsg::Send {
        to_address: owner.to_string(),
        amount: vec![Coin::new(99_u128, test_addr.to_string())],
    });
    let expected = SubMsg::new(expected_wasm);

    assert_eq!(msg, &expected);
}

#[test]
pub fn test_purchase_refund() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "test",
        &Sale {
            start_amount: Uint128::from(100u128),
            remaining_amount: Uint128::from(100u128),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    // Purchase Tokens
    let purchase_amount = coins(105, "test");
    let msg = ExecuteMsg::Purchase { recipient: None };
    let purchaser = deps.api.addr_make("purchaser");
    let info = message_info(&purchaser, &purchase_amount);

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let refund_attribute = res.attributes.first().unwrap();
    let refund_message = res.messages.first().unwrap();

    assert_eq!(refund_attribute, attr("refunded_amount", "5"));
    assert_eq!(
        refund_message,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: coins(5u128, "test")
        }),)
    )
}

#[test]
pub fn test_cancel_sale_unauthorised() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let exchange_asset = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: sale_amount,
            remaining_amount: sale_amount,
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    let msg = ExecuteMsg::CancelSale {
        asset: exchange_asset,
    };
    let anyone = deps.api.addr_make("anyone");
    let unauthorised_info = message_info(&anyone, &[]);

    let err = execute(deps.as_mut(), env, unauthorised_info, msg).unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {})
}

#[test]
pub fn test_cancel_sale_no_sale() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let exchange_asset = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset.to_string()));

    init(&mut deps).unwrap();

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

    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let exchange_asset = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    SALE.save(
        deps.as_mut().storage,
        &exchange_asset_str,
        &Sale {
            start_amount: sale_amount,
            remaining_amount: sale_amount,
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
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
    let mock_cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let expected_message = SubMsg::new(CosmosMsg::Wasm(
        wasm_execute(
            mock_cw20_addr.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: sale_amount,
            },
            vec![],
        )
        .unwrap(),
    ));
    assert_eq!(message, &expected_message)
}

#[test]
fn test_query_sale() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let exchange_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset_addr.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    let msg = QueryMsg::Sale {
        asset: exchange_asset_str.clone(),
    };
    let not_found_response: SaleResponse =
        from_json(query(deps.as_ref(), env.clone(), msg.clone()).unwrap()).unwrap();

    assert!(not_found_response.sale.is_none());

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let sale = Sale {
        start_amount: sale_amount,
        remaining_amount: sale_amount,
        exchange_rate,
        recipient: Recipient::from_string("owner".to_string()),
        start_time: Milliseconds::from_nanos(env.block.time.nanos()),
        end_time: None,
    };
    SALE.save(deps.as_mut().storage, &exchange_asset_str, &sale)
        .unwrap();

    let found_response: SaleResponse = from_json(query(deps.as_ref(), env, msg).unwrap()).unwrap();

    assert_eq!(found_response.sale, Some(sale));
}

#[test]
fn test_query_token_address() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    init(&mut deps).unwrap();

    let msg = QueryMsg::TokenAddress {};
    let resp: TokenAddressResponse = from_json(query(deps.as_ref(), env, msg).unwrap()).unwrap();

    let mock_cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    assert_eq!(resp.address, mock_cw20_addr.to_string())
}

#[test]
fn test_andr_query() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);
    let exchange_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset_addr.to_string()));
    let exchange_asset_str = exchange_asset.inner(&deps.as_ref()).unwrap();

    let exchange_rate = Uint128::from(10u128);
    let sale_amount = Uint128::from(100u128);
    let sale = Sale {
        start_amount: sale_amount,
        remaining_amount: sale_amount,
        exchange_rate,
        recipient: Recipient::from_string("owner".to_string()),
        start_time: Milliseconds::from_nanos(env.block.time.nanos()),
        end_time: None,
    };
    SALE.save(deps.as_mut().storage, &exchange_asset_str, &sale)
        .unwrap();

    let msg = QueryMsg::Sale {
        asset: exchange_asset_str,
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

    let owner = deps.api.addr_make("owner");

    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "test",
        &Sale {
            start_amount: Uint128::from(100u128),
            remaining_amount: Uint128::from(100u128),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();

    let purchaser = deps.api.addr_make("purchaser");
    let msg = ExecuteMsg::Purchase { recipient: None };

    let empty_coin_info = message_info(&purchaser, &coins(0u128, "test"));
    let err = execute(deps.as_mut(), env.clone(), empty_coin_info, msg.clone()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );

    let two_coin_info = message_info(
        &purchaser,
        &[coin(100u128, "test"), coin(10u128, "testtwo")],
    );
    let err = execute(deps.as_mut(), env.clone(), two_coin_info, msg.clone()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::MultipleDenoms {})
    );

    let no_coin_info = message_info(&purchaser, &[]);
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
    let owner = deps.api.addr_make("owner");
    init(&mut deps).unwrap();

    let exchange_rate = Uint128::from(10u128);
    SALE.save(
        deps.as_mut().storage,
        "test",
        &Sale {
            start_amount: Uint128::from(100u128),
            remaining_amount: Uint128::from(100u128),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
        },
    )
    .unwrap();
    SALE.save(
        deps.as_mut().storage,
        "cw20:testaddress",
        &Sale {
            start_amount: Uint128::from(100u128),
            remaining_amount: Uint128::from(100u128),
            exchange_rate,
            recipient: Recipient::from_string(owner.to_string()),
            start_time: Milliseconds::from_nanos(env.block.time.nanos()),
            end_time: None,
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
    assert_eq!(resp.assets[1], "test");
}

#[test]
fn test_start_sale_same_asset() {
    let mut deps = mock_dependencies_custom(&[]);
    let cw20_addr = deps.api.addr_make("cw20");
    let token_info = message_info(&cw20_addr, &[]);

    init(&mut deps).unwrap();

    let cw20_msg = Cw20ReceiveMsg {
        sender: "owner".to_string(),
        msg: to_json_binary(&Cw20HookMsg::StartSale {
            asset: Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string())),
            exchange_rate: Uint128::from(10u128),
            recipient: None,
            schedule: Schedule::new(None, None),
        })
        .unwrap(),
        amount: Uint128::from(100u128),
    };
    let msg = ExecuteMsg::Receive(cw20_msg);

    let err = execute(deps.as_mut(), mock_env(), token_info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidAsset {
            asset: Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string())).to_string()
        }
    );
}

#[test]
fn test_cancel_redeem() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let redeem_asset = deps.api.addr_make("redeem_asset");
    let redeem_asset = Asset::Cw20Token(AndrAddr::from_string(redeem_asset.to_string()));
    let redeem_asset_str = redeem_asset.inner(&deps.as_ref()).unwrap();
    let mock_token_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let payout_asset = Asset::Cw20Token(AndrAddr::from_string(mock_token_addr.to_string()));

    init(&mut deps).unwrap();

    // Setup a redeem condition
    let redeem_amount = Uint128::from(100u128);
    let exchange_rate = Decimal256::percent(200); // 2:1 ratio
    REDEEM
        .save(
            deps.as_mut().storage,
            &redeem_asset_str,
            &Redeem {
                asset: payout_asset.clone(),
                amount: redeem_amount,
                amount_paid_out: Uint128::zero(),
                exchange_rate,
                exchange_type: ExchangeRate::Fixed(exchange_rate),
                recipient: Recipient::from_string(owner.to_string()),
                start_time: Milliseconds::from_nanos(env.block.time.nanos()),
                end_time: None,
            },
        )
        .unwrap();

    // Verify redeem condition exists
    let query_msg = QueryMsg::Redeem {
        asset: redeem_asset.clone(),
    };
    let query_resp: RedeemResponse =
        from_json(query(deps.as_ref(), env.clone(), query_msg.clone()).unwrap()).unwrap();
    assert!(query_resp.redeem.is_some());
    assert_eq!(query_resp.redeem.clone().unwrap().asset, payout_asset);
    assert_eq!(query_resp.redeem.unwrap().amount, redeem_amount);

    // Cancel the redeem
    let cancel_msg = ExecuteMsg::CancelRedeem {
        asset: redeem_asset.clone(),
    };
    let res = execute(deps.as_mut(), env.clone(), info, cancel_msg).unwrap();

    // Verify redeem has been removed
    let query_resp: RedeemResponse =
        from_json(query(deps.as_ref(), env, query_msg).unwrap()).unwrap();
    assert!(query_resp.redeem.is_none());

    // Verify remaining funds were returned
    let message = res.messages.first().unwrap();

    let expected_message = SubMsg::new(CosmosMsg::Wasm(
        wasm_execute(
            mock_token_addr.to_string(),
            &Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: redeem_amount,
            },
            vec![],
        )
        .unwrap(),
    ));
    assert_eq!(message, &expected_message);

    // Check that appropriate attributes were emitted
    assert_eq!(
        res.attributes,
        vec![
            attr("refunded_amount", redeem_amount),
            attr("action", "cancel_redeem"),
            attr("asset", redeem_asset.to_string()),
        ]
    );
}

#[test]
fn test_cancel_redeem_no_redeem() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let redeem_asset_addr = deps.api.addr_make("redeem_asset");
    let redeem_asset = Asset::Cw20Token(AndrAddr::from_string(redeem_asset_addr.to_string()));

    init(&mut deps).unwrap();

    let msg = ExecuteMsg::CancelRedeem {
        asset: redeem_asset,
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::NoOngoingRedeem {});
}

#[test]
fn test_cancel_redeem_unauthorized() {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let owner = deps.api.addr_make("owner");
    let not_owner = deps.api.addr_make("not_owner");
    let info = message_info(&not_owner, &[]);
    let redeem_asset_addr = deps.api.addr_make("redeem_asset");
    let redeem_asset = Asset::Cw20Token(AndrAddr::from_string(redeem_asset_addr.to_string()));
    let redeem_asset_str = redeem_asset.inner(&deps.as_ref()).unwrap();
    let payout_asset_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let payout_asset = Asset::Cw20Token(AndrAddr::from_string(payout_asset_addr.to_string()));

    init(&mut deps).unwrap();

    // Setup a redeem condition
    REDEEM
        .save(
            deps.as_mut().storage,
            &redeem_asset_str,
            &Redeem {
                asset: payout_asset.clone(),
                amount: Uint128::from(100u128),
                amount_paid_out: Uint128::zero(),
                exchange_rate: Decimal256::percent(200),
                exchange_type: ExchangeRate::Fixed(Decimal256::percent(200)),
                recipient: Recipient::from_string(owner.to_string()),
                start_time: Milliseconds::from_nanos(env.block.time.nanos()),
                end_time: None,
            },
        )
        .unwrap();

    let msg = ExecuteMsg::CancelRedeem {
        asset: redeem_asset.clone(),
    };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // Try to create a duplicate redeem
    let redeem_msg = ExecuteMsg::StartRedeem {
        redeem_asset: redeem_asset.clone(),
        exchange_rate: ExchangeRate::Fixed(Decimal256::percent(200)),
        recipient: None,
        schedule: Schedule::new(None, None),
    };
    let info = message_info(&owner, &[coin(100u128, "uusd")]);
    let err = execute(deps.as_mut(), env.clone(), info.clone(), redeem_msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::RedeemNotEnded {});

    // Set up a redeem with an unexpired end time
    REDEEM
        .save(
            deps.as_mut().storage,
            &redeem_asset_str,
            &Redeem {
                asset: payout_asset.clone(),
                amount: Uint128::from(100u128),
                amount_paid_out: Uint128::zero(),
                exchange_rate: Decimal256::percent(200),
                exchange_type: ExchangeRate::Fixed(Decimal256::percent(200)),
                recipient: Recipient::from_string(owner.to_string()),
                start_time: Milliseconds::from_nanos(env.block.time.nanos()),
                end_time: Some(Milliseconds::from_nanos(
                    env.block.time.nanos() + 10000000000000,
                )),
            },
        )
        .unwrap();
    let err = execute(deps.as_mut(), env.clone(), info.clone(), redeem_msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::RedeemNotEnded {});

    // Set up a redeem with an no funds left, and unexpired end time
    REDEEM
        .save(
            deps.as_mut().storage,
            &redeem_asset_str,
            &Redeem {
                asset: payout_asset,
                amount: Uint128::zero(),
                amount_paid_out: Uint128::zero(),
                exchange_rate: Decimal256::percent(200),
                exchange_type: ExchangeRate::Fixed(Decimal256::percent(200)),
                recipient: Recipient::from_string(owner.to_string()),
                start_time: Milliseconds::from_nanos(env.block.time.nanos()),
                end_time: Some(Milliseconds::from_nanos(
                    env.block.time.nanos() + 10000000000000,
                )),
            },
        )
        .unwrap();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), redeem_msg);
    assert!(res.is_ok());
}

// There was an auth check that only allowed the contract owner to start a redeem
// That was removed in #942, and this tests ensures that anyone can start a redeem
#[rstest]
#[case("owner")]
#[case("not_owner")]
fn test_start_redeem_authorization(#[case] sender: &str) {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let sender_addr = deps.api.addr_make(sender);
    let redeem_asset = Asset::NativeToken("uandr".to_string());

    // Init sets the "owner" address as the contract owner
    init(&mut deps).unwrap();

    let msg = ExecuteMsg::StartRedeem {
        redeem_asset: redeem_asset.clone(),
        exchange_rate: ExchangeRate::Fixed(Decimal256::percent(200)),
        recipient: None,
        schedule: Schedule::new(None, None),
    };

    let info = message_info(&sender_addr, &[coin(100u128, "uusd")]);
    let result = execute(deps.as_mut(), env, info, msg);

    assert!(result.is_ok(), "{} should be able to start redeem", sender);
}

// There was an auth check that only allowed the contract owner to start a sale
// That was removed, and this test ensures that anyone can start a sale via CW20 hook
#[rstest]
#[case("owner")]
#[case("not_owner")]
fn test_start_sale_authorization(#[case] sender: &str) {
    let env = mock_env();
    let mut deps = mock_dependencies_custom(&[]);

    let sender_addr = deps.api.addr_make(sender);
    let exchange_asset_addr = deps.api.addr_make("exchanged_asset");
    let exchange_asset = Asset::Cw20Token(AndrAddr::from_string(exchange_asset_addr.to_string()));
    let mock_cw20_addr = deps.api.addr_make(MOCK_TOKEN_ADDRESS);
    let token_info = message_info(&mock_cw20_addr, &[]);

    // Init sets the "owner" address as the contract owner
    init(&mut deps).unwrap();

    let hook = Cw20HookMsg::StartSale {
        asset: exchange_asset.clone(),
        exchange_rate: Uint128::from(10u128),
        recipient: None,
        schedule: Schedule::new(None, None),
    };
    let receive_msg = Cw20ReceiveMsg {
        sender: sender_addr.to_string(),
        msg: to_json_binary(&hook).unwrap(),
        amount: Uint128::from(100u128),
    };
    let msg = ExecuteMsg::Receive(receive_msg);

    let result = execute(deps.as_mut(), env, token_info, msg);

    assert!(result.is_ok(), "{} should be able to start sale", sender);
}
