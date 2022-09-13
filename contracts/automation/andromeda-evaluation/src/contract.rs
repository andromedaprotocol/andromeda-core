use crate::state::{CONDITION_ADO_ADDRESS, QUERY_ADO_ADDRESS};
use ado_base::state::ADOContract;
use andromeda_automation::evaluation::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, Operators, QueryMsg,
};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, app::AndrAddress, encode_binary,
    error::ContractError, require,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Reply, Response, StdError, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::nonpayable;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-evaluation";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONDITION_ADO_ADDRESS.save(deps.storage, &msg.condition_address)?;
    QUERY_ADO_ADDRESS.save(deps.storage, &msg.query_address)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "evaluation".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

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
        ExecuteMsg::Evaluate {
            user_value,
            operation,
        } => execute_evaluate(deps, env, info, user_value, operation),
        ExecuteMsg::ChangeConditionAddress { address } => {
            execute_change_condition_address(deps, env, info, address)
        }
        ExecuteMsg::ChangeQueryAddress { address } => {
            execute_change_query_address(deps, env, info, address)
        }
    }
}

fn execute_change_query_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: AndrAddress,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Only the contract's owner can update the Execute ADO address
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    QUERY_ADO_ADDRESS.save(deps.storage, &address)?;
    Ok(Response::new().add_attribute("action", "changed_query_ado_address"))
}

fn execute_change_condition_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: AndrAddress,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Only the contract's owner can update the Execute ADO address
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    CONDITION_ADO_ADDRESS.save(deps.storage, &address)?;
    Ok(Response::new().add_attribute("action", "changed_execute_ado_address"))
}

fn execute_evaluate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user_value: Uint128,
    operation: Operators,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;

    // get the address of the ADO that will interpret our result
    let contract_addr = CONDITION_ADO_ADDRESS.load(deps.storage)?.get_address(
        deps.api,
        &deps.querier,
        app_contract.clone(),
    )?;

    // get the address of the oracle contract that will provide data to be compared with the user's data
    let query_addr =
        QUERY_ADO_ADDRESS
            .load(deps.storage)?
            .get_address(deps.api, &deps.querier, app_contract)?;

    let oracle_value: Uint128 = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: query_addr,
        msg: to_binary(&andromeda_automation::counter::QueryMsg::Count {})?,
    }))?;

    match operation {
        Operators::Greater => Ok(Response::new()
            .add_attribute("result", (oracle_value > user_value).to_string())
            .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
                    result: oracle_value > user_value,
                })?,
                funds: vec![],
            })))),
        Operators::GreaterEqual => Ok(Response::new()
            .add_attribute("result", (oracle_value >= user_value).to_string())
            .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
                    result: oracle_value >= user_value,
                })?,
                funds: vec![],
            })))),
        Operators::Equal => Ok(Response::new()
            .add_attribute("result", (oracle_value == user_value).to_string())
            .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
                    result: oracle_value == user_value,
                })?,
                funds: vec![],
            })))),
        Operators::LessEqual => Ok(Response::new()
            .add_attribute("result", (oracle_value <= user_value).to_string())
            .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
                    result: oracle_value <= user_value,
                })?,
                funds: vec![],
            })))),
        Operators::Less => Ok(Response::new()
            .add_attribute("result", (oracle_value < user_value).to_string())
            .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
                    result: oracle_value < user_value,
                })?,
                funds: vec![],
            })))),
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

    require(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        },
    )?;

    // New version has to be newer/greater than the old version
    require(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        },
    )?;

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
        QueryMsg::ConditionADO {} => encode_binary(&query_condition_ado(deps)?),
        QueryMsg::QueryADO {} => encode_binary(&query_query_ado(deps)?),
    }
}

fn query_query_ado(deps: Deps) -> Result<String, ContractError> {
    let address = QUERY_ADO_ADDRESS.load(deps.storage)?;
    Ok(address.identifier)
}

