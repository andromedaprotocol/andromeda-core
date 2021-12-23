#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{AddressResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE,store_address,read_address, is_contract_owner};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:updated-fetch";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        count: msg.count,
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("count", msg.count.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Update {name, contract_address} => try_update(deps,info, name, contract_address),
    }
}

pub fn try_update(deps: DepsMut, info: MessageInfo, name: String, contract_address: String) -> Result<Response, ContractError> {
    if is_contract_owner(deps.storage, info.sender.to_string()) == Ok(false) {
        panic!("Only contract admin can update addresses.")
    }
    store_address(deps.storage, name, &contract_address).unwrap();
    
    Ok(Response::new().add_attribute("method", "try_update"))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAddress {name} => to_binary(&query_address(deps,name)?),
    }
}

fn query_address(deps: Deps, name: String) -> StdResult<AddressResponse> {
    let addr = read_address(deps.storage, name).unwrap();
    Ok(AddressResponse { address: addr.to_string()})
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary};

    #[test]
    fn proper_instantiation() {
        let mut deps = mock_dependencies(&[]);
        let msg = InstantiateMsg { count: 0 }; 
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

    }

    #[test]
    fn state_working_test() {
        // Make sure we are able to update address into state and then query it. (2 messages to see if it can differentiate)
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {count: 0};

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let _res = execute(deps.as_mut(), mock_env(), info.clone(), ExecuteMsg::Update {name: String::from("factory"), contract_address: String::from("factory_addr")}).unwrap();
        let _res2 = execute(deps.as_mut(), mock_env(), info.clone(), ExecuteMsg::Update {name: String::from("automation"), contract_address: String::from("auto_addr")}).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetAddress {name: String::from("factory")}).unwrap();
        let res2 = query(deps.as_ref(), mock_env(), QueryMsg::GetAddress {name: String::from("automation")}).unwrap();

        let value: AddressResponse = from_binary(&res).unwrap();
        let value2: AddressResponse = from_binary(&res2).unwrap();
        assert_eq!(value.address, "factory_addr".to_string());
        assert_eq!(value2.address, "auto_addr".to_string());
    }
}
