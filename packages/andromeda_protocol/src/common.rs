use crate::error::ContractError;
use crate::modules::{hooks::HookResponse, Module};
use cosmwasm_std::{DepsMut, Env, MessageInfo};

//Redundant? Can maybe use `Modules` struct?
pub fn generate_instantiate_msgs(
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    modules: Vec<Option<impl Module>>,
) -> Result<HookResponse, ContractError> {
    let mut resp = HookResponse::default();

    for module_opt in modules {
        match module_opt {
            Some(module) => {
                let hook_resp = module.on_instantiate(&deps, info.clone(), env.clone())?;
                resp = resp.add_resp(hook_resp);
            }
            None => {}
        }
    }

    Ok(resp)
}
