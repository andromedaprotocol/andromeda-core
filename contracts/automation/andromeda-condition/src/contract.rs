use ado_base::state::ADOContract;
use andromeda_automation::evaluation::QueryMsg as EvaluationQueryMsg;
use andromeda_automation::{
    condition::{ExecuteMsg, InstantiateMsg, LogicGate, MigrateMsg, QueryMsg},
    execute,
};

use common::app::GetAddress;
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use cosmwasm_std::{
    ensure, entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Reply, Response, StdError, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::nonpayable;
use semver::Version;

use crate::state::{EVAL_ADOS, EXECUTE_ADO, LOGIC_GATE};

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
    EVAL_ADOS.save(deps.storage, &msg.eval_ados)?;
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
            kernel_address: msg.kernel_address,
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
        ExecuteMsg::GetResults {} => execute_get_results(deps, env, info),
        ExecuteMsg::UpdateExecuteADO { address } => {
            execute_update_execute_ado(deps, env, info, address)
        }
        ExecuteMsg::UpdateEvalAdos { addresses } => {
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
    EVAL_ADOS.save(deps.storage, &addresses)?;

    Ok(Response::new().add_attribute("action", "updated_whitelist"))
}

fn execute_update_execute_ado(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
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
        .add_attribute("new_address", address))
}

fn execute_get_results(
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

    let eval_ados = EVAL_ADOS.load(deps.storage)?;

    // Query Eval for results
    let mut eval_results: Vec<bool> = vec![];

    for i in eval_ados.into_iter() {
        // Get the address
        let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
        let contract_addr = i.get_address(deps.api, &deps.querier, app_contract)?;
        let result: bool = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr,
            msg: to_binary(&EvaluationQueryMsg::Evaluation {})?,
        }))?;
        eval_results.push(result);
    }

    execute_interpret(deps, _env, info, eval_results)
}

fn execute_interpret(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    res: Vec<bool>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;

    // Load logic gate
    let logic = LOGIC_GATE.load(deps.storage)?;
    // Load results
    ensure!(!res.is_empty(), ContractError::NoResults {});

    let contract_addr =
        EXECUTE_ADO
            .load(deps.storage)?
            .get_address(deps.api, &deps.querier, app_contract)?;
    match logic {
        LogicGate::And =>
        // We don't want to find a false bool, so we want it to return false
        {
            // At least two results should be available
            ensure!(res.len() > 1_usize, ContractError::NotEnoughResults {});

            if res.iter().any(|x| x == &false) {
                Ok(Response::new().add_attribute("result", "And unmet condition".to_string()))
            } else {
                Ok(Response::new()
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                        funds: vec![],
                    }))
                    .add_attribute("result", "sent by And".to_string()))
            }
        }
        // Just one result being true meets our condition
        LogicGate::Or => {
            if !res.iter().any(|x| x == &true) {
                Ok(Response::new().add_attribute("result", "Or unmet condition".to_string()))
            } else {
                Ok(Response::new()
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                        funds: vec![],
                    }))
                    .add_attribute("result", "sent by Or".to_string()))
            }
        }
        // At least one result should be true, but not all of them
        LogicGate::Xor => {
            // At least two results should be available
            ensure!(res.len() > 1_usize, ContractError::NotEnoughResults {});
            if res
                .iter()
                .all(|x| x == &true || !res.iter().any(|x| x == &true))
            {
                Ok(Response::new().add_attribute("result", "Xor unmet condition".to_string()))
            } else {
                Ok(Response::new()
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                        funds: vec![],
                    }))
                    .add_attribute("result", "sent by Xor".to_string()))
            }
        }

        // Only takes one input, takes false as true
        LogicGate::Not => {
            if res.len() != 1 || res[0] {
                Ok(Response::new().add_attribute("result", "Not unmet condition".to_string()))
            } else {
                Ok(Response::new()
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                        funds: vec![],
                    }))
                    .add_attribute("result", "sent by Not".to_string()))
            }
        }
        // Any input is valid unless they're all true
        LogicGate::Nand => {
            // At least two results should be available

            if res.len() <= 1_usize || res.iter().all(|x| x == &true) {
                Ok(Response::new().add_attribute("result", "Nand unmet condition".to_string()))
            } else {
                Ok(Response::new()
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                        funds: vec![],
                    }))
                    .add_attribute("result", "sent by Nand".to_string()))
            }
        }
        // All input should be false
        LogicGate::Nor => {
            // At least two results should be available
            if res.len() <= 1_usize || !res.iter().all(|x| x == &false) {
                Ok(Response::new().add_attribute("result", "Nor unmet condition".to_string()))
            } else {
                Ok(Response::new()
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                        funds: vec![],
                    }))
                    .add_attribute("result", "sent by Nor".to_string()))
            }
        }
        // Input should be all false or all true
        LogicGate::Xnor => {
            // At least two results should be available
            if res.len() <= 1_usize
                || !res.iter().all(|x| x == &false) && !res.iter().all(|x| x == &true)
            {
                Ok(Response::new().add_attribute("result", "Xnor unmet condition".to_string()))
            } else {
                Ok(Response::new()
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&execute::ExecuteMsg::Execute {})?,
                        funds: vec![],
                    }))
                    .add_attribute("result", "sent by Xnor".to_string()))
            }
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
    StdError::generic_err(format!("Semver: {err}"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::LogicGate {} => encode_binary(&query_logic_gate(deps)?),
        QueryMsg::EvalAdos {} => encode_binary(&query_whitelist(deps)?),
    }
}

