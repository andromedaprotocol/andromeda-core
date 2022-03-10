mod execute;
pub mod modules;
mod query;
pub mod state;
mod withdraw;

use crate::state::ADOContract;

use andromeda_protocol::{
    ado_base::{AndromedaMsg, AndromedaQuery, InstantiateMsg},
    error::ContractError,
};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};

// This makes a conscious choice on the various generics used by the contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ADOContract::default().instantiate(deps, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    ADOContract::default().execute(deps, env, info, msg, execute)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: AndromedaQuery) -> Result<Binary, ContractError> {
    ADOContract::default().query(deps, env, msg, query)
}
