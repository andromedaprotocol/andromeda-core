use ado_base::state::ADOContract;
use andromeda_automation::evaluation::QueryMsg as EvaluationQueryMsg;
use andromeda_automation::{
    condition::{ExecuteMsg, InstantiateMsg, LogicGate, MigrateMsg, QueryMsg},
    evaluation::Operators,
    execute,
};

use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, app::AndrAddress, encode_binary,
    error::ContractError,
};
use cosmwasm_std::{
    ensure, entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Reply, Response, StdError, Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::nonpayable;
use semver::Version;

use crate::state::{EXECUTE_ADO, LOGIC_GATE, RESULTS, WHITELIST};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-condition";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    LOGIC_GATE.save(deps.storage, &msg.logic_gate)?;
    WHITELIST.save(deps.storage, &msg.whitelist)?;
    EXECUTE_ADO.save(deps.storage, &msg.execute_ado)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "condition".to_string(),
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
        ExecuteMsg::Interpret {} => execute_interpret(deps, env, info),
        ExecuteMsg::StoreResult { result } => execute_store_result(deps, env, info, result),
        ExecuteMsg::GetResult {} => execute_get_result(deps, env, info),
        ExecuteMsg::UpdateExecuteADO { address } => {
            execute_update_execute_ado(deps, env, info, address)
        }
        ExecuteMsg::UpdateWhitelist { addresses } => {
            execute_update_whitelist(deps, env, info, addresses)
        }
        ExecuteMsg::UpdateLogicGate { logic_gate } => {
            execute_update_logic_gate(deps, env, info, logic_gate)
        }
    }
}

fn execute_update_logic_gate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    logic_gate: LogicGate,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Check authority
    let contract = ADOContract::default();
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    LOGIC_GATE.save(deps.storage, &logic_gate)?;

    Ok(Response::new().add_attribute("action", "updated_logic_gate"))
}

fn execute_update_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Check authority
    let contract = ADOContract::default();
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    WHITELIST.save(deps.storage, &addresses)?;

    Ok(Response::new().add_attribute("action", "updated_whitelist"))
}

fn execute_update_execute_ado(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: AndrAddress,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Check authority
    let contract = ADOContract::default();
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    EXECUTE_ADO.save(deps.storage, &address)?;
    Ok(Response::new()
        .add_attribute("action", "updated_execute_ado")
        .add_attribute("new_address", address.identifier))
}

fn execute_get_result(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Check authority
    let contract = ADOContract::default();
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let whitelist = WHITELIST.load(deps.storage)?;

    // Query Eval for results
    let mut eval_results: Vec<bool> = vec![];

    for i in whitelist.into_iter() {
        let mut result: bool = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: i,
            msg: to_binary(&EvaluationQueryMsg::Evaluation {})?,
        }))?;
        eval_results.push(result);
    }

    RESULTS.save(deps.storage, &eval_results)?;
    Ok(execute_interpret(deps, _env, info)?)
}

