use andromeda_std::{
    ado_contract::ADOContract, common::response::get_reply_address, error::ContractError,
};
use cosmwasm_std::{DepsMut, Reply, Response};
use enum_repr::EnumRepr;

use crate::execute;
use crate::state::{ADO_ADDRESSES, ADO_DESCRIPTORS};

#[EnumRepr(type = "u64")]
pub enum ReplyId {
    ClaimOwnership = 101,
    AssignApp = 102,
    RegisterPath = 103,
    CrossChainCreate = 104,
}

pub fn on_component_instantiation(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let id = msg.id.to_string();

    let descriptor = ADO_DESCRIPTORS.load(deps.storage, &id)?;

    let addr_str = get_reply_address(msg)?;
    let addr = &deps.api.addr_validate(&addr_str)?;
    ADO_ADDRESSES.save(deps.storage, &descriptor.name, addr)?;

    let mut resp = Response::default();

    if !descriptor.name.starts_with('.') {
        let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
        let register_component_path_msg = execute::register_component_path(
            kernel_address,
            &deps.querier,
            descriptor.name,
            addr.clone(),
        )?;

        resp = resp.add_submessage(register_component_path_msg)
    }

    Ok(resp)
}
