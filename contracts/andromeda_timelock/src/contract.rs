use cosmwasm_std::{
    attr, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, SubMsg,
};

use crate::state::{escrows, get_key, get_keys_for_recipient, State, STATE};
use ado_base::state::ADOContract;
use andromeda_protocol::{
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery, Recipient},
    error::ContractError,
    modules::{
        address_list::{on_address_list_reply, AddressListModule, REPLY_ADDRESS_LIST},
        generate_instantiate_msgs,
        hooks::HookResponse,
    },
    modules::{hooks::MessageHooks, Module},
    operators::{execute_update_operators, query_is_operator, query_operators},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    timelock::{
        Escrow, EscrowCondition, ExecuteMsg, GetLockedFundsForRecipientResponse,
        GetLockedFundsResponse, GetTimelockConfigResponse, InstantiateMsg, MigrateMsg, QueryMsg,
    },
};
use cw2::{get_contract_version, set_contract_version};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-timelock";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        address_list: msg.address_list.clone(),
    };

    let inst_msgs = generate_instantiate_msgs(&deps, info.clone(), env, vec![msg.address_list])?;

    STATE.save(deps.storage, &state)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "instantiate"),
            attr("type", "timelock"),
        ])
        .add_submessages(inst_msgs.msgs)
        .add_events(inst_msgs.events))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    match msg.id {
        REPLY_ADDRESS_LIST => on_address_list_reply(deps, msg),
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // [GLOBAL-02] Changing is_some() + .unwrap() to if let Some()
    if let Some(address_list) = state.address_list {
        let addr_list = address_list;
        addr_list.on_execute(&deps, info.clone(), env.clone())?;
    }
    let test = ADOContract::default();
    match msg {
        ExecuteMsg::HoldFunds {
            condition,
            recipient,
        } => execute_hold_funds(deps, info, env, condition, recipient),
        ExecuteMsg::ReleaseFunds {
            recipient_addr,
            start_after,
            limit,
        } => execute_release_funds(deps, env, info, recipient_addr, start_after, limit),
        ExecuteMsg::ReleaseSpecificFunds {
            owner,
            recipient_addr,
        } => execute_release_specific_funds(deps, env, info, owner, recipient_addr),
        ExecuteMsg::UpdateAddressList { address_list } => {
            execute_update_address_list(deps, info, env, address_list)
        }
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
    }
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let received: ExecuteMsg = parse_message(data)?;

            match received {
                ExecuteMsg::AndrReceive(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => execute(deps, env, info, received),
            }
        }
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
        AndromedaMsg::Withdraw { .. } => Err(ContractError::UnsupportedOperation {}),
    }
}

fn execute_hold_funds(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    condition: Option<EscrowCondition>,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let rec = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));

    //Validate recipient address
    let recipient_addr = rec.get_addr();
    deps.api.addr_validate(&recipient_addr)?;
    let mut escrow = Escrow {
        coins: info.funds,
        condition,
        recipient: rec,
    };
    let key = get_key(info.sender.as_str(), &recipient_addr);
    // Add funds to existing escrow if it exists.
    let existing_escrow = escrows().may_load(deps.storage, key.to_vec())?;
    if let Some(existing_escrow) = existing_escrow {
        // Keep the original condition.
        escrow.condition = existing_escrow.condition;
        escrow.add_funds(existing_escrow.coins);
    } else {
        // Only want to validate if the escrow doesn't exist already. This is because it might be
        // unlocked at this point, which is fine if funds are being added to it.
        escrow.validate(deps.api, &env.block)?;
    }
    escrows().save(deps.storage, key.to_vec(), &escrow)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "hold_funds"),
        attr("sender", info.sender),
        attr("recipient", format!("{:?}", escrow.recipient)),
        attr("condition", format!("{:?}", escrow.condition)),
    ]))
}

fn execute_release_funds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient_addr: Option<String>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let recipient_addr = recipient_addr.unwrap_or_else(|| info.sender.to_string());

    let keys = get_keys_for_recipient(deps.storage, &recipient_addr, start_after, limit);

    require(!keys.is_empty(), ContractError::NoLockedFunds {})?;

    let mut msgs: Vec<SubMsg> = vec![];
    for key in keys.iter() {
        let funds: Escrow = escrows().load(deps.storage, key.clone())?;
        if !funds.is_locked(&env.block)? {
            let msg = funds.recipient.generate_msg_native(deps.api, funds.coins)?;
            msgs.push(msg);
            escrows().remove(deps.storage, key.clone())?;
        }
    }

    require(!msgs.is_empty(), ContractError::FundsAreLocked {})?;

    Ok(Response::new().add_submessages(msgs).add_attributes(vec![
        attr("action", "release_funds"),
        attr("recipient_addr", recipient_addr),
    ]))
}

