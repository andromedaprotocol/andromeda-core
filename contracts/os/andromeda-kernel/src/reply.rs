use crate::state::ADO_OWNER;
use andromeda_std::{
    ado_base::AndromedaMsg, common::response::get_reply_address, error::ContractError,
};
use cosmwasm_std::{to_binary, wasm_execute, DepsMut, Empty, Reply, Response, SubMsg};
use enum_repr::EnumRepr;

#[EnumRepr(type = "u64")]
pub enum ReplyId {
    AMPMsg = 1,
    CreateADO = 2,
    UpdateOwnership = 3,
}

/// Handles the reply from an ADO creation
///
/// Sends an execute message to assign the new owner to the ADO. Will error if the owner is assigned in the ADO creation message.
pub fn on_reply_create_ado(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let new_owner = ADO_OWNER.load(deps.as_ref().storage)?;
    let ado_addr = get_reply_address(msg)?;

    let msg = AndromedaMsg::UpdateOwner {
        address: new_owner.to_string(),
    };
    let wasm_msg = wasm_execute(ado_addr.clone(), &msg, vec![])?;
    let sub_msg: SubMsg<Empty> =
        SubMsg::reply_on_success(wasm_msg, ReplyId::UpdateOwnership as u64);
    Ok(Response::default()
        .add_submessage(sub_msg)
        .set_data(to_binary(&ado_addr)?))
}
