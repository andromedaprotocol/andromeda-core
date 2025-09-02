use crate::{
    contract::{execute, instantiate, query},
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_finance::timelock::InstantiateMsg;
use andromeda_finance::timelock::{
    Escrow, EscrowCondition, EscrowConditionInput, ExecuteMsg, GetLockedFundsResponse, QueryMsg,
};
use andromeda_std::{
    amp::Recipient,
    common::{expiration::Expiry, Milliseconds},
    error::ContractError,
    testing::{mock_querier::MOCK_KERNEL_CONTRACT, utils::assert_response},
};
use cosmwasm_std::{
    attr, coin, coins, from_json,
    testing::{message_info, mock_env},
    Addr, BankMsg, Coin, Response, Timestamp,
};

use super::mock_querier::TestDeps;

const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";

fn init(deps: &mut TestDeps) -> Response {
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap()
}
#[test]
fn test_execute_hold_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let env = mock_env();
    let funds = vec![Coin::new(1000u128, "uusd")];
    let condition = EscrowConditionInput::Expiration(Expiry::AtTime(Milliseconds::from_seconds(
        env.block.time.seconds() + 1,
    )));
    let info = message_info(&Addr::unchecked(OWNER), &funds);

    let msg = ExecuteMsg::HoldFunds {
        condition: Some(condition.clone()),
        recipient: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    let expected = Response::default().add_attributes(vec![
        attr("action", "hold_funds"),
        attr("sender", info.sender.to_string()),
        attr(
            "recipient",
            format!("{:?}", Recipient::from_string(info.sender.to_string())),
        ),
        attr(
            "condition",
            format!("{:?}", Some(condition.clone().to_condition(&env.block))),
        ),
    ]);
    assert_response(&res, &expected, "timelock_execute_hold_funds");

    let query_msg = QueryMsg::GetLockedFunds {
        owner: OWNER.to_string(),
        recipient: OWNER.to_string(),
    };

    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let val: GetLockedFundsResponse = from_json(res).unwrap();
    let expected = Escrow {
        coins: funds,
        condition: Some(condition.to_condition(&env.block)),
        recipient: Recipient::from_string(OWNER.to_string()),
        recipient_addr: OWNER.to_string(),
    };

    assert_eq!(val.funds.unwrap(), expected);
}

#[test]
fn test_execute_hold_funds_escrow_updated() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let mut env = mock_env();

    let info = message_info(&Addr::unchecked(OWNER), &coins(100, "uusd"));

    let recipient = deps.api.addr_make("recipient");
    let msg = ExecuteMsg::HoldFunds {
        condition: Some(EscrowConditionInput::Expiration(Expiry::AtTime(
            Milliseconds::from_seconds(env.block.time.seconds() + 1),
        ))),
        recipient: Some(Recipient::from_string(recipient.to_string())),
    };

    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::HoldFunds {
        condition: Some(EscrowConditionInput::Expiration(Expiry::AtTime(
            Milliseconds::from_seconds(env.block.time.seconds() + 1),
        ))),
        recipient: Some(Recipient::from_string(recipient.to_string())),
    };

    env.block.time = Milliseconds::from_seconds(env.block.time.seconds())
        .plus_seconds(1)
        .into();

    let info = message_info(
        &Addr::unchecked(OWNER),
        &[coin(100, "uusd"), coin(100, "uluna")],
    );
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let query_msg = QueryMsg::GetLockedFunds {
        owner: OWNER.to_string(),
        recipient: recipient.to_string(),
    };

    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let val: GetLockedFundsResponse = from_json(res).unwrap();
    let expected = Escrow {
        // Coins get merged.
        coins: vec![coin(200, "uusd"), coin(100, "uluna")],
        // Original expiration remains.
        condition: Some(EscrowCondition::Expiration(Milliseconds::from_seconds(
            env.block.time.seconds(),
        ))),
        recipient: Recipient::from_string(recipient.to_string()),
        recipient_addr: recipient.to_string(),
    };

    assert_eq!(val.funds.unwrap(), expected);
}

