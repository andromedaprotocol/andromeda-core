use crate::state::{QUERY_MSG, QUERY_RESPONSE, RESPONSE_ELEMENT, TARGET_ADO_ADDRESS};
use ado_base::state::ADOContract;
use andromeda_automation::{
    counter::CounterResponse,
    oracle::{
        CustomTypes, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, RegularTypes, TypeOfResponse,
    },
};

use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};

use cosmwasm_std::{
    ensure, entry_point, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Reply,
    Response, StdError, Uint128, WasmQuery,
};
use serde_json::Value;
use serde_json::{from_str, to_string};
use std::env;

use cw2::{get_contract_version, set_contract_version};

use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    QUERY_MSG.save(deps.storage, &msg.message_binary)?;
    TARGET_ADO_ADDRESS.save(deps.storage, &msg.target_address)?;

    if let Some(response_element) = msg.response_element {
        RESPONSE_ELEMENT.save(deps.storage, &response_element)?;
    }
    QUERY_RESPONSE.save(deps.storage, &msg.return_type)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "oracle".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    match msg {
        ExecuteMsg::AndrReceive(msg) => contract.execute(deps, env, info, msg, execute),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::CurrentTarget {} => encode_binary(&query_current_target(deps)?),
        QueryMsg::Target {} => encode_binary(&query_target(deps)?),
        QueryMsg::StoredMessage {} => encode_binary(&query_stored_message(deps)?),
    }
}

fn query_stored_message(deps: Deps) -> Result<Binary, ContractError> {
    let message = QUERY_MSG.load(deps.storage)?;
    Ok(message)
}

fn query_current_target(deps: Deps) -> Result<String, ContractError> {
    let address = TARGET_ADO_ADDRESS.load(deps.storage)?;
    Ok(address)
}