fn execute_release_specific_funds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let recipient = recipient.unwrap_or_else(|| info.sender.to_string());
    let key = get_key(&owner, &recipient);
    let escrow = escrows().may_load(deps.storage, key.clone())?;
    match escrow {
        None => Err(ContractError::NoLockedFunds {}),
        Some(escrow) => {
            require(
                !escrow.is_locked(&env.block)?,
                ContractError::FundsAreLocked {},
            )?;
            escrows().remove(deps.storage, key)?;
            let msg = escrow
                .recipient
                .generate_msg_native(deps.api, escrow.coins)?;
            Ok(Response::new().add_submessage(msg).add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient_addr", recipient),
            ]))
        }
    }
}

fn execute_update_address_list(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    address_list: Option<AddressListModule>,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let mod_resp = match address_list.clone() {
        None => HookResponse::default(),
        Some(addr_list) => addr_list.on_instantiate(&deps, info, env)?,
    };
    state.address_list = address_list;

    STATE.save(deps.storage, &state)?;

    Ok(Response::default()
        .add_submessages(mod_resp.msgs)
        .add_events(mod_resp.events)
        .add_attributes(vec![attr("action", "update_address_list")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let contract = ADOContract::default();
    match msg {
        QueryMsg::GetLockedFunds { owner, recipient } => {
            encode_binary(&query_held_funds(deps, owner, recipient)?)
        }
        QueryMsg::GetLockedFundsForRecipient {
            recipient,
            start_after,
            limit,
        } => encode_binary(&query_funds_for_recipient(
            deps,
            recipient,
            start_after,
            limit,
        )?),
        QueryMsg::GetTimelockConfig {} => encode_binary(&query_config(deps)?),
        QueryMsg::AndrQuery(msg) => contract.query(deps, env, msg, query),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let received: QueryMsg = parse_message(data)?;
            match received {
                QueryMsg::AndrQuery(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => query(deps, env, received),
            }
        }
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
        AndromedaQuery::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, &address)?)
        }
    }
}

