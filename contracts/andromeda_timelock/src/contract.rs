use cosmwasm_std::{
    attr, entry_point, to_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult,
};

use cw721::Expiration;

use crate::state::{State, STATE};
use andromeda_protocol::{
    modules::{
        address_list::{on_address_list_reply, AddressListModule, REPLY_ADDRESS_LIST},
        generate_instantiate_msgs,
        hooks::HookResponse,
    },
    modules::{hooks::MessageHooks, Module},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    timelock::{
        get_funds, hold_funds, release_funds, Escrow, ExecuteMsg, GetLockedFundsResponse,
        GetTimelockConfigResponse, InstantiateMsg, QueryMsg,
    },
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        address_list: msg.address_list.clone(),
    };

    let inst_msgs = generate_instantiate_msgs(&deps, info.clone(), env, vec![msg.address_list])?;

    STATE.save(deps.storage, &state)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender.to_string())?;
    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "instantiate"),
            attr("type", "timelock"),
        ])
        .add_submessages(inst_msgs.msgs)
        .add_events(inst_msgs.events))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    if msg.result.is_err() {
        return Err(StdError::generic_err(msg.result.unwrap_err()));
    }

    match msg.id {
        REPLY_ADDRESS_LIST => on_address_list_reply(deps, msg),
        _ => Err(StdError::generic_err("reply id is invalid")),
    }
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
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
        } => execute_hold_funds(deps, info, expiration, recipient, env),
        ExecuteMsg::ReleaseFunds {} => execute_release_funds(deps, env, info),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        ExecuteMsg::UpdateAddressList { address_list } => {
            execute_update_address_list(deps, info, env, address_list)
        }
    }
}

fn execute_hold_funds(
    deps: DepsMut,
    info: MessageInfo,
    expiration: Option<Expiration>,
    recipient: Option<String>,
    env: Env,
) -> StdResult<Response> {
    let rec = recipient.unwrap_or(info.sender.to_string());
    //Validate recipient address
    deps.api.addr_validate(&rec)?;

    let escrow = Escrow {
        coins: info.funds,
        expiration,
        recipient: rec,
    };
    //Adding clone for escrow here to allow for moving
    escrow.clone().validate(deps.api, &env.block)?;
    hold_funds(escrow.clone(), deps.storage, info.sender.to_string())?;
    let expiration_string = match escrow.expiration {
        Some(e) => e.to_string(),
        None => String::from("none"),
    };

    Ok(Response::default().add_attributes(vec![
        attr("action", "hold_funds"),
        attr("sender", info.sender.to_string()),
        attr("recipient", escrow.recipient),
        attr("expiration", expiration_string),
    ]))
}

fn execute_release_funds(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let result: Option<Escrow> = get_funds(deps.storage, info.sender.to_string())?;

    if result.is_none() {
        return Err(StdError::generic_err("No locked funds for your address"));
    }

    let funds: Escrow = result.unwrap();
    match funds.expiration {
        Some(expiration) => match expiration {
            Expiration::AtTime(t) => {
                if t > env.block.time {
                    return Err(StdError::generic_err("Your funds are still locked"));
                }
            }
            Expiration::AtHeight(h) => {
                if h > env.block.height {
                    return Err(StdError::generic_err("Your funds are still locked"));
                }
            }
            _ => {}
        },
        None => {}
    }

    let bank_msg = BankMsg::Send {
        to_address: funds.recipient.clone(),
        amount: funds.coins,
    };

    release_funds(deps.storage, info.sender.to_string())?;
    Ok(Response::new().add_message(bank_msg).add_attributes(vec![
        attr("action", "release_funds"),
        attr("recipient", funds.recipient),
    ]))
}

fn execute_update_address_list(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    address_list: Option<AddressListModule>,
) -> StdResult<Response> {
    let mut state = STATE.load(deps.storage)?;
    require(
        is_contract_owner(deps.storage, info.sender.to_string())?,
        StdError::generic_err("May only be used by the contract owner"),
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

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetLockedFunds { address } => to_binary(&query_held_funds(deps, address)?),
        QueryMsg::GetTimelockConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::ContractOwner {} => to_binary(&query_contract_owner(deps)?),
    }
}

