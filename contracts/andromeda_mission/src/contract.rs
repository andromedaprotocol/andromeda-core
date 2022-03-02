#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, Storage};
use cw2::{get_contract_version, set_contract_version};

use andromeda_protocol::{
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    mission::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ADO_ADDRESSES},
    operators::{
        execute_update_operators, initialize_operators, is_operator, query_is_operator,
        query_operators,
    },
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_primitive";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    initialize_operators(deps.storage, msg.operators)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
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
        AndromedaMsg::Withdraw { .. } => Err(ContractError::UnsupportedOperation {}),
    }
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, msg),
    }
}

fn handle_andromeda_query(deps: Deps, msg: AndromedaQuery) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => match data {
            // Treat no binary as request to get value with default key.
            None => Err(ContractError::ParsingError {
                err: String::from("No data passed with AndrGet query"),
            }),
            Some(_) => {
                let name: String = parse_message(data)?;
                encode_binary(&query_ado_address(deps, name)?)
            }
        },
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
        AndromedaQuery::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, &address)?)
        }
    }
}

fn query_ado_address(deps: Deps, name: String) -> Result<String, ContractError> {
    let value = ADO_ADDRESSES.load(deps.storage, name)?;
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            operators: vec![],
            mission: vec![],
            xfer_ado_ownership: false,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
