#[cfg(not(feature = "library"))]
use andromeda_modules::shunting::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use andromeda_modules::shunting::{ShuntingObject, ShuntingResponse};
use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg},
    ado_contract::ADOContract,
    common::{context::ExecuteContext, encode_binary},
    error::{from_semver, ContractError},
};

use cosmwasm_std::{attr, ensure, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cosmwasm_std::{entry_point, to_binary};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::nonpayable;
use semver::Version;

use crate::state::SHUNTING;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-shunting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    SHUNTING.save(
        deps.storage,
        &ShuntingObject {
            expression: msg.expression,
        },
    )?;

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "shunting".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    Ok(inst_resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let _contract = ADOContract::default();
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateExpression { expression } => execute_update_expression(ctx, expression),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_update_expression(
    ctx: ExecuteContext,
    expression: String,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut obj = SHUNTING.load(deps.storage)?;
    obj.expression = expression.clone();
    SHUNTING.save(deps.storage, &obj)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_expression"),
        attr("expression", expression),
    ]))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrHook(msg) => handle_andr_hook(deps, msg),
        QueryMsg::EvalExpression {} => encode_binary(&handle_eval_expression(deps)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn handle_andr_hook(deps: Deps, msg: AndromedaHook) -> Result<Binary, ContractError> {
    match msg {
        _ => Ok(to_binary(&None::<Response>)?),
    }
}

fn handle_eval_expression(deps: Deps) -> Result<ShuntingResponse, ContractError> {
    let obj = SHUNTING.load(deps.storage)?;
    let result = obj.eval().unwrap().to_string();
    Ok(ShuntingResponse { result })
}