fn query_funds_for_recipient(
    deps: Deps,
    recipient: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<GetLockedFundsForRecipientResponse, ContractError> {
    let keys = get_keys_for_recipient(deps.storage, &recipient, start_after, limit);
    let mut recipient_escrows: Vec<Escrow> = vec![];
    for key in keys.iter() {
        recipient_escrows.push(escrows().load(deps.storage, key.to_vec())?);
    }
    Ok(GetLockedFundsForRecipientResponse {
        funds: recipient_escrows,
    })
}

fn query_held_funds(
    deps: Deps,
    owner: String,
    recipient: String,
) -> Result<GetLockedFundsResponse, ContractError> {
    let hold_funds = escrows().may_load(deps.storage, get_key(&owner, &recipient))?;
    Ok(GetLockedFundsResponse { funds: hold_funds })
}

fn query_config(deps: Deps) -> Result<GetTimelockConfigResponse, ContractError> {
    let state = STATE.load(deps.storage)?;

    let address_list_contract = match state.address_list.clone() {
        None => None,
        Some(addr_list) => addr_list.get_contract_address(deps.storage),
    };

    Ok(GetTimelockConfigResponse {
        address_list: state.address_list,
        address_list_contract,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        coin, coins, from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, BankMsg, Coin, Timestamp,
    };
    use cw721::Expiration;

    fn mock_state() -> State {
        State { address_list: None }
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg { address_list: None };
        let res = instantiate(deps.as_mut(), env, info, msg.clone()).unwrap();

        assert_eq!(0, res.messages.len());

        //checking
        let state = STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(msg.address_list, state.address_list);
    }

    #[test]
    fn test_execute_hold_funds() {
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        let owner = "owner";
        let funds = vec![Coin::new(1000, "uusd")];
        let condition = EscrowCondition::Expiration(Expiration::AtHeight(1));
        let info = mock_info(owner, &funds);
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let msg = ExecuteMsg::HoldFunds {
            condition: Some(condition.clone()),
            recipient: None,
        };
        env.block.height = 0;

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "hold_funds"),
            attr("sender", info.sender.to_string()),
            attr(
                "recipient",
                format!("{:?}", Recipient::Addr(info.sender.to_string())),
            ),
            attr("condition", format!("{:?}", Some(condition.clone()))),
        ]);
        assert_eq!(expected, res);

        let query_msg = QueryMsg::GetLockedFunds {
            owner: owner.to_string(),
            recipient: owner.to_string(),
        };

        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetLockedFundsResponse = from_binary(&res).unwrap();
        let expected = Escrow {
            coins: funds,
            condition: Some(condition),
            recipient: Recipient::Addr(owner.to_string()),
        };

        assert_eq!(val.funds.unwrap(), expected);
    }

    #[test]
    fn test_execute_hold_funds_escrow_updated() {
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();

        let owner = "owner";
        let info = mock_info(owner, &coins(100, "uusd"));
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let msg = ExecuteMsg::HoldFunds {
            condition: Some(EscrowCondition::Expiration(Expiration::AtHeight(10))),
            recipient: Some(Recipient::Addr("recipient".into())),
        };

        env.block.height = 0;

        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::HoldFunds {
            condition: Some(EscrowCondition::Expiration(Expiration::AtHeight(100))),
            recipient: Some(Recipient::Addr("recipient".into())),
        };

        env.block.height = 120;

        let info = mock_info(owner, &[coin(100, "uusd"), coin(100, "uluna")]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let query_msg = QueryMsg::GetLockedFunds {
            owner: owner.to_string(),
            recipient: "recipient".to_string(),
        };

        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetLockedFundsResponse = from_binary(&res).unwrap();
        let expected = Escrow {
            // Coins get merged.
            coins: vec![coin(200, "uusd"), coin(100, "uluna")],
            // Original expiration remains.
            condition: Some(EscrowCondition::Expiration(Expiration::AtHeight(10))),
            recipient: Recipient::Addr("recipient".to_string()),
        };

        assert_eq!(val.funds.unwrap(), expected);
    }

    #[test]
    fn test_execute_release_funds_block_condition() {
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            condition: Some(EscrowCondition::Expiration(Expiration::AtHeight(1))),
            recipient: None,
        };
        env.block.height = 0;
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        env.block.height = 2;
        let msg = ExecuteMsg::ReleaseFunds {
            recipient_addr: None,
            start_after: None,
            limit: None,
        };
        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
        let bank_msg = BankMsg::Send {
            to_address: "owner".into(),
            amount: info.funds,
        };
        assert_eq!(
            Response::new().add_message(bank_msg).add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient_addr", "owner"),
            ]),
            res
        );
    }

    #[test]
    fn test_execute_release_funds_no_condition() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
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
            to_address: "owner".into(),
            amount: info.funds,
        };
        assert_eq!(
            Response::new().add_message(bank_msg).add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient_addr", "owner"),
            ]),
            res
        );
    }

    #[test]
    fn test_execute_release_multiple_escrows() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let recipient = Recipient::Addr("recipient".into());
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let msg = ExecuteMsg::HoldFunds {
            condition: None,
            recipient: Some(recipient),
        };
        let info = mock_info("sender1", &coins(100, "uusd"));
        let _res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

        let info = mock_info("sender2", &coins(200, "uusd"));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ReleaseFunds {
            recipient_addr: Some("recipient".into()),
            start_after: None,
            limit: None,
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let bank_msg1 = BankMsg::Send {
            to_address: "recipient".into(),
            amount: coins(100, "uusd"),
        };
        let bank_msg2 = BankMsg::Send {
            to_address: "recipient".into(),
            amount: coins(200, "uusd"),
        };
        assert_eq!(
            Response::new()
                .add_messages(vec![bank_msg1, bank_msg2])
                .add_attributes(vec![
                    attr("action", "release_funds"),
                    attr("recipient_addr", "recipient"),
                ]),
            res
        );
    }

    #[test]
    fn test_execute_release_funds_time_condition() {
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            condition: Some(EscrowCondition::Expiration(Expiration::AtTime(
                Timestamp::from_seconds(100),
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
            to_address: "owner".into(),
            amount: info.funds,
        };
        assert_eq!(
            Response::new().add_message(bank_msg).add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient_addr", "owner"),
            ]),
            res
        );
    }

    #[test]
    fn test_execute_release_funds_locked() {
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            condition: Some(EscrowCondition::Expiration(Expiration::AtTime(
                Timestamp::from_seconds(100),
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
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            condition: Some(EscrowCondition::MinimumFunds(vec![
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
        let info = mock_info(owner, &[coin(110, "uusd"), coin(120, "uluna")]);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Now try to release funds.
        let msg = ExecuteMsg::ReleaseFunds {
            recipient_addr: None,
            start_after: None,
            limit: None,
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let bank_msg = BankMsg::Send {
            to_address: "owner".into(),
            amount: vec![coin(210, "uusd"), coin(120, "uluna")],
        };
        assert_eq!(
            Response::new().add_message(bank_msg).add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient_addr", "owner"),
            ]),
            res
        );
    }

    #[test]
    fn test_execute_release_specific_funds_no_funds_locked() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[]);
        let msg = ExecuteMsg::ReleaseSpecificFunds {
            recipient_addr: None,
            owner: owner.into(),
        };
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::NoLockedFunds {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_release_specific_funds_no_condition() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            condition: None,
            recipient: None,
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ReleaseSpecificFunds {
            recipient_addr: None,
            owner: owner.into(),
        };
        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
        let bank_msg = BankMsg::Send {
            to_address: "owner".into(),
            amount: info.funds,
        };
        assert_eq!(
            Response::new().add_message(bank_msg).add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient_addr", "owner"),
            ]),
            res
        );
    }

    #[test]
    fn test_execute_release_specific_funds_time_condition() {
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            condition: Some(EscrowCondition::Expiration(Expiration::AtTime(
                Timestamp::from_seconds(100),
            ))),
            recipient: None,
        };
        env.block.time = Timestamp::from_seconds(50);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ReleaseSpecificFunds {
            recipient_addr: None,
            owner: owner.into(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
        let bank_msg = BankMsg::Send {
            to_address: "owner".into(),
            amount: info.funds,
        };
        assert_eq!(
            Response::new().add_message(bank_msg).add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient_addr", "owner"),
            ]),
            res
        );
    }

    #[test]
    fn test_execute_release_specific_funds_min_funds_condition() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            condition: Some(EscrowCondition::MinimumFunds(vec![
                coin(200, "uusd"),
                coin(100, "uluna"),
            ])),
            recipient: None,
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ReleaseSpecificFunds {
            recipient_addr: None,
            owner: owner.into(),
        };

        let res = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(ContractError::FundsAreLocked {}, res.unwrap_err());

        // Update the escrow with enough funds.
        let msg = ExecuteMsg::HoldFunds {
            condition: None,
            recipient: None,
        };
        let info = mock_info(owner, &[coin(110, "uusd"), coin(120, "uluna")]);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Now try to release funds.
        let msg = ExecuteMsg::ReleaseSpecificFunds {
            recipient_addr: None,
            owner: owner.into(),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let bank_msg = BankMsg::Send {
            to_address: "owner".into(),
            amount: vec![coin(210, "uusd"), coin(120, "uluna")],
        };
        assert_eq!(
            Response::new().add_message(bank_msg).add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient_addr", "owner"),
            ]),
            res
        );
    }

    #[test]
    fn test_execute_update_address_list() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "creator";

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner.to_string()))
            .unwrap();

        let state = State { address_list: None };
        STATE.save(deps.as_mut().storage, &state).unwrap();

        let address_list = AddressListModule {
            address: Some(String::from("terra1contractaddress")),
            code_id: Some(1),
            operators: Some(vec![String::from("operator1")]),
            inclusive: true,
        };
        let msg = ExecuteMsg::UpdateAddressList {
            address_list: Some(address_list.clone()),
        };

        let unauth_info = mock_info("anyone", &[]);
        let err_res = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
        assert_eq!(err_res, ContractError::Unauthorized {});

        let info = mock_info(owner, &[]);
        let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let mod_resp = address_list
            .on_instantiate(&deps.as_mut(), info, env)
            .unwrap();
        let expected = Response::default()
            .add_submessages(mod_resp.msgs)
            .add_events(mod_resp.events)
            .add_attributes(vec![attr("action", "update_address_list")]);

        assert_eq!(resp, expected);

        let updated = STATE.load(deps.as_mut().storage).unwrap();

        assert_eq!(updated.address_list.unwrap(), address_list);
    }

    #[test]
    fn test_execute_receive() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        let funds = vec![Coin::new(1000, "uusd")];
        let info = mock_info(owner, &funds);
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let msg_struct = ExecuteMsg::HoldFunds {
            condition: None,
            recipient: None,
        };
        let msg_string = encode_binary(&msg_struct).unwrap();

        let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(msg_string)));

        let received = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "hold_funds"),
            attr("sender", info.sender.to_string()),
            attr("recipient", "Addr(\"owner\")"),
            attr("condition", "None"),
        ]);

        assert_eq!(expected, received)
    }
}
