use crate::state::{State, SPLITTER, STATE};
use andromeda_protocol::modules::address_list::{
    on_address_list_reply, AddressListModule, REPLY_ADDRESS_LIST,
};
use andromeda_protocol::modules::hooks::MessageHooks;
use andromeda_protocol::modules::Module;
use andromeda_protocol::splitter::GetSplitterConfigResponse;
use andromeda_protocol::{
    require::require,
    splitter::{
        validate_recipient_list, AddressPercent, ExecuteMsg, InstantiateMsg, QueryMsg, Splitter,
    },
};
use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg,
};
// use std::collections::HashMap;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    msg.validate()?;
    let state = State {
        owner: info.clone().sender,
    };

    let splitter = Splitter {
        recipients: msg.recipients,
        locked: false,
        address_list: msg.address_list.clone(),
    };

    let mut res = Response::default();

    if msg.address_list.is_some() {
        let addr_res =
            msg.address_list
                .clone()
                .unwrap()
                .on_instantiate(&deps, info.clone(), env.clone())?;
        res = res.add_submessages(addr_res.msgs);
    }

    STATE.save(deps.storage, &state)?;
    SPLITTER.save(deps.storage, &splitter)?;
    Ok(res)
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let splitter = SPLITTER.load(deps.storage)?;

    if splitter.address_list.is_some() {
        let addr_list = splitter.address_list.unwrap();
        addr_list.on_execute(&deps, info.clone(), env.clone())?;
    }

    match msg {
        ExecuteMsg::UpdateRecipients { recipients } => {
            execute_update_recipients(deps, info, recipients)
        }
        ExecuteMsg::UpdateLock { lock } => execute_update_lock(deps, info, lock),
        ExecuteMsg::UpdateAddressList { address_list } => {
            execute_update_address_list(deps, info, address_list)
        }
        ExecuteMsg::Send {} => execute_send(deps, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        REPLY_ADDRESS_LIST => on_address_list_reply(deps, msg),
        _ => Err(StdError::generic_err("reply id is invalid")),
    }
}

fn execute_send(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let sent_funds: Vec<Coin> = info.funds.clone();

    require(sent_funds.len() > 0, StdError::generic_err("No coin sent"))?;

    let splitter = SPLITTER.load(deps.storage)?;

    if splitter.address_list.is_some() {
        splitter
            .address_list
            .unwrap()
            .is_authorized(&deps, info.sender.to_string())?;
    }

    require(
        splitter.recipients.len() > 0,
        StdError::generic_err("No recipient received"),
    )?;

    let mut submsg: Vec<SubMsg> = Vec::new();

    for recipient_addr in splitter.recipients {
        let recipient_percent = recipient_addr.percent;
        let mut vec_coin: Vec<Coin> = Vec::new();
        for coin in &sent_funds {
            let mut recip_coin: Coin = coin.clone();
            recip_coin.amount = coin.amount.multiply_ratio(recipient_percent, 100 as u128);
            vec_coin.push(recip_coin);
        }
        submsg.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient_addr.addr,
            amount: vec_coin,
        })));
    }

    Ok(Response::new().add_submessages(submsg))
}

fn execute_update_recipients(
    deps: DepsMut,
    info: MessageInfo,
    recipients: Vec<AddressPercent>,
) -> StdResult<Response> {
    let state = STATE.load(deps.storage)?;
    require(
        state.owner == info.sender,
        StdError::generic_err("May only be used by the contract owner"),
    )?;

    validate_recipient_list(recipients.clone())?;

    let mut splitter = SPLITTER.load(deps.storage)?;

    if splitter.locked == true {
        StdError::generic_err("Not allow to change recipient");
    }

    splitter.recipients.clear();

    splitter.recipients = recipients.clone();
    SPLITTER.save(deps.storage, &splitter)?;
    Ok(Response::default())
}

fn execute_update_lock(deps: DepsMut, info: MessageInfo, lock: bool) -> StdResult<Response> {
    let state = STATE.load(deps.storage)?;
    require(
        state.owner == info.sender,
        StdError::generic_err("May only be used by the contract owner"),
    )?;
    let mut splitter = SPLITTER.load(deps.storage)?;
    splitter.locked = lock;
    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default())
}

fn execute_update_address_list(
    deps: DepsMut,
    info: MessageInfo,
    address_list: AddressListModule,
) -> StdResult<Response> {
    let state = STATE.load(deps.storage)?;
    require(
        state.owner == info.sender,
        StdError::generic_err("May only be used by the contract owner"),
    )?;
    let mut splitter = SPLITTER.load(deps.storage)?;

    if splitter.locked == true {
        StdError::generic_err("Not allow to change whitelist");
    }

    splitter.address_list = Some(address_list);

    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetSplitterConfig {} => to_binary(&query_splitter(deps)?),
    }
}

