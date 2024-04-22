use crate::{
    ado_contract::{permissioning::is_context_permissioned, ADOContract},
    amp::messages::AMPPkt,
    error::ContractError,
};
use cosmwasm_std::{ensure, DepsMut, Env, MessageInfo, Response};

pub fn call_action(
    deps: &mut DepsMut,
    info: &MessageInfo,
    env: &Env,
    amp_ctx: &Option<AMPPkt>,
    action: &str,
) -> Result<Response, ContractError> {
    ensure!(
        is_context_permissioned(deps.storage, info, env, amp_ctx, action)?,
        ContractError::Unauthorized {}
    );

    let payee = if let Some(amp_ctx) = amp_ctx.clone() {
        deps.api.addr_validate(amp_ctx.ctx.get_origin().as_str())?
    } else {
        info.sender.clone()
    };

    let fee_msg =
        ADOContract::default().pay_fee(deps.storage, &deps.querier, action.to_owned(), payee)?;

    Ok(Response::default().add_submessage(fee_msg))
}
