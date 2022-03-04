use crate::contract::{execute, instantiate, query, reply, DEPOSIT_ID, WITHDRAW_ID};
use crate::state::{
    Config, Position, CONFIG, POSITION, PREV_AUST_BALANCE, PREV_UUSD_BALANCE, RECIPIENT_ADDR,
};
use crate::testing::mock_querier::{
    mock_dependencies_custom, MOCK_AUST_TOKEN, MOCK_BLUNA_HUB_CONTRACT, MOCK_BLUNA_TOKEN,
    MOCK_CUSTODY_CONTRACT, MOCK_MARKET_CONTRACT, MOCK_ORACLE_CONTRACT, MOCK_OVERSEER_CONTRACT,
};
use andromeda_protocol::{
    anchor::{BLunaHubExecuteMsg, ExecuteMsg, InstantiateMsg, PositionResponse, QueryMsg},
    communication::{ADORecipient, AndromedaMsg, AndromedaQuery, Recipient},
    error::ContractError,
    withdraw::{Withdrawal, WithdrawalType},
};
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    attr, coin, coins, from_binary,
    testing::{mock_env, mock_info},
    to_binary, Addr, BankMsg, Coin, ContractResult, CosmosMsg, Decimal, DepsMut, Reply, Response,
    SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use moneymarket::{
    custody::Cw20HookMsg as CustodyCw20HookMsg,
    market::{Cw20HookMsg as MarketCw20HookMsg, ExecuteMsg as MarketExecuteMsg},
    overseer::ExecuteMsg as OverseerExecuteMsg,
};

fn deposit_stable_msg(amount: u128) -> SubMsg {
    SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "anchor_market".to_string(),
            msg: to_binary(&MarketExecuteMsg::DepositStable {}).unwrap(),
            funds: vec![coin(amount, "uusd")],
        }),
        DEPOSIT_ID,
    )
}

fn redeem_stable_msg(amount: u128) -> SubMsg {
    SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "aust_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "anchor_market".to_string(),
                amount: Uint128::from(amount),
                msg: to_binary(&MarketCw20HookMsg::RedeemStable {}).unwrap(),
            })
            .unwrap(),
            funds: vec![],
        }),
        WITHDRAW_ID,
    )
}

fn withdraw_aust_msg(amount: u128) -> SubMsg {
    SubMsg::new(WasmMsg::Execute {
        contract_addr: "aust_token".to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: "addr0000".to_string(),
            amount: amount.into(),
        })
        .unwrap(),
        funds: vec![],
    })
}

fn init(deps: DepsMut) {
    let env = mock_env();
    let owner = "addr0000";
    let info = mock_info(owner, &[]);
    let msg = InstantiateMsg {
        anchor_bluna_hub: MOCK_BLUNA_HUB_CONTRACT.to_owned(),
        anchor_bluna_custody: MOCK_CUSTODY_CONTRACT.to_owned(),
        anchor_market: MOCK_MARKET_CONTRACT.to_owned(),
    };
    let res = instantiate(deps, env, info, msg.clone()).unwrap();

    assert_eq!(0, res.messages.len());
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let config = CONFIG.load(deps.as_ref().storage).unwrap();

    assert_eq!(
        Config {
            anchor_market: Addr::unchecked(MOCK_MARKET_CONTRACT),
            aust_token: Addr::unchecked(MOCK_AUST_TOKEN),
            anchor_bluna_hub: Addr::unchecked(MOCK_BLUNA_HUB_CONTRACT),
            anchor_bluna_custody: Addr::unchecked(MOCK_CUSTODY_CONTRACT),
            anchor_overseer: Addr::unchecked(MOCK_OVERSEER_CONTRACT),
            bluna_token: Addr::unchecked(MOCK_BLUNA_TOKEN),
            anchor_oracle: Addr::unchecked(MOCK_ORACLE_CONTRACT)
        },
        config
    );
}