#[test]
fn test_execute_release_funds_no_condition() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let env = mock_env();

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        condition: None,
        recipient: None,
    };
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ReleaseFunds {
        recipient_addr: None,
        start_after: None,
        limit: None,
    };
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let bank_msg = BankMsg::Send {
        to_address: OWNER.into(),
        amount: info.funds,
    };
    let expected_res: Response = Response::new()
        .add_message(bank_msg)
        .add_attribute("action", "release_funds")
        .add_attribute("recipient_addr", OWNER);
    assert_response(&res, &expected_res, "timelock_release_funds");
}

#[test]
fn test_execute_release_multiple_escrows() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let env = mock_env();
    let recipient_addr = deps.api.addr_make("recipient");
    let recipient = Recipient::from_string(recipient_addr.to_string());

    let msg = ExecuteMsg::HoldFunds {
        condition: None,
        recipient: Some(recipient),
    };
    let sender1 = deps.api.addr_make("sender1");
    let info = message_info(&Addr::unchecked(sender1), &coins(100, "uusd"));
    let _res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

    let sender2 = deps.api.addr_make("sender2");
    let info = message_info(&Addr::unchecked(sender2), &coins(200, "uusd"));
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ReleaseFunds {
        recipient_addr: Some(recipient_addr.to_string()),
        start_after: None,
        limit: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let bank_msg1 = BankMsg::Send {
        to_address: recipient_addr.to_string(),
        amount: coins(100, "uusd"),
    };
    let bank_msg2 = BankMsg::Send {
        to_address: recipient_addr.to_string(),
        amount: coins(200, "uusd"),
    };
    let expected_res: Response = Response::new()
        .add_messages(vec![bank_msg1, bank_msg2])
        .add_attributes(vec![
            attr("action", "release_funds"),
            attr("recipient_addr", recipient_addr.to_string()),
        ]);
    assert_response(&res, &expected_res, "timelock_release_multiple_escrows");
}

#[test]
fn test_execute_release_funds_time_condition() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let mut env = mock_env();
    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        condition: Some(EscrowConditionInput::Expiration(Expiry::AtTime(
            Milliseconds::from_seconds(100),
        ))),
        recipient: None,
    };
    env.block.time = Timestamp::from_seconds(50);
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ReleaseFunds {
        recipient_addr: None,
        start_after: None,
        limit: None,
    };

    env.block.time = Timestamp::from_seconds(150);
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let bank_msg = BankMsg::Send {
        to_address: OWNER.into(),
        amount: info.funds,
    };
    let expected_res: Response = Response::new()
        .add_message(bank_msg)
        .add_attribute("action", "release_funds")
        .add_attribute("recipient_addr", OWNER);
    assert_response(&res, &expected_res, "timelock_release_funds_time_condition");
}

#[test]
fn test_execute_release_funds_locked() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        condition: Some(EscrowConditionInput::Expiration(Expiry::FromNow(
            Milliseconds::from_seconds(100),
        ))),
        recipient: None,
    };
    env.block.time = Timestamp::from_seconds(50);
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ReleaseFunds {
        recipient_addr: None,
        start_after: None,
        limit: None,
    };

    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::FundsAreLocked {}, res.unwrap_err());
}

#[test]
fn test_execute_release_funds_min_funds_condition() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let env = mock_env();

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        condition: Some(EscrowConditionInput::MinimumFunds(vec![
            coin(200, "uusd"),
            coin(100, "uluna"),
        ])),
        recipient: None,
    };
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ReleaseFunds {
        recipient_addr: None,
        start_after: None,
        limit: None,
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::FundsAreLocked {}, res.unwrap_err());

    // Update the escrow with enough funds.
    let msg = ExecuteMsg::HoldFunds {
        condition: None,
        recipient: None,
    };
    let info = message_info(
        &Addr::unchecked(OWNER),
        &[coin(110, "uusd"), coin(120, "uluna")],
    );
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Now try to release funds.
    let msg = ExecuteMsg::ReleaseFunds {
        recipient_addr: None,
        start_after: None,
        limit: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let bank_msg = BankMsg::Send {
        to_address: OWNER.into(),
        amount: vec![coin(210, "uusd"), coin(120, "uluna")],
    };
    let expected_res: Response = Response::new()
        .add_message(bank_msg)
        .add_attribute("action", "release_funds")
        .add_attribute("recipient_addr", OWNER);
    assert_response(
        &res,
        &expected_res,
        "timelock_release_funds_min_funds_condition",
    );
}

