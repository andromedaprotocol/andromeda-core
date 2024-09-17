use crate::state::{REGISTRY, SERVICE_ADDRESS};
use andromeda_std::os::ibc_registry::{
    AllDenomInfoResponse, DenomInfo, DenomInfoResponse, ExecuteMsg, IBCDenomInfo, InstantiateMsg,
    QueryMsg,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, Storage,
};
use cw_storage_plus::Bound;
use std::collections::HashSet;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-ibc-registry";
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
            kernel_address: msg.kernel_address.into_string(),
            owner: msg.owner,
        },
    )?;

    // Save service address
    let service_address = msg
        .service_address
        .get_raw_address(&deps.as_ref())?
        .into_string();
    SERVICE_ADDRESS.save(deps.storage, &service_address)?;

    Ok(resp.add_attribute("service_address", service_address))
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

fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let res = match msg {
        ExecuteMsg::StoreDenomInfo { ibc_denom_info } => {
            execute_store_denom_info(ctx, ibc_denom_info)
        }
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res)
}

pub fn execute_store_denom_info(
    ctx: ExecuteContext,
    ibc_denom_info: Vec<IBCDenomInfo>,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    let service_address = SERVICE_ADDRESS.load(ctx.deps.storage)?;
    // Only servie address can call this message
    ensure!(
        service_address == sender.clone().into_string(),
        ContractError::Unauthorized {}
    );

    // Vector can't be empty
    ensure!(
        !ibc_denom_info.is_empty(),
        ContractError::NoDenomInfoProvided {}
    );

    let mut seen_denoms = HashSet::new(); // To track unique denoms

    for info in ibc_denom_info {
        let denom = info.denom;

        // Ensure the denom is valid (you could add further validation here if needed)
        if denom.trim().is_empty() {
            return Err(ContractError::EmptyDenom {});
        }

        // Ensure that the denom is formatted correctly. It should start with "ibc/"

        if !denom.starts_with("ibc/") {
            return Err(ContractError::InvalidDenom {
                msg: Some("The denom should start with 'ibc/'".to_string()),
            });
        }

        // Check for duplicates
        if !seen_denoms.insert(denom.clone()) {
            return Err(ContractError::DuplicateDenoms { denom }); // Return an error for duplicates
        }

        // Store the denom info securely
        REGISTRY.save(ctx.deps.storage, denom, &info.denom_info)?;
    }

    Ok(Response::new().add_attributes(vec![
        attr("action", "store_denom_info"),
        attr("sender", sender),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::DenomInfo { denom } => encode_binary(&get_denom_info(deps.storage, denom)?),
        QueryMsg::AllDenomInfo { limit, start_after } => {
            encode_binary(&get_all_denom_info(deps.storage, limit, start_after)?)
        }
    }
}

pub fn get_denom_info(
    storage: &dyn Storage,
    denom: String,
) -> Result<DenomInfoResponse, ContractError> {
    let denom_info = REGISTRY.load(storage, denom)?;
    Ok(DenomInfoResponse { denom_info })
}

pub fn get_all_denom_info(
    storage: &dyn Storage,
    limit: Option<u64>,
    start_after: Option<u64>,
) -> Result<AllDenomInfoResponse, ContractError> {
    // Convert `start_after` into a Bound if provided
    let min = Some(Bound::inclusive(start_after.unwrap_or(0).to_string()));

    // Set the limit, defaulting to 10 if none is provided
    let limit = limit.unwrap_or(100) as usize;

    // Query the registry with pagination
    let denom_info_iter: Result<Vec<(String, DenomInfo)>, StdError> = REGISTRY
        .range(storage, min, None, Order::Ascending)
        .take(limit)
        .collect();

    // Collect the results into a vector of `DenomInfo`
    let denom_info_list: Vec<DenomInfo> = denom_info_iter?
        .into_iter()
        .map(|(_, denom_info)| denom_info)
        .collect();

    Ok(AllDenomInfoResponse {
        denom_info: denom_info_list,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