#[test]
fn test_deposit_and_withdraw_ust() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("addr0000", &[]);
    let amount = 1000000u128;

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Receive(None));
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(amount),
        }],
    );
    let env = mock_env();
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let expected_res = Response::new()
        .add_submessage(deposit_stable_msg(amount))
        .add_attributes(vec![
            attr("action", "deposit"),
            attr("deposit_amount", amount.to_string()),
        ]);
    assert_eq!(res, expected_res);
    assert!(POSITION.has(deps.as_mut().storage, "addr0000"));
    assert_eq!(
        Uint128::zero(),
        PREV_AUST_BALANCE.load(deps.as_mut().storage).unwrap()
    );
    assert_eq!(
        "addr0000",
        RECIPIENT_ADDR.load(deps.as_mut().storage).unwrap()
    );

    // Suppose exchange rate is 1 uusd = 0.5 aUST.
    let aust_amount = amount / 2;
    deps.querier.token_balance = aust_amount.into();

    let my_reply = Reply {
        id: DEPOSIT_ID,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    let res = reply(deps.as_mut(), mock_env(), my_reply).unwrap();
    assert_eq!(
        Response::new().add_attributes(vec![
            attr("action", "reply_update_position"),
            attr("recipient_addr", "addr0000"),
            attr("aust_amount", aust_amount.to_string()),
        ]),
        res
    );
    assert_eq!(
        aust_amount,
        POSITION
            .load(deps.as_mut().storage, "addr0000")
            .unwrap()
            .aust_amount
            .u128()
    );

    let query_res: PositionResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AndrQuery(AndromedaQuery::Get(Some(to_binary(&"addr0000").unwrap()))),
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        PositionResponse {
            recipient: Recipient::Addr("addr0000".to_string()),
            aust_amount: Uint128::from(aust_amount),
        },
        query_res
    );

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: None,
        tokens_to_withdraw: Some(vec![Withdrawal {
            token: "uusd".to_string(),
            withdrawal_type: None,
        }]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let expected_res = Response::new()
        .add_submessage(redeem_stable_msg(aust_amount))
        .add_attributes(vec![
            attr("action", "withdraw_uusd"),
            attr("recipient_addr", "addr0000"),
        ]);
    assert_eq!(res, expected_res);
    assert_eq!(
        "addr0000",
        RECIPIENT_ADDR.load(deps.as_mut().storage).unwrap()
    );
    assert_eq!(
        Uint128::zero(),
        POSITION
            .load(deps.as_mut().storage, "addr0000")
            .unwrap()
            .aust_amount
    );
    assert_eq!(
        Uint128::zero(),
        PREV_UUSD_BALANCE.load(deps.as_mut().storage).unwrap()
    );

    let my_reply = Reply {
        id: WITHDRAW_ID,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    // aUST has been redeemed back to uusd.
    deps.querier
        .base
        .update_balance(mock_env().contract.address, coins(amount, "uusd"));
    let res = reply(deps.as_mut(), mock_env(), my_reply).unwrap();
    assert_eq!(
        Response::new()
            .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: coins(amount, "uusd"),
            })))
            .add_attribute("action", "reply_withdraw_ust")
            .add_attribute("recipient", "addr0000")
            .add_attribute("amount", amount.to_string()),
        res
    );
}

#[test]
fn test_deposit_and_withdraw_aust() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let amount = 1000000u128;

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Receive(None));
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(amount),
        }],
    );
    let env = mock_env();
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let expected_res = Response::new()
        .add_submessage(deposit_stable_msg(amount))
        .add_attributes(vec![
            attr("action", "deposit"),
            attr("deposit_amount", amount.to_string()),
        ]);
    assert_eq!(res, expected_res);
    assert!(POSITION.has(deps.as_mut().storage, "addr0000"));
    assert_eq!(
        Uint128::zero(),
        PREV_AUST_BALANCE.load(deps.as_mut().storage).unwrap()
    );
    assert_eq!(
        "addr0000",
        RECIPIENT_ADDR.load(deps.as_mut().storage).unwrap()
    );

    // Suppose exchange rate is 1 uusd = 0.5 aUST.
    let aust_amount = amount / 2;
    deps.querier.token_balance = aust_amount.into();

    let my_reply = Reply {
        id: DEPOSIT_ID,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    let res = reply(deps.as_mut(), mock_env(), my_reply).unwrap();
    assert_eq!(
        Response::new().add_attributes(vec![
            attr("action", "reply_update_position"),
            attr("recipient_addr", "addr0000"),
            attr("aust_amount", aust_amount.to_string()),
        ]),
        res
    );
    assert_eq!(
        aust_amount,
        POSITION
            .load(deps.as_mut().storage, "addr0000")
            .unwrap()
            .aust_amount
            .u128()
    );

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: None,
        tokens_to_withdraw: Some(vec![Withdrawal {
            token: "aust".to_string(),
            withdrawal_type: None,
        }]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let expected_res = Response::new()
        .add_submessage(withdraw_aust_msg(aust_amount))
        .add_attributes(vec![
            attr("action", "withdraw_aust"),
            attr("recipient_addr", "addr0000"),
        ]);
    assert_eq!(res, expected_res);
    assert_eq!(
        Uint128::zero(),
        POSITION
            .load(deps.as_mut().storage, "addr0000")
            .unwrap()
            .aust_amount
    );
}

#[test]
fn test_deposit_existing_position() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let amount = 1000000u128;

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Receive(None));
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(amount),
        }],
    );
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Suppose exchange rate is 1 uusd = 0.5 aUST.
    let aust_amount = amount / 2;
    deps.querier.token_balance = aust_amount.into();

    let my_reply = Reply {
        id: DEPOSIT_ID,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), my_reply.clone()).unwrap();

    assert_eq!(
        aust_amount,
        POSITION
            .load(deps.as_mut().storage, "addr0000")
            .unwrap()
            .aust_amount
            .u128()
    );

    // Deposit again.
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // The amount of aUST has now doubled.
    deps.querier.token_balance = (aust_amount * 2).into();

    let _res = reply(deps.as_mut(), mock_env(), my_reply).unwrap();

    assert_eq!(
        2 * aust_amount,
        POSITION
            .load(deps.as_mut().storage, "addr0000")
            .unwrap()
            .aust_amount
            .u128()
    );
}

