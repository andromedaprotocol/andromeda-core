use crate::error::ContractError;
use crate::modules::{hooks::HookResponse, Module};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

//Redundant? Can maybe use `Modules` struct?
pub fn generate_instantiate_msgs(
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    modules: Vec<Option<impl Module>>,
) -> Result<HookResponse, ContractError> {
    let mut resp = HookResponse::default();

    for module in modules.into_iter().flatten() {
        let hook_resp = module.on_instantiate(deps, info.clone(), env.clone())?;
        resp = resp.add_resp(hook_resp);
    }

    Ok(resp)
}

pub fn unwrap_or_err<T>(val_opt: Option<T>, err: ContractError) -> Result<T, ContractError> {
    match val_opt {
        Some(val) => Ok(val),
        None => Err(err),
    }
}

pub fn merge_responses(resp_a: Response, resp_b: Response) -> Response {
    resp_a
        .add_attributes(resp_b.attributes)
        .add_submessages(resp_b.messages)
}