fn execute_store_result(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    result: bool,
) -> Result<Response, ContractError> {
    let whitelist = WHITELIST.load(deps.storage)?;
    let contract = ADOContract::default();
    // Check authority
    ensure!(
        whitelist.contains(&info.sender.to_string())
            || contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    // There won't be any results to load at the beginning
    let results = RESULTS.may_load(deps.storage)?;

    // In case this isn't the first time we're storing results
    let res = if let Some(mut results) = results {
        results.push(result);
        results
    }
    // In case we're storing our first result
    else {
        let results = vec![result];
        results
    };
    RESULTS.save(deps.storage, &res)?;
    let whitelist = WHITELIST.load(deps.storage)?;

    // if the number of results equals the number of whitelisted addressses, interpret the results
    if res.len() == whitelist.len() {
        Ok(execute_interpret(deps, _env, info)?)
    } else {
        Ok(Response::new()
            .add_attribute("action", "stored result")
            .add_attribute("result", result.to_string())
            .add_attribute("address", info.sender.to_string())
            .add_attribute("result_count", res.len().to_string()))
    }
}

fn execute_interpret(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;

    // Load logic gate
    let logic = LOGIC_GATE.load(deps.storage)?;
    // Load results
    let res = RESULTS.load(deps.storage)?;
    ensure!(!res.is_empty(), ContractError::NoResults {});

    let contract_addr =
        EXECUTE_ADO
            .load(deps.storage)?
            .get_address(deps.api, &deps.querier, app_contract)?;
    match logic {
        LogicGate::AND =>
        // We don't want to find a false bool, so we want it to return false
        {
            // At least two results should be available
            ensure!(res.len() > 1_usize, ContractError::NotEnoughResults {});

            ensure!(
                !res.iter().any(|x| x == &false),
                ContractError::UnmetCondition {}
            );

            // Reset stored results after they meet our conditions
            let new: Vec<bool> = vec![];
            RESULTS.save(deps.storage, &new)?;

            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                    funds: vec![],
                }))
                .add_attribute("result", "sent by AND".to_string()))
        }
        // Just one result being true meets our condition
        LogicGate::OR => {
            ensure!(
                res.iter().any(|x| x == &true),
                ContractError::UnmetCondition {}
            );

            // Reset stored results after they meet our conditions
            let new: Vec<bool> = vec![];
            RESULTS.save(deps.storage, &new)?;

            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                    funds: vec![],
                }))
                .add_attribute("result", "sent by OR".to_string()))
        }
        // At least one result should be true, but not all of them
        LogicGate::XOR => {
            // At least two results should be available
            ensure!(res.len() > 1_usize, ContractError::NotEnoughResults {});

            ensure!(
                !res.iter()
                    .all(|x| x == &true && res.iter().any(|x| x == &true)),
                ContractError::UnmetCondition {}
            );

            // Reset stored results after they meet our conditions
            let new: Vec<bool> = vec![];
            RESULTS.save(deps.storage, &new)?;

            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                    funds: vec![],
                }))
                .add_attribute("result", "sent by XOR".to_string()))
        }
        // Only takes one input, takes false as true
        LogicGate::NOT => {
            ensure!(res.len() == 1 && !res[0], ContractError::UnmetCondition {});

            // Reset stored results after they meet our conditions
            let new: Vec<bool> = vec![];
            RESULTS.save(deps.storage, &new)?;

            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                    funds: vec![],
                }))
                .add_attribute("result", "sent by NOT".to_string()))
        }
        // Any input is valid unless they're all true
        LogicGate::NAND => {
            // At least two results should be available
            ensure!(res.len() > 1_usize, ContractError::NotEnoughResults {});

            ensure!(
                !res.iter().all(|x| x == &true),
                ContractError::UnmetCondition {}
            );
            // Reset stored results after they meet our conditions
            let new: Vec<bool> = vec![];
            RESULTS.save(deps.storage, &new)?;

            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                    funds: vec![],
                }))
                .add_attribute("result", "sent by NAND".to_string()))
        }
        // All input should be false
        LogicGate::NOR => {
            // At least two results should be available
            ensure!(res.len() > 1_usize, ContractError::NotEnoughResults {});

            ensure!(
                res.iter().all(|x| x == &false),
                ContractError::UnmetCondition {}
            );
            // Reset stored results after they meet our conditions
            let new: Vec<bool> = vec![];
            RESULTS.save(deps.storage, &new)?;

            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                    funds: vec![],
                }))
                .add_attribute("result", "sent by NOR".to_string()))
        }
        // Input should be all false or all true
        LogicGate::XNOR => {
            // At least two results should be available
            ensure!(res.len() > 1_usize, ContractError::NotEnoughResults {});

            ensure!(
                res.iter().all(|x| x == &false) || res.iter().all(|x| x == &true),
                ContractError::UnmetCondition {}
            );
            // Reset stored results after they meet our conditions
            let new: Vec<bool> = vec![];
            RESULTS.save(deps.storage, &new)?;

            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                    funds: vec![],
                }))
                .add_attribute("result", "sent by XNOR".to_string()))
        }
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
        QueryMsg::LogicGate {} => encode_binary(&query_logic_gate(deps)?),
        QueryMsg::Whitelist {} => encode_binary(&query_whitelist(deps)?),
        QueryMsg::Results {} => encode_binary(&query_results(deps)?),
    }
}