fn query_target(deps: Deps) -> Result<String, ContractError> {
    let contract_addr = TARGET_ADO_ADDRESS.load(deps.storage)?;
    let stored_msg = QUERY_MSG.load(deps.storage)?;

    let msg = from_binary(&stored_msg)?;
    let expected_response = QUERY_RESPONSE.load(deps.storage)?;
    let response_element = RESPONSE_ELEMENT.may_load(deps.storage)?;

    if expected_response == TypeOfResponse::RegularType(RegularTypes::Bool) {
        let response: bool = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
        Ok(response.to_string())
    } else if expected_response == TypeOfResponse::RegularType(RegularTypes::Uint128) {
        let response: Uint128 = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
        Ok(response.to_string())
    } else if expected_response == TypeOfResponse::RegularType(RegularTypes::String) {
        let response: String = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
        Ok(response)
    } else if expected_response == TypeOfResponse::CustomType(CustomTypes::CounterResponse) {
        let query_response: CounterResponse = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;

        let json = to_string(&query_response).unwrap();
        let from_json: Value = from_str(&json).unwrap();

        if let Some(response_element) = response_element {
            let response = &from_json[response_element];
            Ok(response.to_string())
        } else {
            Err(ContractError::ResponseElementRequired {})
        }
    } else {
        Err(ContractError::UnsupportedReturnType {})
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::mock_querier::{
        mock_dependencies_custom, MOCK_COUNTER_CONTRACT, MOCK_RESPONSE_COUNTER_CONTRACT,
    };
    use cosmwasm_std::{
        from_binary,
        testing::{mock_env, mock_info},
        to_binary, Uint128,
    };
    use serde_json::{self, from_str, to_string, Value};

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            message_binary: to_binary("eyJjb3VudCI6e319").unwrap(),
            return_type: TypeOfResponse::CustomType(CustomTypes::CounterResponse),
            response_element: Some("count".to_string()),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_binary_conversion() {
        // receive encoded the json as base64
        let binary = to_binary("eyJjdXJyZW50X3RhcmdldCI6e319").unwrap();
        let vec_bin: Binary = from_binary(&binary).unwrap();
        let actual_binary = to_binary(&QueryMsg::CurrentTarget {}).unwrap();

        assert_eq!(actual_binary, vec_bin);
    }

    #[test]
    fn test_json_conversion() {
        let query_response = CounterResponse {
            count: Uint128::new(1),
            previous_count: Uint128::zero(),
        };
        let json = to_string(&query_response).unwrap();
        println!("JSON to_string: {:?}", json);

        let from_json: Value = from_str(&json).unwrap();
        println!("From String: {:?}", from_json["count"]);

        let count = &from_json["count"];
        let string_count = count.to_string();
        println!("String version: {:?}", string_count);

        let from_stringg: Uint128 = from_str(&string_count).unwrap();
        println!("From String version: {:?}", from_stringg);

        assert_eq!(from_stringg, Uint128::new(1));
    }

    #[test]
    fn test_uint128() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            // Current Count msg
            message_binary: to_binary("eyJjdXJyZW50X2NvdW50Ijp7fX0=").unwrap(),
            return_type: TypeOfResponse::RegularType(RegularTypes::Uint128),
            response_element: None,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = TARGET_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(addr, MOCK_COUNTER_CONTRACT.to_string());

        let res = query_target(deps.as_ref()).unwrap();
        println!("Response: {:?}", res);

        let from_stringg: Uint128 = res.parse().unwrap();
        println!("Parsed version: {:?}", from_stringg);
        assert_eq!(from_stringg, Uint128::new(1))
    }

    #[test]
    fn test_u32() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            // Current Count msg
            message_binary: to_binary("eyJjdXJyZW50X2NvdW50Ijp7fX0=").unwrap(),
            return_type: TypeOfResponse::RegularType(RegularTypes::Uint128),
            response_element: None,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = TARGET_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(addr, MOCK_COUNTER_CONTRACT.to_string());

        let res = query_target(deps.as_ref()).unwrap();
        println!("Response: {:?}", res);

        let from_stringg: u32 = res.parse().unwrap();
        println!("Parsed version: {:?}", from_stringg);
        assert_eq!(from_stringg, 1)
        // We can now assume that we can parse into any type of number
    }

    #[test]
    fn test_bool() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            // Is Zero msg
            message_binary: to_binary("eyJpc196ZXJvIjp7fX0=").unwrap(),
            return_type: TypeOfResponse::RegularType(RegularTypes::Bool),
            response_element: None,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res: String = query_target(deps.as_ref()).unwrap();

        println!("Pre-parsed result: {:?}", res);
        let parsed_result: bool = res.parse().unwrap();

        println!("Parsed result: {:?}", parsed_result);

        // The mock querier always returns false
        assert!(!parsed_result)
    }

    #[test]
    fn test_counter_response_count() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_RESPONSE_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            // Count msg
            message_binary: to_binary("eyJjb3VudCI6e319").unwrap(),
            return_type: TypeOfResponse::CustomType(CustomTypes::CounterResponse),
            response_element: Some("count".to_string()),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res: String = query_target(deps.as_ref()).unwrap();

        println!("Pre-parsed result: {:?}", res);

        let from_stringg: Uint128 = from_str(&res).unwrap();
        println!("From String version: {:?}", from_stringg);

        assert_eq!(from_stringg, Uint128::new(1));
    }

    #[test]
    fn test_counter_response_count_no_response_element() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_RESPONSE_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            // Count msg
            message_binary: to_binary("eyJjb3VudCI6e319").unwrap(),
            return_type: TypeOfResponse::CustomType(CustomTypes::CounterResponse),
            response_element: None,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let err = query_target(deps.as_ref()).unwrap_err();
        assert_eq!(err, ContractError::ResponseElementRequired {});
    }

    #[test]
    fn test_counter_response_previous_count() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_RESPONSE_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            // Count msg
            message_binary: to_binary("eyJjb3VudCI6e319").unwrap(),
            return_type: TypeOfResponse::CustomType(CustomTypes::CounterResponse),
            response_element: Some("previous_count".to_string()),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query_target(deps.as_ref()).unwrap();

        println!("Pre-parsed result: {:?}", res);
        let from_stringg: Uint128 = from_str(&res).unwrap();
        println!("From String version: {:?}", from_stringg);

        assert_eq!(from_stringg, Uint128::zero());
    }
}
