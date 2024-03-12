use andromeda_std::ado_base::InstantiateMsg as BaseInstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::encode_binary;
use andromeda_std::common::reply::ReplyId;
use andromeda_std::error::{from_semver, ContractError};
use andromeda_std::os::economics::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
#[allow(unused_imports)]
use cosmwasm_std::{
    attr, coin, ensure, entry_point, from_binary, to_binary, Addr, BankMsg, Binary, CosmosMsg,
    Deps, DepsMut, Empty, Env, MessageInfo, Response, Storage, SubMsg, Uint128, WasmMsg,
};
use cosmwasm_std::{Reply, StdError};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use semver::Version;

use crate::{execute, query};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-economics";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "economics".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),

            kernel_address: msg.kernel_address,
            owner: msg.owner,
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

    match ReplyId::from_repr(msg.id) {
        Some(ReplyId::Cw20WithdrawMsg) => Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        ))),
        Some(ReplyId::PayFee) => Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        ))),
        _ => Ok(Response::default()),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit { address } => execute::deposit_native(deps, info, address),
        ExecuteMsg::PayFee { payee, action } => execute::pay_fee(deps, env, info, payee, action),
        ExecuteMsg::Withdraw { amount, asset } => {
            execute::withdraw_native(deps, info, amount, asset)
        }
        ExecuteMsg::WithdrawCW20 { amount, asset } => {
            execute::withdraw_cw20(deps, info, amount, asset)
        }
        ExecuteMsg::Receive(cw20msg) => cw20_receive(deps, env, info, cw20msg),
    }
}

pub fn cw20_receive(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&msg.sender)?;
    let amount = msg.amount;

    match from_binary::<Cw20HookMsg>(&msg.msg)? {
        Cw20HookMsg::Deposit { address } => {
            execute::cw20_deposit(deps, info, sender, amount, address)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

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

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Balance { address, asset } => {
            Ok(to_binary(&query::balance(deps, address, asset)?)?)
        }
        // Base queries
        QueryMsg::Version {} => encode_binary(&ADOContract::default().query_version(deps)?),
        QueryMsg::Type {} => encode_binary(&ADOContract::default().query_type(deps)?),
        QueryMsg::Owner {} => encode_binary(&ADOContract::default().query_contract_owner(deps)?),
        QueryMsg::KernelAddress {} => {
            encode_binary(&ADOContract::default().query_kernel_address(deps)?)
        }
    }
}
