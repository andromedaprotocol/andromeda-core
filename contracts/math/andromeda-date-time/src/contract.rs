use andromeda_std::andr_execute_fn;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError};

use andromeda_math::date_time::GetDateTimeResponse;
use andromeda_math::date_time::{ExecuteMsg, InstantiateMsg, QueryMsg, Timezone};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::encode_binary,
    error::ContractError,
};
use chrono::{DateTime, Datelike, Timelike, Weekday};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-date-time";
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

    Ok(resp)
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    ADOContract::default().execute(ctx, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetDateTime { timezone } => encode_binary(&get_date_time(env, timezone)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn get_date_time(
    env: Env,
    timezone: Option<Timezone>,
) -> Result<GetDateTimeResponse, ContractError> {
    let timestamp = env.block.time.seconds() as i64;
    let timezone_i64 = timezone.unwrap_or(Timezone::Utc) as i64;
    let offset = timezone_i64.checked_mul(36).unwrap();
    let local_timestamp = timestamp.checked_add(offset).unwrap();
    let local_datetime = DateTime::from_timestamp(local_timestamp, 0).unwrap();

    let day_of_week = match local_datetime.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    };

    let date_time = format!(
        "{:04}-{:02}-{:02} {:02}-{:02}-{:02}",
        local_datetime.year(),
        local_datetime.month(),
        local_datetime.day(),
        local_datetime.hour(),
        local_datetime.minute(),
        local_datetime.second(),
    );

    Ok(GetDateTimeResponse {
        day_of_week: day_of_week.to_string(),
        date_time,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
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