fn query_held_funds(deps: Deps, address: String) -> StdResult<GetLockedFundsResponse> {
    let hold_funds = get_funds(deps.storage, address)?;
    Ok(GetLockedFundsResponse { funds: hold_funds })
}

fn query_config(deps: Deps) -> StdResult<GetTimelockConfigResponse> {
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
        coin, from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Coin,
    };

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
        let env = mock_env();
        let owner = "owner";
        let funds = vec![Coin::new(1000, "uusd")];
        let expiration = Expiration::AtHeight(1);
        let info = mock_info(owner, &funds.clone());
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();

        let msg = ExecuteMsg::HoldFunds {
            expiration: Some(expiration.clone()),
            recipient: None,
        };

        //add address for registered moderator

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "hold_funds"),
            attr("sender", info.sender.to_string()),
            attr("recipient", info.sender),
            attr("expiration", expiration.to_string()),
        ]);
        assert_eq!(expected, res);

        let query_msg = QueryMsg::GetLockedFunds {
            address: owner.to_string(),
        };

        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetLockedFundsResponse = from_binary(&res).unwrap();
        let expected = Escrow {
            coins: funds,
            expiration: Some(expiration),
            recipient: owner.to_string(),
        };

        assert_eq!(val.funds.unwrap(), expected);
    }

    #[test]
    fn test_execute_release_funds() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        let funds = vec![Coin::new(1000, "uusd")];
        STATE.save(deps.as_mut().storage, &mock_state()).unwrap();
        let info = mock_info(owner, &funds.clone());

        //test for Expiration::AtHeight(1)
        let msg = ExecuteMsg::HoldFunds {
            expiration: Some(Expiration::AtHeight(1)),
            recipient: None,
        };

        //add address for registered moderator
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let info = mock_info(owner, &vec![coin(100u128, "uluna")]);
        let msg = ExecuteMsg::ReleaseFunds {};
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let bank_msg = BankMsg::Send {
            to_address: owner.to_string(),
            amount: funds.clone(),
        };

        let expected = Response::default()
            .add_message(bank_msg)
            .add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient", info.sender.clone()),
            ]);

        assert_eq!(res, expected);

        //test when Expiration is none
        let info = mock_info(owner, &funds.clone());
        let msg = ExecuteMsg::HoldFunds {
            expiration: None,
            recipient: None,
        };

        //add address for registered moderator
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let info = mock_info(owner, &vec![coin(100u128, "uluna")]);
        let msg = ExecuteMsg::ReleaseFunds {};
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let bank_msg = BankMsg::Send {
            to_address: owner.to_string(),
            amount: funds,
        };

        let expected = Response::default()
            .add_message(bank_msg)
            .add_attributes(vec![
                attr("action", "release_funds"),
                attr("recipient", info.sender.clone()),
            ]);

        assert_eq!(res, expected);

        let msg = ExecuteMsg::HoldFunds {
            expiration: Some(Expiration::AtHeight(10000000)),
            recipient: None,
        };
        //add address for registered moderator
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::ReleaseFunds {};
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

        let expected = StdError::generic_err("Your funds are still locked");

        assert_eq!(res, expected);
    }

    #[test]
    fn test_execute_update_address_list() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "creator";

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &owner.to_string())
            .unwrap();

        let state = State { address_list: None };
        STATE.save(deps.as_mut().storage, &state).unwrap();

        let address_list = AddressListModule {
            address: Some(String::from("terra1contractaddress")),
            code_id: Some(1),
            moderators: Some(vec![String::from("moderator1")]),
            inclusive: true,
        };
        let msg = ExecuteMsg::UpdateAddressList {
            address_list: Some(address_list.clone()),
        };

        let unauth_info = mock_info("anyone", &[]);
        let err_res = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
        assert_eq!(
            err_res,
            StdError::generic_err("May only be used by the contract owner")
        );

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
}