fn query_logic_gate(deps: Deps) -> Result<LogicGate, ContractError> {
    Ok(LOGIC_GATE.load(deps.storage)?)
}

fn query_whitelist(deps: Deps) -> Result<Vec<String>, ContractError> {
    Ok(EVAL_ADOS.load(deps.storage)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::mock_dependencies_custom;
    use cosmwasm_std::testing::{mock_env, mock_info};

    // legit_address1 always returns true
    // legit_address2 always returns false

    #[test]
    fn test_instantiate_works() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::And,
            eval_ados: vec!["legit_address".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let eval_ados = EVAL_ADOS.load(&deps.storage).unwrap();
        assert_eq!(eval_ados[0], "legit_address".to_string());
        assert_eq!(LOGIC_GATE.load(&deps.storage).unwrap(), LogicGate::And)
    }

    #[test]
    fn test_interpret_unauthorized() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::And,
            eval_ados: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("legit_address1", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err)
    }

    #[test]
    fn test_interpret_works_and() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::And,
            eval_ados: vec!["legit_address1".to_string(), "legit_address1".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by And".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_or_all_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Or,
            eval_ados: vec!["legit_address1".to_string(), "legit_address1".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Or".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_or_some_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Or,
            eval_ados: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Or".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_xor_some_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Xor,
            eval_ados: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Xor".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_xor_all_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Xor,
            eval_ados: vec!["legit_address1".to_string(), "legit_address1".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_res = Response::new().add_attribute("result", "Xor unmet condition");

        assert_eq!(expected_res, res)
    }

    #[test]
    fn test_interpret_works_not() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Not,
            eval_ados: vec!["legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Not".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_not_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Not,
            eval_ados: vec!["legit_address1".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_res = Response::new().add_attribute("result", "Not unmet condition");

        assert_eq!(expected_res, res)
    }

    #[test]
    fn test_interpret_not_more_than_one_input() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Not,
            eval_ados: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_res = Response::new().add_attribute("result", "Not unmet condition");

        assert_eq!(expected_res, res)
    }

    #[test]
    fn test_interpret_works_nand_some_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Nand,
            eval_ados: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Nand".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_nand_all_false() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Nand,
            eval_ados: vec!["legit_address2".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Nand".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_nand_all_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Nand,
            eval_ados: vec!["legit_address1".to_string(), "legit_address1".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_res = Response::new().add_attribute("result", "Nand unmet condition");

        assert_eq!(expected_res, res)
    }

    #[test]
    fn test_interpret_works_nor_all_false() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Nor,
            eval_ados: vec!["legit_address2".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Nor".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_nor_some_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Nor,
            eval_ados: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_res = Response::new().add_attribute("result", "Nor unmet condition");

        assert_eq!(expected_res, res)
    }

    #[test]
    fn test_interpret_nor_all_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Nor,
            eval_ados: vec!["legit_address1".to_string(), "legit_address1".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_res = Response::new().add_attribute("result", "Nor unmet condition");

        assert_eq!(expected_res, res)
    }

    #[test]
    fn test_interpret_works_xnor_all_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Xnor,
            eval_ados: vec!["legit_address1".to_string(), "legit_address1".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Xnor".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_works_xnor_all_false() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Xnor,
            eval_ados: vec!["legit_address2".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let contract_addr = EXECUTE_ADO.load(&deps.storage).unwrap();
        let expected_response = Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&execute::ExecuteMsg::Execute {}).unwrap(),
                funds: vec![],
            }))
            .add_attribute("result", "sent by Xnor".to_string());
        assert_eq!(expected_response, res)
    }

    #[test]
    fn test_interpret_xnor_some_true() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            logic_gate: LogicGate::Xnor,
            eval_ados: vec!["legit_address1".to_string(), "legit_address2".to_string()],
            execute_ado: "execute_ado".to_string(),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::GetResults {};
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_res = Response::new().add_attribute("result", "Xnor unmet condition");

        assert_eq!(expected_res, res)
    }
}
