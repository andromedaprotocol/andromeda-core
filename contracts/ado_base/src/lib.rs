mod execute;
#[cfg(test)]
pub mod mock_querier;
pub mod modules;
mod query;
pub mod state;
#[cfg(feature = "withdraw")]
mod withdraw;

pub use crate::state::ADOContract;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;
    use andromeda_protocol::{
        ado_base::{AndromedaMsg, AndromedaQuery, InstantiateMsg},
        error::ContractError,
    };
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        ADOContract::default().instantiate(deps, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: AndromedaMsg,
    ) -> Result<Response, ContractError> {
        ADOContract::default().execute(deps, env, info, msg, execute)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: AndromedaQuery) -> Result<Binary, ContractError> {
        ADOContract::default().query(deps, env, msg, query)
    }
}
