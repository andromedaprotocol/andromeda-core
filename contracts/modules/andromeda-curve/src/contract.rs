#[cfg(not(feature = "library"))]
use crate::state::{
    BASE_VALUE, CONSTANT_VALUE, CURVE_ID, CURVE_TYPE, DEFAULT_CONSTANT_VALUE,
    DEFAULT_MULTIPLE_VARIABLE_VALUE, IS_CONFIGURED_EXP, MULTIPLE_VARIABLE_VALUE, RESTRICTION,
};
use andromeda_modules::curve::{
    CurveId, CurveRestriction, CurveType, ExecuteMsg, GetConfigurationExpResponse,
    GetCurveTypeResponse, GetPlotYFromXResponse, GetRestrictionResponse, InstantiateMsg, QueryMsg,
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
    CURVE_TYPE.save(deps.storage, &msg.curve_type)?;

    IS_CONFIGURED_EXP.save(deps.storage, &false)?;

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
        ExecuteMsg::UpdateCurveType { curve_type } => {
            execute_update_curve_type(ctx, curve_type, action)
        }
        ExecuteMsg::UpdateRestriction { restriction } => {
            execute_update_restriction(ctx, restriction, action)
        }
        ExecuteMsg::ConfigureExponential {
            curve_id,
            base_value,
            multiple_variable_value,
            constant_value,
        } => execute_configure_exponential(
            ctx,
            curve_id,
            base_value,
            multiple_variable_value,
            constant_value,
            action,
        ),
        ExecuteMsg::Reset {} => execute_reset(ctx, action),
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

pub fn execute_update_curve_type(
    ctx: ExecuteContext,
    curve_type: CurveType,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    CURVE_TYPE.save(ctx.deps.storage, &curve_type)?;

    CURVE_ID.remove(ctx.deps.storage);
    BASE_VALUE.remove(ctx.deps.storage);
    MULTIPLE_VARIABLE_VALUE.remove(ctx.deps.storage);
    CONSTANT_VALUE.remove(ctx.deps.storage);
    IS_CONFIGURED_EXP.save(ctx.deps.storage, &false)?;

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

pub fn execute_configure_exponential(
    ctx: ExecuteContext,
    curve_id: CurveId,
    base_value: u64,
    multiple_variable_value: Option<u64>,
    constant_value: Option<u64>,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );

    CURVE_ID.save(ctx.deps.storage, &curve_id)?;
    BASE_VALUE.save(ctx.deps.storage, &base_value)?;

    if let Some(value) = multiple_variable_value {
        MULTIPLE_VARIABLE_VALUE.save(ctx.deps.storage, &value)?;
    } else {
        MULTIPLE_VARIABLE_VALUE.save(ctx.deps.storage, &DEFAULT_MULTIPLE_VARIABLE_VALUE)?;
    }

    if let Some(value) = constant_value {
        CONSTANT_VALUE.save(ctx.deps.storage, &value)?;
    } else {
        CONSTANT_VALUE.save(ctx.deps.storage, &DEFAULT_CONSTANT_VALUE)?;
    }

    IS_CONFIGURED_EXP.save(ctx.deps.storage, &true)?;

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

    let is_configured_exp: bool = IS_CONFIGURED_EXP.load(ctx.deps.storage)?;
    ensure!(is_configured_exp, ContractError::UnmetCondition {});

    CURVE_ID.remove(ctx.deps.storage);
    BASE_VALUE.remove(ctx.deps.storage);
    MULTIPLE_VARIABLE_VALUE.remove(ctx.deps.storage);
    CONSTANT_VALUE.remove(ctx.deps.storage);
    IS_CONFIGURED_EXP.save(ctx.deps.storage, &false)?;

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
        QueryMsg::GetCurveType {} => encode_binary(&query_curve_type(deps.storage)?),
        QueryMsg::GetConfigurationExp {} => encode_binary(&query_configuration_exp(deps.storage)?),
        QueryMsg::GetRestriction {} => encode_binary(&query_restriction(deps.storage)?),
        QueryMsg::GetPlotYFromX { x_value } => {
            encode_binary(&query_plot_y_from_x(deps.storage, x_value)?)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn query_curve_type(storage: &dyn Storage) -> Result<GetCurveTypeResponse, ContractError> {
    let curve_type = CURVE_TYPE.load(storage)?;
    Ok(GetCurveTypeResponse { curve_type })
}

pub fn query_configuration_exp(
    storage: &dyn Storage,
) -> Result<GetConfigurationExpResponse, ContractError> {
    let is_configured_exp: bool = IS_CONFIGURED_EXP.load(storage)?;
    ensure!(is_configured_exp, ContractError::UnmetCondition {});

    let curve_id = CURVE_ID.load(storage)?;
    let base_value = BASE_VALUE.load(storage)?;
    let constant_value = CONSTANT_VALUE.load(storage)?;
    let multiple_variable_value = MULTIPLE_VARIABLE_VALUE.load(storage)?;
    Ok(GetConfigurationExpResponse {
        curve_id,
        base_value,
        multiple_variable_value,
        constant_value,
    })
}

pub fn query_restriction(storage: &dyn Storage) -> Result<GetRestrictionResponse, ContractError> {
    let restriction = RESTRICTION.load(storage)?;
    Ok(GetRestrictionResponse { restriction })
}

pub fn query_plot_y_from_x(
    storage: &dyn Storage,
    x_value: f64,
) -> Result<GetPlotYFromXResponse, ContractError> {
    let is_configured_exp: bool = IS_CONFIGURED_EXP.load(storage)?;
    ensure!(is_configured_exp, ContractError::UnmetCondition {});

    let curve_id = match CURVE_ID.load(storage)? {
        CurveId::Growth => 1 as f64,
        CurveId::Decay => (-1) as f64,
    };

    let base_value = BASE_VALUE.load(storage)? as f64;
    let constant_value = CONSTANT_VALUE.load(storage)? as f64;
    let multiple_variable_value = MULTIPLE_VARIABLE_VALUE.load(storage)? as f64;

    let y_value = (constant_value * base_value.powf(curve_id * multiple_variable_value * x_value))
        .to_string();

    Ok(GetPlotYFromXResponse { y_value })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
