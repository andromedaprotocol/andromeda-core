use crate::state::{EXPECTED_TYPE_RESPONSE, QUERY_MSG, TARGET_ADO_ADDRESS};
use ado_base::state::ADOContract;
use andromeda_automation::{
    counter::CounterResponse,
    oracle::{
        ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ResponseTypes, TypeOfResponse, Types,
    },
};
use base64;
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use serde_json_wasm::to_string as json_string;

use std::env;

use cosmwasm_std::{
    ensure, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Reply, Response,
    StdError, Uint128, WasmQuery,
};

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
    EXPECTED_TYPE_RESPONSE.save(deps.storage, &msg.return_type)?;

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

fn query_stored_message(deps: Deps) -> Result<String, ContractError> {
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

    let decoded_string = base64::decode(stored_msg).unwrap();
    let msg = Binary::from(decoded_string);

    let ty = EXPECTED_TYPE_RESPONSE.load(deps.storage)?;

    if ty == TypeOfResponse::Types(Types::Bool) {
        let response: bool = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
        Ok(json_string(&response).unwrap())
    } else if ty == TypeOfResponse::Types(Types::Uint128) {
        let response: Uint128 = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
        Ok(json_string(&response).unwrap())
    } else if ty == TypeOfResponse::Types(Types::String) {
        let response: String = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
        Ok(response)
    } else if ty == TypeOfResponse::ResponseTypes(ResponseTypes::CounterResponseCount) {
        let response: CounterResponse = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
        let count = response.count.to_string();
        Ok(count)
    } else if ty == TypeOfResponse::ResponseTypes(ResponseTypes::CounterResponsePreviousCount) {
        let response: CounterResponse = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
        let previous_count = response.previous_count.to_string();
        Ok(previous_count)
    } else {
        Err(ContractError::UnsupportedReturnType {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{
        mock_dependencies_custom, MOCK_BOOL_CONTRACT, MOCK_COUNTER_CONTRACT,
        MOCK_RESPONSE_COUNTER_CONTRACT,
    };
    use andromeda_automation::oracle::TypeOfResponse;
    use cosmwasm_std::{
        testing::{mock_env, mock_info},
        to_binary, Uint128,
    };
    use serde_json_wasm::from_str as from_json;

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            message_binary: "eyJjb3VudCI6e319".to_string(),
            return_type: TypeOfResponse::Types(Types::Uint128),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_binary_conversion() {
        // receive encoded the json as base64
        let binary = "eyJjdXJyZW50X3RhcmdldCI6e319";

        // turn base64 into string
        let decoded_binary = base64::decode(binary).unwrap();
        let vec_bin = Binary::from(decoded_binary);

        let actual_binary = to_binary(&QueryMsg::CurrentTarget {}).unwrap();

        assert_eq!(actual_binary, vec_bin)
    }

    #[test]
    fn test_uint128() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            message_binary: "eyJjb3VudCI6e319".to_string(),
            return_type: TypeOfResponse::Types(Types::Uint128),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = TARGET_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(addr, MOCK_COUNTER_CONTRACT.to_string());

        let message = QUERY_MSG.load(&deps.storage).unwrap();
        assert_eq!(message, "eyJjb3VudCI6e319".to_string());

        let res = query_target(deps.as_ref()).unwrap();

        println!("Pre-parsed result: {:?}", res);
        let parsed_result: Uint128 = from_json(&res).unwrap();

        println!("Parsed result: {:?}", parsed_result);
        assert_eq!(parsed_result, Uint128::new(1))
    }

    #[test]
    fn test_u32() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            message_binary: "eyJjb3VudCI6e319".to_string(),
            return_type: TypeOfResponse::Types(Types::Uint128),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = TARGET_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(addr, MOCK_COUNTER_CONTRACT.to_string());
        let message = QUERY_MSG.load(&deps.storage).unwrap();
        assert_eq!(message, "eyJjb3VudCI6e319".to_string());
        let res = query_target(deps.as_ref()).unwrap();

        println!("Pre-parsed result: {:?}", res);
        let parsed_result: i32 = res.parse().unwrap();

        println!("Parsed result: {:?}", parsed_result);
        assert_eq!(parsed_result, 1)
        // We can now assume that we can parse into any type of number
    }

    #[test]
    fn test_bool() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_BOOL_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            message_binary: "eyJjb3VudCI6e319".to_string(),
            return_type: TypeOfResponse::Types(Types::Bool),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = TARGET_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(addr, MOCK_BOOL_CONTRACT.to_string());

        let message = QUERY_MSG.load(&deps.storage).unwrap();
        assert_eq!(message, "eyJjb3VudCI6e319".to_string());

        let res = query_target(deps.as_ref()).unwrap();

        println!("Pre-parsed result: {:?}", res);
        let parsed_result: bool = res.parse().unwrap();

        println!("Parsed result: {:?}", parsed_result);

        // The mock querier always returns true
        assert!(parsed_result)
    }

    #[test]
    fn test_counter_response_count() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_RESPONSE_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            message_binary: "eyJjb3VudCI6e319".to_string(),
            return_type: TypeOfResponse::ResponseTypes(ResponseTypes::CounterResponseCount),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query_target(deps.as_ref()).unwrap();

        println!("Pre-parsed result: {:?}", res);
        let parsed_result: Uint128 = res.parse().unwrap();

        println!("Parsed result: {:?}", parsed_result);

        // Count is always 1
        assert_eq!(parsed_result, Uint128::new(1))
    }

    #[test]
    fn test_counter_response_previous_count() {
        let mut deps = mock_dependencies_custom(&[]);
        let target_address = MOCK_RESPONSE_COUNTER_CONTRACT.to_string();

        let msg = InstantiateMsg {
            target_address,
            message_binary: "eyJjb3VudCI6e319".to_string(),
            return_type: TypeOfResponse::ResponseTypes(ResponseTypes::CounterResponsePreviousCount),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query_target(deps.as_ref()).unwrap();

        println!("Pre-parsed result: {:?}", res);
        let parsed_result: Uint128 = res.parse().unwrap();

        println!("Parsed result: {:?}", parsed_result);

        // Previous count is always 0
        assert_eq!(parsed_result, Uint128::zero())
    }
}
