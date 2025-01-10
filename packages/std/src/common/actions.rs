use crate::{
    ado_contract::{permissioning::is_context_permissioned, ADOContract},
    amp::messages::AMPPkt,
    error::ContractError,
    os::aos_querier::AOSQuerier,
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
        is_context_permissioned(deps, info, env, amp_ctx, action)?,
        ContractError::Unauthorized {}
    );

    let payee = if let Some(amp_ctx) = amp_ctx.clone() {
        deps.api.addr_validate(amp_ctx.ctx.get_origin().as_str())?
    } else {
        info.sender.clone()
    };

    let adodb_addr = ADOContract::default().get_adodb_address(deps.storage, &deps.querier)?;
    let contract_info = deps
        .querier
        .query_wasm_contract_info(env.contract.address.clone())?;
    let code_id = contract_info.code_id;
    // If ADO type is not found, return default response
    let ado_type = AOSQuerier::ado_type_getter(&deps.querier, &adodb_addr, code_id)?;
    if let Some(ado_type) = ado_type {
        if AOSQuerier::action_fee_getter(&deps.querier, &adodb_addr, &ado_type, action)?.is_some() {
            let fee_msg = ADOContract::default().pay_fee(
                deps.storage,
                &deps.querier,
                action.to_owned(),
                payee,
            )?;
            Ok(Response::default().add_submessage(fee_msg))
        } else {
            Ok(Response::default())
        }
    } else {
        Ok(Response::default())
    }
}