fn query_condition_ado(deps: Deps) -> Result<String, ContractError> {
    let address = CONDITION_ADO_ADDRESS.load(deps.storage)?;
    Ok(address.identifier)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use andromeda_automation::evaluation::Operators;
//     use common::app::AndrAddress;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

//     #[test]
//     fn test_initialization() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Greater;
//         let msg = InstantiateMsg {
//             operation,
//             execute_address: todo!(),
//             query_address: todo!(),
//         };
//         let info = mock_info("creator", &[]);

//         // we can just call .unwrap() to assert this was a success
//         let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // make sure address was saved correctly
//         let addr = EXECUTE_ADO_ADDRESS.load(&deps.storage).unwrap();
//         assert_eq!(
//             addr,
//             AndrAddress {
//                 identifier: "legit_address".to_string(),
//             }
//         )
//     }

//     #[test]
//     fn test_evaluate_greater_is_greater() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Greater;
//         let msg = InstantiateMsg {
//             operation: operation.clone(),
//             execute_address: todo!(),
//             query_address: todo!(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(40);
//         let user_value = Uint128::new(30);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "true".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: true,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_greater_is_less() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Greater;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(40);
//         let user_value = Uint128::new(130);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "false".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: false,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_greater_is_equal() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Greater;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(40);
//         let user_value = Uint128::new(40);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "false".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: false,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_greater_equal_is_equal() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::GreaterEqual;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(40);
//         let user_value = Uint128::new(40);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "true".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: true,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_greater_equal_is_greater() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::GreaterEqual;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(140);
//         let user_value = Uint128::new(40);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "true".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: true,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_greater_equal_is_less() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::GreaterEqual;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(40);
//         let user_value = Uint128::new(140);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "false".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: false,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_equal_is_equal() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Equal;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(40);
//         let user_value = Uint128::new(40);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "true".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: true,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_equal_is_greater() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Equal;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(140);
//         let user_value = Uint128::new(40);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "false".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: false,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_equal_is_less() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Equal;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(140);
//         let user_value = Uint128::new(1140);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "false".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: false,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_less_equal_is_less() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::LessEqual;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(140);
//         let user_value = Uint128::new(1140);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "true".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: true,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_less_equal_is_equal() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::LessEqual;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(140);
//         let user_value = Uint128::new(140);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "true".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: true,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_less_equal_is_greater() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::LessEqual;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(1140);
//         let user_value = Uint128::new(140);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "false".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: false,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_less_is_greater() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Less;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(1140);
//         let user_value = Uint128::new(140);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "false".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: false,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_less_is_equal() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Less;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(140);
//         let user_value = Uint128::new(140);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "false".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: false,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_evaluate_less_is_less() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Less;
//         let msg = InstantiateMsg {
//             address,
//             operation: operation.clone(),
//         };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let oracle_value = Uint128::new(40);
//         let user_value = Uint128::new(140);
//         let msg = ExecuteMsg::Evaluate {
//             oracle_value,
//             user_value,
//             operation,
//         };

//         let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let expected = Response::new()
//             .add_attribute("result", "true".to_string())
//             .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "legit_address".to_string(),
//                 msg: to_binary(&andromeda_automation::condition::ExecuteMsg::StoreResult {
//                     result: true,
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })));
//         assert_eq!(res, expected);
//     }

//     #[test]
//     fn test_change_address_unauthorized() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Greater;
//         let msg = InstantiateMsg { address, operation };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         let address = AndrAddress {
//             identifier: "new_address".to_string(),
//         };
//         let msg = ExecuteMsg::ChangeExecuteAddress { address };
//         let info = mock_info("random", &[]);

//         let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
//         assert_eq!(err, ContractError::Unauthorized {})
//     }

//     #[test]
//     fn test_change_address_works() {
//         let mut deps = mock_dependencies();
//         let address = AndrAddress {
//             identifier: "legit_address".to_string(),
//         };
//         let operation = Operators::Greater;
//         let msg = InstantiateMsg { address, operation };
//         let info = mock_info("creator", &[]);

//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let address = AndrAddress {
//             identifier: "new_address".to_string(),
//         };
//         let msg = ExecuteMsg::ChangeExecuteAddress {
//             address: address.clone(),
//         };

//         let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let actual = EXECUTE_ADO_ADDRESS.load(&deps.storage).unwrap();
//         assert_eq!(address, actual)
//     }
// }
