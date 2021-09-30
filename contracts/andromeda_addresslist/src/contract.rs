use andromeda_protocol::address_list::{
    AddressList, ExecuteMsg, IncludesAddressResponse, InstantiateMsg, QueryMsg,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::{
    error::ContractError,
    state::{State, STATE},
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        creator: info.sender.to_string(),
        address_list: AddressList {
            moderators: msg.moderators.clone(),
        },
    };

    STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAddress { address } => execute_add_address(deps, info, address),
        ExecuteMsg::RemoveAddress { address } => execute_remove_address(deps, info, address),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IncludesAddress { address } => to_binary(&query_address(deps, &address)?),
    }
}

fn query_address(deps: Deps, address: &String) -> StdResult<IncludesAddressResponse> {
    let state = STATE.load(deps.storage)?;

    Ok(IncludesAddressResponse {
        included: state.address_list.includes_address(deps.storage, address)?,
    })
}

fn execute_add_address(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    if state.address_list.is_moderator(&info.sender.to_string()) == false {
        return Err(ContractError::Unauthorized {});
    }

    state
        .address_list
        .add_address(deps.storage, &address)
        .unwrap();

    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

fn execute_remove_address(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.address_list.is_moderator(&info.sender.to_string()) == false {
        return Err(ContractError::Unauthorized {});
    }

    state
        .address_list
        .remove_address(deps.storage, &address)
        .unwrap();
    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::address_list::ADDRESS_LIST;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            moderators: vec!["11".to_string(), "22".to_string()],
        };
        let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_add_address() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let moderator = "creator";
        let info = mock_info(moderator.clone(), &[]);

        let address = "whitelistee";

        //input moderator for test

        let state = State {
            creator: moderator.to_string(),
            address_list: AddressList {
                moderators: vec![moderator.to_string()],
            },
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let msg = ExecuteMsg::AddAddress {
            address: address.to_string(),
        };

        //add address for registered moderator

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default(), res);

        let whitelisted = ADDRESS_LIST
            .load(deps.as_ref().storage, address.to_string())
            .unwrap();
        assert_eq!(true, whitelisted);

        let included = ADDRESS_LIST
            .load(deps.as_ref().storage, "111".to_string())
            .unwrap_err();

        match included {
            cosmwasm_std::StdError::NotFound { .. } => {
                assert_eq!(false, false);
            }
            _ => {
                assert_eq!(false, true);
            }
        }

        //add address for unregistered moderator
        let unauth_info = mock_info("anyone", &[]);
        let res =
            execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone()).unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, res);
    }

    #[test]
    fn test_remove_address() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let moderator = "creator";
        let info = mock_info(moderator.clone(), &[]);

        let whitelistee = "whitelistee";

        //save moderator

        let state = State {
            creator: moderator.to_string(),
            address_list: AddressList {
                moderators: vec![moderator.to_string()],
            },
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let msg = ExecuteMsg::RemoveAddress {
            address: whitelistee.to_string(),
        };

        //add address for registered moderator
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default(), res);

        let whitelisted = ADDRESS_LIST
            .load(deps.as_ref().storage, whitelistee.to_string())
            .unwrap();
        assert_eq!(false, whitelisted);

        //add address for unregistered moderator
        let unauth_info = mock_info("anyone", &[]);
        let res =
            execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone()).unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, res);
    }
}
