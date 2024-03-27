use andromeda_std::{common::response::get_reply_address, error::ContractError};
use cosmwasm_std::{ensure_eq, Addr, DepsMut, Reply, Response};

use crate::state::{ADO_ADDRESSES, ADO_DESCRIPTORS};

pub fn on_component_instantiation(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let id = msg.id.to_string();

    let descriptor = ADO_DESCRIPTORS.load(deps.storage, &id)?;

    let addr_str = get_reply_address(msg)?;
    let addr = &deps.api.addr_validate(&addr_str)?;
    let saved_addr = ADO_ADDRESSES.load(deps.storage, &descriptor.name)?;
    ensure_eq!(
        addr,
        saved_addr,
        ContractError::Instantiate2AddressMismatch {
            expected: saved_addr,
            received: Addr::unchecked(addr_str)
        }
    );

    let resp = Response::default();

    Ok(resp)
}
