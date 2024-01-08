use andromeda_modules::address_list::IncludesAddressResponse;
#[cfg(not(feature = "library"))]
use andromeda_modules::address_list::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg},
    ado_contract::ADOContract,
    common::{context::ExecuteContext, encode_binary},
    error::{from_semver, ContractError},
};

use cosmwasm_std::{attr, ensure, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError};
use cosmwasm_std::{entry_point, to_binary};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::nonpayable;
use semver::Version;

use crate::state::{add_address, includes_address, remove_address, IS_INCLUSIVE};
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-addresslist";
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
        info,
        BaseInstantiateMsg {
            ado_type: "address-list".to_string(),
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
                Ok(to_binary(&None::<Response>)?)
            }
        }
        _ => Ok(to_binary(&None::<Response>)?),
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
