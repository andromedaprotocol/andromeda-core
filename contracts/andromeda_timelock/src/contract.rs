use cosmwasm_std::{
    attr, entry_point, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response,
    StdError, SubMsg,
};

use cw_storage_plus::Bound;

use crate::state::{escrows, State, STATE};
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
        Escrow, EscrowCondition, ExecuteMsg, GetLockedFundsResponse, GetTimelockConfigResponse,
        InstantiateMsg, QueryMsg,
    },
};

const DEFAULT_LIMIT: u32 = 10u32;
const MAX_LIMIT: u32 = 30u32;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
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

    match msg {
        ExecuteMsg::HoldFunds {
            expiration,
            recipient,
        } => execute_hold_funds(deps, info, env, expiration, recipient),
        ExecuteMsg::ReleaseFunds {
            recipient_addr,
            start_after,
            limit,
        } => execute_release_funds(deps, env, info, recipient_addr, start_after, limit),
        ExecuteMsg::UpdateAddressList { address_list } => {
            execute_update_address_list(deps, info, env, address_list)
        }
        ExecuteMsg::AndrReceive(msg) => execute_receive(deps, env, info, msg),
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
    }
}

fn execute_hold_funds(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    expiration: Option<EscrowCondition>,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let rec = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));

    //Validate recipient address
    let recipient_addr = rec.get_addr();
    deps.api.addr_validate(&recipient_addr)?;
    let mut escrow = Escrow {
        coins: info.funds,
        expiration,
        recipient: rec,
    };
    let key = get_key(info.sender.as_str(), &recipient_addr);
    // Add funds to existing escrow if it exists.
    let existing_escrow = escrows().may_load(deps.storage, key.to_vec())?;
    if let Some(existing_escrow) = existing_escrow {
        merge_coins(&mut escrow.coins, existing_escrow.coins);
    }

    escrows().save(deps.storage, key.to_vec(), &escrow)?;
    escrow.validate(deps.api, &env.block)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "hold_funds"),
        attr("sender", info.sender),
        attr("recipient", format!("{:?}", escrow.recipient)),
        attr("expiration", format!("{:?}", escrow.expiration)),
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
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let pks: Vec<_> = escrows()
        .idx
        .owner
        .prefix(recipient_addr.clone())
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect();

    let keys: Vec<Vec<u8>> = pks.iter().map(|v| v.to_vec()).collect();

    if keys.is_empty() {
        return Err(ContractError::NoLockedFunds {});
    }

    let mut msgs: Vec<SubMsg> = vec![];
    for key in keys.iter() {
        let funds: Escrow = escrows().load(deps.storage, key.clone())?;
        if funds.is_unlocked(&env.block)? {
            let msg = funds.recipient.generate_msg(&deps, funds.coins)?;
            msgs.push(msg);
            escrows().remove(deps.storage, key.clone())?;
        }
    }

    if msgs.is_empty() {
        return Err(ContractError::FundsAreLocked {});
    }

    Ok(Response::new().add_submessages(msgs).add_attributes(vec![
        attr("action", "release_funds"),
        attr("recipient_addr", recipient_addr),
    ]))
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

/// Merges the coins of the same denom into vector `a` in place.
///
/// ## Arguments
/// * `a` - The `Vec<Coin>` that gets modified in place
/// * `b` - The `Vec<Coin>` that is merged into `a`
///
/// Returns nothing as it is done in place.
fn merge_coins(a: &mut Vec<Coin>, b: Vec<Coin>) {
    for a_coin in a.iter_mut() {
        let same_denom_coin = b.iter().find(|&c| c.denom == a_coin.denom);
        if let Some(b_coin) = same_denom_coin {
            a_coin.amount += b_coin.amount;
        }
    }
    for b_coin in b.iter() {
        if !a.iter().any(|c| c.denom == b_coin.denom) {
            a.push(b_coin.clone());
        }
    }
}

fn get_key(sender: &str, recipient: &str) -> Vec<u8> {
    vec![sender.as_bytes(), recipient.as_bytes()].concat()
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetLockedFunds { owner, recipient } => {
            encode_binary(&query_held_funds(deps, owner, recipient)?)
        }
        QueryMsg::GetTimelockConfig {} => encode_binary(&query_config(deps)?),
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
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

fn query_held_funds(
    deps: Deps,
    address: String,
    recipient: String,
) -> Result<GetLockedFundsResponse, ContractError> {
    let hold_funds = escrows().may_load(deps.storage, get_key(&address, &recipient))?;
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
        let expiration = EscrowCondition::Expiration(Expiration::AtHeight(1));
        let info = mock_info(owner, &funds);
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let msg = ExecuteMsg::HoldFunds {
            expiration: Some(expiration.clone()),
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
            attr("expiration", format!("{:?}", Some(expiration.clone()))),
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
            expiration: Some(expiration.clone()),
            recipient: Recipient::Addr(owner.to_string()),
        };

        assert_eq!(val.funds.unwrap(), expected);
    }

    #[test]
    fn test_execute_hold_funds_escrow_updated() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let owner = "owner";
        let info = mock_info(owner, &coins(100, "uusd"));
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let msg = ExecuteMsg::HoldFunds {
            expiration: None,
            recipient: None,
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd"), coin(100, "uluna")]);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let query_msg = QueryMsg::GetLockedFunds {
            owner: owner.to_string(),
            recipient: owner.to_string(),
        };

        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetLockedFundsResponse = from_binary(&res).unwrap();
        let expected = Escrow {
            // Coins get merged.
            coins: vec![coin(200, "uusd"), coin(100, "uluna")],
            expiration: None,
            recipient: Recipient::Addr(owner.to_string()),
        };

        assert_eq!(val.funds.unwrap(), expected);
    }

    #[test]
    fn test_execute_release_funds_block_expiration() {
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            expiration: Some(EscrowCondition::Expiration(Expiration::AtHeight(1))),
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
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
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
    fn test_execute_release_funds_no_expiration() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            expiration: None,
            recipient: None,
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ReleaseFunds {
            recipient_addr: None,
            start_after: None,
            limit: None,
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
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
            expiration: None,
            recipient: Some(recipient),
        };
        let info = mock_info("sender1", &coins(100, "uusd"));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let info = mock_info("sender2", &coins(200, "uusd"));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::ReleaseFunds {
            recipient_addr: Some("recipient".into()),
            start_after: None,
            limit: None,
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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
    fn test_execute_release_funds_time_expiration() {
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        let owner = "owner";
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let info = mock_info(owner, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::HoldFunds {
            expiration: Some(EscrowCondition::Expiration(Expiration::AtTime(
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
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
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
            expiration: Some(EscrowCondition::Expiration(Expiration::AtTime(
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

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
        assert_eq!(ContractError::FundsAreLocked {}, res.unwrap_err());
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
            expiration: None,
            recipient: None,
        };
        let msg_string = encode_binary(&msg_struct).unwrap();

        let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(msg_string)));

        let received = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "hold_funds"),
            attr("sender", info.sender.to_string()),
            attr("recipient", "Addr(\"owner\")"),
            attr("expiration", "None"),
        ]);

        assert_eq!(expected, received)
    }
}