#[test]
fn test_execute_release_specific_funds_no_funds_locked() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let env = mock_env();

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let msg = ExecuteMsg::ReleaseSpecificFunds {
        recipient_addr: None,
        owner: OWNER.into(),
    };
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::NoLockedFunds {}, res.unwrap_err());
}

#[test]
fn test_execute_release_specific_funds_no_condition() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let env = mock_env();

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        condition: None,
        recipient: None,
    };
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ReleaseSpecificFunds {
        recipient_addr: None,
        owner: OWNER.into(),
    };
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let bank_msg = BankMsg::Send {
        to_address: OWNER.into(),
        amount: info.funds,
    };
    let expected_res: Response = Response::new()
        .add_message(bank_msg)
        .add_attribute("action", "release_funds")
        .add_attribute("recipient_addr", OWNER);
    assert_response(
        &res,
        &expected_res,
        "timelock_release_specific_funds_no_condition",
    );
}

#[test]
fn test_execute_release_specific_funds_time_condition() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let mut env = mock_env();

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        condition: Some(EscrowConditionInput::Expiration(Expiry::AtTime(
            Milliseconds::from_seconds(100),
        ))),
        recipient: None,
    };
    env.block.time = Timestamp::from_seconds(50);
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ReleaseSpecificFunds {
        recipient_addr: None,
        owner: OWNER.into(),
    };

    env.block.time = Timestamp::from_seconds(150);
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let bank_msg = BankMsg::Send {
        to_address: OWNER.into(),
        amount: info.funds,
    };
    let expected_res: Response = Response::new()
        .add_message(bank_msg)
        .add_attribute("action", "release_funds")
        .add_attribute("recipient_addr", OWNER);
    assert_response(
        &res,
        &expected_res,
        "timelock_release_specific_funds_time_condition",
    );
}

#[test]
fn test_execute_release_specific_funds_min_funds_condition() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let env = mock_env();

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        condition: Some(EscrowConditionInput::MinimumFunds(vec![
            coin(200, "uusd"),
            coin(100, "uluna"),
        ])),
        recipient: None,
    };
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ReleaseSpecificFunds {
        recipient_addr: None,
        owner: OWNER.into(),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::FundsAreLocked {}, res.unwrap_err());

    // Update the escrow with enough funds.
    let msg = ExecuteMsg::HoldFunds {
        condition: None,
        recipient: None,
    };
    let info = message_info(
        &Addr::unchecked(OWNER),
        &[coin(110, "uusd"), coin(120, "uluna")],
    );
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Now try to release funds.
    let msg = ExecuteMsg::ReleaseSpecificFunds {
        recipient_addr: None,
        owner: OWNER.into(),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let bank_msg = BankMsg::Send {
        to_address: OWNER.into(),
        amount: vec![coin(210, "uusd"), coin(120, "uluna")],
    };
    let expected_res: Response = Response::new()
        .add_message(bank_msg)
        .add_attribute("action", "release_funds")
        .add_attribute("recipient_addr", OWNER);
    assert_response(
        &res,
        &expected_res,
        "timelock_release_specific_funds_min_funds_condition",
    );
}

// #[test]
// fn test_execute_receive() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let owner = "owner";
//     let funds = vec![Coin::new(1000, "uusd")];
//     let info = message_info(&Addr::unchecked(owner), &funds);

//     let msg_struct = ExecuteMsg::HoldFunds {
//         condition: None,
//         recipient: None,
//     };
//     let msg_string = encode_binary(&msg_struct).unwrap();

//     let msg = ExecuteMsg::Receive(Some(msg_string));

//     let received = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
//     let expected = Response::default().add_attributes(vec![
//         attr("action", "hold_funds"),
//         attr("sender", info.sender.to_string()),
//         attr("recipient", "Addr(\"owner\")"),
//         attr("condition", "None"),
//     ]);

//     assert_eq!(expected, received)
// }
