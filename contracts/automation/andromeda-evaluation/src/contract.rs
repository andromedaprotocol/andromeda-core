use crate::state::EXECUTE_ADO_ADDRESS;
use ado_base::state::ADOContract;
use andromeda_automation::evaluation::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, app::AndrAddress, encode_binary,
    error::ContractError, require,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, SubMsg, Uint128, WasmMsg,
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
    EXECUTE_ADO_ADDRESS.save(deps.storage, &msg.address)?;
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
        ExecuteMsg::Evaluate { first, second } => execute_evaluate(deps, env, info, first, second),
        ExecuteMsg::ChangeExecuteAddress { address } => {
            execute_change_execute_address(deps, env, info, address)
        }
    }
}

fn execute_change_execute_address(
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
    EXECUTE_ADO_ADDRESS.save(deps.storage, &address)?;
    Ok(Response::new().add_attribute("action", "changed_execute_ado_address"))
}

fn execute_evaluate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    first: Uint128,
    second: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;

    let res: bool = if first > second { true } else { false };

    // get the address of the ADO that will interpret our result
    let contract_addr = EXECUTE_ADO_ADDRESS.load(deps.storage)?.get_address(
        deps.api,
        &deps.querier,
        app_contract,
    )?;

    Ok(Response::new()
        .add_attribute("result", res.to_string())
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&andromeda_automation::execution::ExecuteMsg::Interpret { res })?,
            funds: vec![],
        }))))
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
        QueryMsg::ExecuteADO {} => encode_binary(&query_execute_ado_query(deps)?),
    }
}

fn query_execute_ado_query(deps: Deps) -> Result<String, ContractError> {
    let address = EXECUTE_ADO_ADDRESS.load(deps.storage)?;
    Ok(address.identifier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::app::AndrAddress;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies();
        let address = AndrAddress {
            identifier: "legit_address".to_string(),
        };
        let msg = InstantiateMsg { address };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = EXECUTE_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(
            addr,
            AndrAddress {
                identifier: "legit_address".to_string(),
            }
        )
    }

    #[test]
    fn test_evaluate() {
        let mut deps = mock_dependencies();
        let address = AndrAddress {
            identifier: "legit_address".to_string(),
        };
        let msg = InstantiateMsg { address };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let first = Uint128::new(40);
        let second = Uint128::new(30);
        let msg = ExecuteMsg::Evaluate { first, second };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected = Response::new()
            .add_attribute("result", "true".to_string())
            .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "legit_address".to_string(),
                msg: to_binary(&andromeda_automation::execution::ExecuteMsg::Interpret {
                    res: true,
                })
                .unwrap(),
                funds: vec![],
            })));
        assert_eq!(res, expected);
        println!("{:?}", res)
    }

    #[test]
    fn test_change_address_unauthorized() {
        let mut deps = mock_dependencies();
        let address = AndrAddress {
            identifier: "legit_address".to_string(),
        };
        let msg = InstantiateMsg { address };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let address = AndrAddress {
            identifier: "new_address".to_string(),
        };
        let msg = ExecuteMsg::ChangeExecuteAddress { address };
        let info = mock_info("random", &[]);

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_change_address() {
        let mut deps = mock_dependencies();
        let address = AndrAddress {
            identifier: "legit_address".to_string(),
        };
        let msg = InstantiateMsg { address };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let address = AndrAddress {
            identifier: "new_address".to_string(),
        };
        let msg = ExecuteMsg::ChangeExecuteAddress {
            address: address.clone(),
        };

        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let actual = EXECUTE_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(address, actual)
    }
}
