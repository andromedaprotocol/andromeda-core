use andromeda_protocol::{
    address_list::{AddressList, ExecuteMsg, IncludesAddressResponse, InstantiateMsg, QueryMsg},
    ownership::{execute_update_owner, query_contract_owner, CONTRACT_OWNER},
    require,
};
use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
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
        owner: info.sender.to_string(),
        address_list: AddressList {
            moderators: msg.moderators,
        },
    };

    CONTRACT_OWNER.save(deps.storage, &info.sender.to_string())?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "instantiate"),
        attr("type", "address_list"),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::AddAddress { address } => execute_add_address(deps, info, address),
        ExecuteMsg::RemoveAddress { address } => execute_remove_address(deps, info, address),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
    }
}

fn execute_add_address(deps: DepsMut, info: MessageInfo, address: String) -> StdResult<Response> {
    let state = STATE.load(deps.storage)?;

    require(
        state.address_list.is_moderator(&info.sender.to_string()),
        StdError::generic_err("Only a moderator can add an address to the address list"),
    )?;

    state
        .address_list
        .add_address(deps.storage, &address)
        .unwrap();

    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "add_address"),
        attr("address", address),
    ]))
}

fn execute_remove_address(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> StdResult<Response> {
    let state = STATE.load(deps.storage)?;
    require(
        state.address_list.is_moderator(&info.sender.to_string()),
        StdError::generic_err("Only a moderator can remove an address from the address list"),
    )?;

    state.address_list.remove_address(deps.storage, &address);
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "remove_address"),
        attr("address", address),
    ]))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IncludesAddress { address } => to_binary(&query_address(deps, &address)?),
        QueryMsg::ContractOwner {} => to_binary(&query_contract_owner(deps)?),
    }
}

fn query_address(deps: Deps, address: &String) -> StdResult<IncludesAddressResponse> {
    let state = STATE.load(deps.storage)?;

    Ok(IncludesAddressResponse {
        included: state.address_list.includes_address(deps.storage, address)?,
    })
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
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
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
            owner: moderator.to_string(),
            address_list: AddressList {
                moderators: vec![moderator.to_string()],
            },
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let msg = ExecuteMsg::AddAddress {
            address: address.to_string(),
        };

        //add address for registered moderator

        let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "add_address"),
            attr("address", address),
        ]);
        assert_eq!(expected, res);

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
        let res = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
        assert_eq!(
            StdError::generic_err("Only a moderator can add an address to the address list"),
            res
        );
    }

    #[test]
    fn test_remove_address() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let moderator = "creator";
        let info = mock_info(moderator.clone(), &[]);

        let address = "whitelistee";

        //save moderator

        let state = State {
            owner: moderator.to_string(),
            address_list: AddressList {
                moderators: vec![moderator.to_string()],
            },
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let msg = ExecuteMsg::RemoveAddress {
            address: address.to_string(),
        };

        //add address for registered moderator
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "remove_address"),
            attr("address", address.to_string()),
        ]);
        assert_eq!(expected, res);

        let included_is_err = ADDRESS_LIST
            .load(deps.as_ref().storage, address.to_string())
            .is_err();
        assert_eq!(true, included_is_err);

        //add address for unregistered moderator
        let unauth_info = mock_info("anyone", &[]);
        let res = execute(deps.as_mut(), env, unauth_info.clone(), msg).unwrap_err();
        assert_eq!(
            StdError::generic_err("Only a moderator can remove an address from the address list"),
            res
        );
    }
}