fn query_splitter(deps: Deps) -> StdResult<GetSplitterConfigResponse> {
    let splitter = SPLITTER.load(deps.storage)?;
    let address_list_contract = match splitter.clone().address_list {
        Some(addr_list) => addr_list.get_contract_address(deps.storage),
        None => None,
    };

    Ok(GetSplitterConfigResponse {
        config: splitter,
        address_list_contract,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::modules::address_list::AddressListModule;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Addr, Coin, Uint128};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            address_list: None,
            recipients: vec![AddressPercent {
                addr: String::from("Some Address"),
                percent: Uint128::from(100 as u128),
            }],
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_update_lock() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let owner = "creator";
        let info = mock_info(owner.clone(), &[]);

        let lock = true;
        let msg = ExecuteMsg::UpdateLock { lock: lock };

        //incorrect owner
        let state = State {
            owner: Addr::unchecked("incorrect_owner".to_string()),
        };
        STATE.save(deps.as_mut().storage, &state).unwrap();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_ret) => assert!(false),
            _ => {}
        }

        let state = State {
            owner: Addr::unchecked(owner.to_string()),
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
            address_list: None,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default(), res);

        //check result
        let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
        assert_eq!(splitter.locked, lock);
    }

    #[test]
    fn test_execute_update_address_list() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "creator";

        let state = State {
            owner: Addr::unchecked(owner.to_string()),
        };
        STATE.save(deps.as_mut().storage, &state).unwrap();

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
            address_list: None,
        };
        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let address_list = AddressListModule {
            address: Some(String::from("terra1contractaddress")),
            code_id: None,
            moderators: None,
            inclusive: true,
        };
        let msg = ExecuteMsg::UpdateAddressList {
            address_list: address_list.clone(),
        };

        let unauth_info = mock_info("anyone", &[]);
        let err_res =
            execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            err_res,
            StdError::generic_err("May only be used by the contract owner")
        );

        let info = mock_info(owner.clone(), &[]);
        execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let updated = SPLITTER.load(deps.as_mut().storage).unwrap();

        assert_eq!(updated.address_list.unwrap(), address_list);
    }

    #[test]
    fn test_execute_update_recipients() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let owner = "creator";
        let info = mock_info(owner.clone(), &[]);

        let recipient = vec![
            AddressPercent {
                addr: "address1".to_string(),
                percent: Uint128::from(40 as u128),
            },
            AddressPercent {
                addr: "address1".to_string(),
                percent: Uint128::from(60 as u128),
            },
        ];
        let msg = ExecuteMsg::UpdateRecipients {
            recipients: recipient.clone(),
        };

        //incorrect owner
        let state = State {
            owner: Addr::unchecked("incorrect_owner".to_string()),
        };
        STATE.save(deps.as_mut().storage, &state).unwrap();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_ret) => assert!(false),
            _ => {}
        }

        let state = State {
            owner: Addr::unchecked(owner.to_string()),
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
            address_list: None,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default(), res);

        //check result
        let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
        assert_eq!(splitter.recipients, recipient.clone());
    }

    #[test]
    fn test_execute_send() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let owner = "creator";
        let info = mock_info(owner.clone(), &vec![Coin::new(10000, "uluna")]);

        let recipient = vec![
            AddressPercent {
                addr: "address1".to_string(),
                percent: Uint128::from(10 as u128),
            },
            AddressPercent {
                addr: "address1".to_string(),
                percent: Uint128::from(20 as u128),
            },
        ];
        let msg = ExecuteMsg::Send {};

        //incorrect owner
        let state = State {
            owner: Addr::unchecked("incorrect_owner".to_string()),
        };
        STATE.save(deps.as_mut().storage, &state).unwrap();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_ret) => assert!(false),
            _ => {}
        }

        let state = State {
            owner: Addr::unchecked(owner.to_string()),
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let splitter = Splitter {
            recipients: recipient,
            locked: false,
            address_list: None,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_ne!(Response::default(), res);
    }

    #[test]
    fn test_query_splitter() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let splitter = Splitter {
            recipients: vec![],
            locked: false,
            address_list: Some(AddressListModule {
                address: Some(String::from("somecontractaddress")),
                code_id: None,
                moderators: None,
                inclusive: false,
            }),
        };

        SPLITTER
            .save(deps.as_mut().storage, &splitter.clone())
            .unwrap();

        let query_msg = QueryMsg::GetSplitterConfig {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetSplitterConfigResponse = from_binary(&res).unwrap();

        assert_eq!(val.config, splitter);
        assert_eq!(
            val.address_list_contract.unwrap(),
            splitter.address_list.unwrap().address.unwrap()
        );
    }
}
