#[cfg(not(feature = "library"))]
use crate::state::{
    CURVE_CONFIG, DEFAULT_CONSTANT_VALUE, DEFAULT_MULTIPLE_VARIABLE_VALUE, RESTRICTION,
};
use andromeda_modules::curve::{
    CurveConfig, CurveId, CurveRestriction, ExecuteMsg, GetCurveConfigResponse,
    GetPlotYFromXResponse, GetRestrictionResponse, InstantiateMsg, QueryMsg,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{actions::call_action, context::ExecuteContext, encode_binary},
    error::ContractError,
};

use cosmwasm_std::{
    ensure, entry_point, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, Storage,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-curve";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    RESTRICTION.save(deps.storage, &msg.restriction)?;
    CURVE_CONFIG.save(deps.storage, &msg.curve_config)?;

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);
    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action = msg.as_ref().to_string();

    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;

    let res = match msg.clone() {
        ExecuteMsg::UpdateCurveConfig { curve_config } => {
            execute_update_curve_config(ctx, curve_config, action)
        }
        ExecuteMsg::UpdateRestriction { restriction } => {
            execute_update_restriction(ctx, restriction, action)
        }
        ExecuteMsg::Reset {} => execute_reset(ctx, action),
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

pub fn execute_update_curve_config(
    ctx: ExecuteContext,
    curve_config: CurveConfig,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    CURVE_CONFIG.update(ctx.deps.storage, |_| {
        Ok::<CurveConfig, ContractError>(curve_config)
    })?;

    Ok(Response::new()
        .add_attribute("action", action)
        .add_attribute("sender", sender))
}

pub fn execute_update_restriction(
    ctx: ExecuteContext,
    restriction: CurveRestriction,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );
    RESTRICTION.save(ctx.deps.storage, &restriction)?;

    Ok(Response::new()
        .add_attribute("action", action)
        .add_attribute("sender", sender))
}

pub fn execute_reset(ctx: ExecuteContext, action: String) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );

    CURVE_CONFIG.remove(ctx.deps.storage);

    Ok(Response::new().add_attribute("action", action))
}

pub fn has_permission(storage: &dyn Storage, addr: &Addr) -> Result<bool, ContractError> {
    let is_operator = ADOContract::default().is_owner_or_operator(storage, addr.as_str())?;
    let allowed = match RESTRICTION.load(storage)? {
        CurveRestriction::Private => is_operator,
        CurveRestriction::Public => true,
    };
    Ok(is_operator || allowed)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetCurveConfig {} => encode_binary(&query_curve_config(deps.storage)?),
        QueryMsg::GetRestriction {} => encode_binary(&query_restriction(deps.storage)?),
        QueryMsg::GetPlotYFromX { x_value } => {
            encode_binary(&query_plot_y_from_x(deps.storage, x_value)?)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn query_curve_config(storage: &dyn Storage) -> Result<GetCurveConfigResponse, ContractError> {
    let curve_config = CURVE_CONFIG.load(storage)?;
    Ok(GetCurveConfigResponse { curve_config })
}

pub fn query_restriction(storage: &dyn Storage) -> Result<GetRestrictionResponse, ContractError> {
    let restriction = RESTRICTION.load(storage)?;
    Ok(GetRestrictionResponse { restriction })
}

pub fn query_plot_y_from_x(
    storage: &dyn Storage,
    x_value: f64,
) -> Result<GetPlotYFromXResponse, ContractError> {
    let curve_config = CURVE_CONFIG.load(storage)?;

    let y_value = match curve_config {
        CurveConfig::ExpConfig {
            curve_id,
            base_value,
            multiple_variable_value,
            constant_value,
        } => {
            let curve_id_f64 = match curve_id {
                CurveId::Growth => 1_f64,
                CurveId::Decay => -1_f64,
            };
            let base_value_f64 = base_value as f64;
            let constant_value_f64 = constant_value.unwrap_or(DEFAULT_CONSTANT_VALUE) as f64;
            let multiple_variable_value_f64 =
                multiple_variable_value.unwrap_or(DEFAULT_MULTIPLE_VARIABLE_VALUE) as f64;

            (constant_value_f64
                * base_value_f64.powf(curve_id_f64 * multiple_variable_value_f64 * x_value))
            .to_string()
        }
    };

    Ok(GetPlotYFromXResponse { y_value })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
