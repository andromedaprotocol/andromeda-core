#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, Storage, attr};

use andromeda_modules::string_utils::{Delimiter, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_modules::string_utils::GetSplitResultResponse;
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{
        context::ExecuteContext, encode_binary,
        actions::call_action,
        call_action::get_action_name,
    },
    error::ContractError,
};
use crate::state::SPLIT_RESULT;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-string-utils";
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

    let action = get_action_name(CONTRACT_NAME, msg.as_ref());

    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;

    let res = match msg {
        ExecuteMsg::Split { input, delimiter } => execute_split(ctx, input, delimiter, action),
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

pub fn execute_split(
    ctx: ExecuteContext,
    input: String,
    delimiter: Delimiter,
    action: String,
) -> Result<Response, ContractError> {

    let sender = ctx.info.sender.clone();

    match delimiter {
        Delimiter::WhiteSpace => {
            let parts: Vec<String> = input.split_whitespace().map(|part| part.to_string()).collect();
            SPLIT_RESULT.save(ctx.deps.storage, &parts)?;
        },
        Delimiter::Other { limiter } => {
            let parts: Vec<String> = input.split(&limiter).map(|part| part.to_string()).collect();
            SPLIT_RESULT.save(ctx.deps.storage, &parts)?;
        },
    }
    
    Ok(
        Response::new().add_attributes(vec![
        attr("action", action),
        attr("sender", sender),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetSplitResult {} => encode_binary(&get_split_result(deps.storage)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn get_split_result(storage: &dyn Storage) -> Result<GetSplitResultResponse, ContractError> {
    let split_result = SPLIT_RESULT.may_load(storage)?;
    match split_result {
        Some(result) => Ok(GetSplitResultResponse { split_result: result }),
        None => Ok(GetSplitResultResponse { split_result: vec!["No split result found.".to_string()] }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