#[test]
fn test_deposit_other_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(
        to_binary(&Recipient::Addr("recipient".into())).unwrap(),
    )));
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(amount),
        }],
    );
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Suppose exchange rate is 1 uusd = 0.5 aUST.
    let aust_amount = amount / 2;
    deps.querier.token_balance = aust_amount.into();

    let my_reply = Reply {
        id: DEPOSIT_ID,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    let res = reply(deps.as_mut(), mock_env(), my_reply).unwrap();

    assert_eq!(
        Response::new().add_attributes(vec![
            attr("action", "reply_update_position"),
            attr("recipient_addr", "recipient"),
            attr("aust_amount", aust_amount.to_string()),
        ]),
        res
    );

    assert_eq!(
        aust_amount,
        POSITION
            .load(deps.as_mut().storage, "recipient")
            .unwrap()
            .aust_amount
            .u128()
    );
}

#[test]
fn test_withdraw_percent() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let position = Position {
        recipient: Recipient::Addr(recipient.to_owned()),
        aust_amount: Uint128::from(amount),
    };
    POSITION
        .save(deps.as_mut().storage, recipient, &position)
        .unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: Some(vec![Withdrawal {
            withdrawal_type: Some(WithdrawalType::Percentage(50u128.into())),
            token: "uusd".to_string(),
        }]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let expected_res = Response::new()
        .add_submessage(redeem_stable_msg(amount / 2))
        .add_attributes(vec![
            attr("action", "withdraw_uusd"),
            attr("recipient_addr", recipient),
        ]);
    assert_eq!(res, expected_res);
    assert_eq!(
        amount / 2,
        POSITION
            .load(deps.as_mut().storage, recipient)
            .unwrap()
            .aust_amount
            .u128()
    );
}

#[test]
fn test_withdraw_invalid_percent() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let position = Position {
        recipient: Recipient::Addr(recipient.to_owned()),
        aust_amount: Uint128::from(amount),
    };
    POSITION
        .save(deps.as_mut().storage, recipient, &position)
        .unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: Some(vec![Withdrawal {
            withdrawal_type: Some(WithdrawalType::Percentage(101u128.into())),
            token: "uusd".to_string(),
        }]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InvalidRate {}, res.unwrap_err());
}

#[test]
fn test_withdraw_amount() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let position = Position {
        recipient: Recipient::Addr(recipient.to_owned()),
        aust_amount: Uint128::from(amount),
    };
    POSITION
        .save(deps.as_mut().storage, recipient, &position)
        .unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: Some(vec![Withdrawal {
            withdrawal_type: Some(WithdrawalType::Amount(50u128.into())),
            token: "uusd".to_string(),
        }]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let expected_res = Response::new()
        .add_submessage(redeem_stable_msg(50u128))
        .add_attributes(vec![
            attr("action", "withdraw_uusd"),
            attr("recipient_addr", recipient),
        ]);
    assert_eq!(res, expected_res);
    assert_eq!(
        amount - 50u128,
        POSITION
            .load(deps.as_mut().storage, recipient)
            .unwrap()
            .aust_amount
            .u128()
    );
}

