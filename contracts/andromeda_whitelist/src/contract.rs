use andromeda_protocol::modules::whitelist::Whitelist;
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, IsWhitelistedResponse, QueryMsg},
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
        whitelist: Whitelist {
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
        ExecuteMsg::Whitelist { address } => execute_whitelist(deps, info, address),
        ExecuteMsg::RemoveWhitelist { address } => execute_remove_whitelist(deps, info, address),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsWhitelisted { address } => query_process(deps, &address),
    }
}

fn query_process(deps: Deps, address: &String) -> StdResult<Binary> {
    let state = STATE.load(deps.storage)?;

    to_binary(&IsWhitelistedResponse {
        whitelisted: state.whitelist.is_whitelisted(deps.storage, address)?,
    })
}

fn execute_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    if state.whitelist.is_moderator(&info.sender.to_string()) == false {
        return Err(ContractError::Unauthorized {});
    }

    state
        .whitelist
        .whitelist_addr(deps.storage, &address)
        .unwrap();

    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

fn execute_remove_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.whitelist.is_moderator(&info.sender.to_string()) == false {
        return Err(ContractError::Unauthorized {});
    }

    state
        .whitelist
        .remove_whitelist(deps.storage, &address)
        .unwrap();
    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::modules::whitelist::WHITELIST;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            creator: String::from("creator"),
            moderators: vec!["11".to_string(), "22".to_string()],
        };
        let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_whitelist() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let moderator = "creator";
        let info = mock_info(moderator.clone(), &[]);

        let whitelistee = "whitelistee";

        //input moderator for test

        let state = State {
            creator: moderator.to_string(),
            whitelist: Whitelist {
                moderators: vec![moderator.to_string()],
            },
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let msg = ExecuteMsg::Whitelist {
            address: whitelistee.to_string(),
        };

        //add address for registered moderator

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default(), res);

        let whitelisted = WHITELIST
            .load(deps.as_ref().storage, whitelistee.to_string())
            .unwrap();
        assert_eq!(true, whitelisted);

        let whitelisted = WHITELIST
            .load(deps.as_ref().storage, "111".to_string())
            .unwrap_err();

        match whitelisted {
            cosmwasm_std::StdError::NotFound { .. } => {
                assert_eq!(false, false);
            }
            _ => {
                assert_eq!(false, true);
            }
        }

        // assert_eq!(cosmwasm_std::StdError::NotFound {kind: }, whitelisted);

        //add address for unregistered moderator
        let unauth_info = mock_info("anyone", &[]);
        let res =
            execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone()).unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, res);
    }

    #[test]
    fn test_remove_whitelist() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let moderator = "creator";
        let info = mock_info(moderator.clone(), &[]);

        let whitelistee = "whitelistee";

        //save moderator

        let state = State {
            creator: moderator.to_string(),
            whitelist: Whitelist {
                moderators: vec![moderator.to_string()],
            },
        };

        STATE.save(deps.as_mut().storage, &state).unwrap();

        let msg = ExecuteMsg::RemoveWhitelist {
            address: whitelistee.to_string(),
        };

        //add address for registered moderator
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default(), res);

        let whitelisted = WHITELIST
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
