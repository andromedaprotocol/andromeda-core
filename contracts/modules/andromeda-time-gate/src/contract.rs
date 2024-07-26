#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, ensure, attr, Storage, Addr};

use andromeda_modules::time_gate::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_modules::time_gate::{GateAddresses, GateTime};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{
        context::ExecuteContext, encode_binary,
        actions::call_action,
    },
    error::ContractError,
};
use crate::state::{GATE_ADDRESSES, GATE_TIME};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-time-gate";
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

    let gate_time = msg.gate_time;
    gate_time.validate().unwrap();

    GATE_ADDRESSES.save(deps.storage, &msg.gate_addresses)?;
    GATE_TIME.save(deps.storage, &gate_time)?;

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
        },
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

    let res = match msg {
        ExecuteMsg::SetGateTime { gate_time } => execute_set_gate_time(ctx, gate_time, action),
        ExecuteMsg::UpdateGateAddresses { new_gate_addresses } => execute_update_gate_addressess(ctx, new_gate_addresses, action),
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

fn execute_set_gate_time (
    ctx: ExecuteContext,
    gate_time: GateTime,
    action: String,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let old_gate_time = GATE_TIME.load(deps.storage)?;

    ensure!(
        old_gate_time != gate_time,
        ContractError::InvalidParameter { error: Some("Same as existed gate time".to_string())}
    );

    gate_time.validate().unwrap();

    GATE_TIME.save(deps.storage, &gate_time)?;

    Ok(
        Response::new().add_attributes(vec![
        attr("action", action),
        attr("sender", info.sender),
    ]))
}

fn execute_update_gate_addressess (
    ctx: ExecuteContext,
    new_gate_addresses: GateAddresses,
    action: String,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let old_gate_addresses = GATE_ADDRESSES.load(deps.storage)?;

    ensure!(
        old_gate_addresses != new_gate_addresses,
        ContractError::InvalidParameter { error: Some("Same as existed gate addresses".to_string())}
    );

    GATE_ADDRESSES.save(deps.storage, &new_gate_addresses)?;

    Ok(
        Response::new().add_attributes(vec![
        attr("action", action),
        attr("sender", info.sender),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetGateAddresses {} => encode_binary(&get_gate_addresses(deps.storage)?),
        QueryMsg::GetGateTime {} => encode_binary(&get_gate_time(deps.storage)?),
        QueryMsg::GetPathByCurrentTime {} => encode_binary(&get_path(deps, deps.storage, env)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn get_gate_addresses(storage: &dyn Storage) -> Result<GateAddresses, ContractError> {
    let gate_addresses = GATE_ADDRESSES.load(storage)?;
    Ok(gate_addresses)
}

pub fn get_gate_time(storage: &dyn Storage) -> Result<GateTime, ContractError> {
    let gate_time = GATE_TIME.load(storage)?;
    Ok(gate_time)
}

pub fn get_path(deps: Deps, storage: &dyn Storage, env: Env) -> Result<Addr, ContractError> {
    let gate_time = GATE_TIME.load(storage)?;
    let gate_addresses = GATE_ADDRESSES.load(storage)?;
    
    let GateTime { year, month, day, hour, minute, second } = gate_time;

    let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
    let time = NaiveTime::from_hms_nano_opt(hour, minute, second, 0).unwrap();
    let datetime = NaiveDateTime::new(date, time);

    let duration = datetime.signed_duration_since(
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(), 
            NaiveTime::from_hms_nano_opt(0, 0, 0, 0).unwrap()
        )
    );

    let total_nanoseconds = duration.num_nanoseconds().unwrap();

    let current_time = env.block.time.nanos();

    match current_time >= (total_nanoseconds as u64) {
        true => Ok(Addr::from(gate_addresses.ado_1.get_raw_address(&deps).unwrap())),
        false => Ok(Addr::from(gate_addresses.ado_2.get_raw_address(&deps).unwrap()))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