fn query_results(deps: Deps) -> Result<Vec<bool>, ContractError> {
    Ok(RESULTS.load(deps.storage)?)
}

fn query_logic_gate(deps: Deps) -> Result<LogicGate, ContractError> {
    Ok(LOGIC_GATE.load(deps.storage)?)
}

fn query_whitelist(deps: Deps) -> Result<Vec<String>, ContractError> {
    Ok(WHITELIST.load(deps.storage)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::app::AndrAddress;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::AND,
            whitelist: vec!["legit_address".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(
            WHITELIST.load(&deps.storage).unwrap(),
            vec!["legit_address".to_string()]
        );
        assert_eq!(LOGIC_GATE.load(&deps.storage).unwrap(), LogicGate::AND)
    }

    #[test]
    fn test_store_results_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::AND,
            whitelist: vec!["legit_address".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        let msg = ExecuteMsg::StoreResult { result: true };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_store_results_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::AND,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result)
    }

    #[test]
    fn test_interpret_works_and() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::AND,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address2", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap().identifier;
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by AND".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_or_all_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::OR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address2", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap().identifier;
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by OR".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_or_some_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::OR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap().identifier;
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by OR".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_xor_some_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::XOR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap().identifier;
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by XOR".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_xor_all_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::XOR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address2", &[]);
        // Interpret gets fired off since the number of results == the number of whitelisted addresses
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true, true];
        assert_eq!(result, expected_result);
        assert_eq!(err, ContractError::UnmetCondition {})
    }

    #[test]
    fn test_interpret_works_not() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NOT,
            whitelist: vec!["legit_address1".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address1", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap().identifier;
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by NOT".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_not_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NOT,
            whitelist: vec!["legit_address1".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(err, ContractError::UnmetCondition {});
    }

    #[test]
    fn test_interpret_not_more_than_one_input() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NOT,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![false];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        let _result = RESULTS.load(&deps.storage).unwrap();
        let _expected_result = vec![false, false];

        assert_eq!(err, ContractError::UnmetCondition {});
    }

    #[test]
    fn test_interpret_works_nand_some_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NAND,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);
        assert_eq!(
            res,
            Response::new().add_attribute("action", "stored result")
        );

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap().identifier;
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by NAND".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_nand_all_false() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NAND,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![false];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);
        println!("{:?}", res)
    }

    #[test]
    fn test_interpret_nand_all_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NAND,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address2", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true, true];
        assert_eq!(result, expected_result);
        assert_eq!(err, ContractError::UnmetCondition {});
    }

    #[test]
    fn test_interpret_works_nor_all_false() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NOR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![false];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::Interpret {};
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        let expected_response = ContractError::NoResults {};
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_nor_some_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NOR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true, false];
        assert_eq!(result, expected_result);
        assert_eq!(err, ContractError::UnmetCondition {});
    }

    #[test]
    fn test_interpret_nor_all_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::NOR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address2", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true, true];
        assert_eq!(result, expected_result);
        assert_eq!(err, ContractError::UnmetCondition {});
    }

    #[test]
    fn test_interpret_works_xnor_all_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::XNOR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address2", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap().identifier;
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by XNOR".to_string());
        assert_eq!(expected_response, res);
    }

    #[test]
    fn test_interpret_works_xnor_all_false() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::XNOR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![false];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result: Vec<bool> = vec![];
        assert_eq!(result, expected_result);

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap().identifier;
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by XNOR".to_string());
        assert_eq!(expected_response, res);
    }

    #[test]
    fn test_interpret_xnor_some_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::XNOR,
            whitelist: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: AndrAddress {
                identifier: "execute_ado".to_string(),
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::StoreResult { result: true };
        let info = mock_info("legit_address1", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true];
        assert_eq!(result, expected_result);

        let msg = ExecuteMsg::StoreResult { result: false };
        let info = mock_info("legit_address2", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        let result = RESULTS.load(&deps.storage).unwrap();
        let expected_result = vec![true, false];
        assert_eq!(result, expected_result);
        assert_eq!(err, ContractError::UnmetCondition {});
    }
}
