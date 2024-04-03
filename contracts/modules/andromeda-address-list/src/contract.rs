use andromeda_modules::address_list::IncludesAddressResponse;
#[cfg(not(feature = "library"))]
use andromeda_modules::address_list::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};

use cosmwasm_std::{attr, ensure, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError};
use cosmwasm_std::{entry_point, to_json_binary};
use cw2::set_contract_version;
use cw_utils::nonpayable;

use crate::state::{add_address, includes_address, remove_address, IS_INCLUSIVE};
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-address-list";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    IS_INCLUSIVE.save(deps.storage, &msg.is_inclusive)?;

    let inst_resp = ADOContract::default().instantiate(
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
        ExecuteMsg::AddAddress { address } => execute_add_address(ctx, address),
        ExecuteMsg::RemoveAddress { address } => execute_remove_address(ctx, address),
        ExecuteMsg::AddAddresses { addresses } => execute_add_addresses(ctx, addresses),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_add_address(ctx: ExecuteContext, address: String) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    add_address(deps.storage, &address)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "add_address"),
        attr("address", address),
    ]))
}

fn execute_remove_address(ctx: ExecuteContext, address: String) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    remove_address(deps.storage, &address);

    Ok(Response::new().add_attributes(vec![
        attr("action", "remove_address"),
        attr("address", address),
    ]))
}

const MAX_ADDRESSES_SIZE: usize = 100;

fn execute_add_addresses(
    ctx: ExecuteContext,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;

    ensure!(
        !addresses.is_empty(),
        ContractError::Std(StdError::generic_err("addresses cannot be empty"))
    );
    ensure!(
        addresses.len() <= MAX_ADDRESSES_SIZE,
        ContractError::Std(StdError::generic_err(
            "addresses length cannot be more than 100"
        ))
    );

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    for address in addresses.clone() {
        add_address(deps.storage, &address)?;
    }

    Ok(Response::new().add_attributes(vec![
        attr("action", "add_addresses"),
        attr("addresses", addresses.join(",")),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrHook(msg) => handle_andr_hook(deps, msg),
        QueryMsg::IncludesAddress { address } => encode_binary(&query_address(deps, &address)?),
        QueryMsg::IsInclusive {} => encode_binary(&handle_is_inclusive(deps)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn handle_andr_hook(deps: Deps, msg: AndromedaHook) -> Result<Binary, ContractError> {
    match msg {
        AndromedaHook::OnExecute { sender, .. } => {
            let is_included = includes_address(deps.storage, &sender)?;
            let is_inclusive = IS_INCLUSIVE.load(deps.storage)?;
            if is_included != is_inclusive {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(to_json_binary(&None::<Response>)?)
            }
        }
        _ => Ok(to_json_binary(&None::<Response>)?),
    }
}

fn handle_is_inclusive(deps: Deps) -> Result<bool, ContractError> {
    let is_inclusive = IS_INCLUSIVE.load(deps.storage)?;
    Ok(is_inclusive)
}

fn query_address(deps: Deps, address: &str) -> Result<IncludesAddressResponse, ContractError> {
    Ok(IncludesAddressResponse {
        included: includes_address(deps.storage, address)?,
    })
}
