use crate::contract::{execute, instantiate, query, reply, DEPOSIT_ID, WITHDRAW_ID};
use crate::state::{
    Position, CONFIG, POSITION, PREV_AUST_BALANCE, PREV_UUSD_BALANCE, RECIPIENT_ADDR,
};
use crate::testing::mock_querier::mock_dependencies_custom;
use andromeda_protocol::anchor::{
    AnchorMarketMsg, ExecuteMsg, InstantiateMsg, PositionResponse, QueryMsg,
};
use common::{
    ado_base::{
        recipient::{ADORecipient, Recipient},
        AndromedaMsg, AndromedaQuery,
    },
    error::ContractError,
    mission::AndrAddress,
    withdraw::{Withdrawal, WithdrawalType},
};
use cosmwasm_std::{
    attr, coin, coins, from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Api, BankMsg, Coin, ContractResult, CosmosMsg, Reply, Response, SubMsg,
    SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

fn deposit_stable_msg(amount: u128) -> SubMsg {
    SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "anchor_market".to_string(),
            msg: to_binary(&AnchorMarketMsg::DepositStable {}).unwrap(),
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
                msg: to_binary(&AnchorMarketMsg::RedeemStable {}).unwrap(),
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

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &[]);
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let res = instantiate(deps.as_mut(), env, info, msg.clone()).unwrap();

    assert_eq!(0, res.messages.len());

    let config = CONFIG.load(deps.as_ref().storage).unwrap();

    assert_eq!(
        msg.aust_token,
        deps.api
            .addr_humanize(&config.aust_token)
            .unwrap()
            .to_string()
    );
    assert_eq!(
        msg.anchor_market,
        deps.api
            .addr_humanize(&config.anchor_market)
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_deposit_and_withdraw_ust() {
    let mut deps = mock_dependencies_custom(&[]);
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let recipient = "recipient";

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::ADO(ADORecipient {
            address: AndrAddress {
                identifier: recipient.to_owned(),
            },
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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    let msg = InstantiateMsg {
        aust_token: "aust_token".to_string(),
        anchor_market: "anchor_market".to_string(),
    };
    let amount = 1000000u128;
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

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