#[test]
fn test_withdraw_invalid_amount() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let position = Position {
        recipient: Recipient::Addr(recipient.to_owned()),
        aust_amount: Uint128::from(amount),
    };
    POSITION
        .save(deps.as_mut().storage, recipient, &position)
        .unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: Some(vec![Withdrawal {
            withdrawal_type: Some(WithdrawalType::Amount((amount + 1).into())),
            token: "uusd".to_string(),
        }]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Requested withdrawal amount exceeds token balance".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_invalid_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::ADO(ADORecipient {
            addr: recipient.to_owned(),
            msg: None,
        })),
        tokens_to_withdraw: Some(vec![Withdrawal {
            withdrawal_type: None,
            token: "uusd".to_string(),
        }]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidRecipientType {
            msg: "Only recipients of type Addr are allowed as it only specifies the owner of the position to withdraw from".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_tokens_none() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: None,
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidTokensToWithdraw {
            msg: "Must specify tokens to withdraw".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_tokens_empty() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: Some(vec![]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidTokensToWithdraw {
            msg: "Must specify exactly one of uusd or aust to withdraw".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_tokens_uusd_and_aust_specified() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: Some(vec![
            Withdrawal {
                withdrawal_type: None,
                token: "uusd".to_string(),
            },
            Withdrawal {
                withdrawal_type: None,
                token: "aust".to_string(),
            },
        ]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidTokensToWithdraw {
            msg: "Must specify exactly one of uusd or aust to withdraw".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_aust_with_address() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);

    let position = Position {
        recipient: Recipient::Addr("addr0000".to_string()),
        aust_amount: Uint128::from(amount),
    };
    POSITION
        .save(deps.as_mut().storage, "addr0000", &position)
        .unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr("addr0000".to_string())),
        tokens_to_withdraw: Some(vec![Withdrawal {
            withdrawal_type: None,
            // Specifying the contract address of aust is also valid.
            token: "aust_token".to_string(),
        }]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let expected_res = Response::new()
        .add_submessage(withdraw_aust_msg(amount))
        .add_attributes(vec![
            attr("action", "withdraw_aust"),
            attr("recipient_addr", "addr0000"),
        ]);
    assert_eq!(res, expected_res)
}

#[test]
fn test_withdraw_recipient_sender() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);

    let recipient = "recipient";

    let position = Position {
        recipient: Recipient::Addr(recipient.to_owned()),
        aust_amount: Uint128::from(amount),
    };
    POSITION
        .save(deps.as_mut().storage, recipient, &position)
        .unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: Some(vec![Withdrawal {
            withdrawal_type: None,
            token: "uusd".to_string(),
        }]),
    });
    // Sender is the recipient, NOT the owner of the contract.
    let info = mock_info(recipient, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let expected_res = Response::new()
        .add_submessage(redeem_stable_msg(amount))
        .add_attributes(vec![
            attr("action", "withdraw_uusd"),
            attr("recipient_addr", recipient),
        ]);
    assert_eq!(res, expected_res)
}

#[test]
fn test_deposit_collateral() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::DepositCollateral {};

    let info = mock_info("addr0000", &coins(100, "uluna"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "deposit_collateral")
            // Convert luna -> bluna
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_BLUNA_HUB_CONTRACT.to_owned(),
                funds: info.funds,
                msg: to_binary(&BLunaHubExecuteMsg::Bond {}).unwrap(),
            }))
            // Provide collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_BLUNA_TOKEN.to_owned(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_CUSTODY_CONTRACT.to_owned(),
                    msg: to_binary(&CustodyCw20HookMsg::DepositCollateral {}).unwrap(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
            }))
            // Lock collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_OVERSEER_CONTRACT.to_owned(),
                msg: to_binary(&OverseerExecuteMsg::LockCollateral {
                    collaterals: vec![(MOCK_BLUNA_TOKEN.to_owned(), Uint256::from(100u128))],
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_borrow_new_loan() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal::percent(50),
        recipient: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "borrow")
            .add_attribute("desired_ltv_ratio", "0.5")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_MARKET_CONTRACT.to_owned(),
                msg: to_binary(&MarketExecuteMsg::BorrowStable {
                    // The current collateral is worth 100
                    borrow_amount: Uint256::from(50u128),
                    to: Some(mock_env().contract.address.to_string()),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: coins(50, "uusd")
            })),
        res
    );
}

#[test]
fn test_borrow_existing_loan() {
    let mut deps = mock_dependencies_custom(&[]);
    deps.querier.loan_amount = Uint256::from(50u128);

    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal::percent(75),
        recipient: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "borrow")
            .add_attribute("desired_ltv_ratio", "0.75")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_MARKET_CONTRACT.to_owned(),
                msg: to_binary(&MarketExecuteMsg::BorrowStable {
                    // The current ltv ratio is 0.5, so need to borrow another 25 to get to 0.75
                    // ltv ratio.
                    borrow_amount: Uint256::from(25u128),
                    to: Some(mock_env().contract.address.to_string()),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: coins(25, "uusd")
            })),
        res
    );
}

#[test]
fn test_borrow_existing_loan_lower_ltv() {
    let mut deps = mock_dependencies_custom(&[]);
    deps.querier.loan_amount = Uint256::from(50u128);

    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal::percent(20),
        recipient: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    assert_eq!(
        ContractError::InvalidLtvRatio {
            msg: "Desired LTV ratio lower than current".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_borrow_ltv_too_hight() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal::one(),
        recipient: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    assert_eq!(
        ContractError::InvalidLtvRatio {
            msg: "Desired LTV ratio must be less than 1".to_string(),
        },
        res.unwrap_err()
    );
}
