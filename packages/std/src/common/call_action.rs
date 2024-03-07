use crate::ado_contract::permissioning::is_context_permissioned;
use crate::{amp::messages::AMPPkt, error::ContractError};
use cosmwasm_std::{DepsMut, Env, MessageInfo};

pub fn call_action(
    deps: &mut DepsMut,
    info: &MessageInfo,
    env: &Env,
    ctx: &Option<AMPPkt>,
    action: impl Into<String>,
) -> Result<(), ContractError> {
    // Check if permissioned
    if !is_context_permissioned(deps, info, env, ctx, action)? {
        return Err(ContractError::Unauthorized {});
    };
    // Input other potential checks here

    Ok(())
}
