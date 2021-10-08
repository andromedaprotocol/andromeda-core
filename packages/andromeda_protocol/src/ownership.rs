use cosmwasm_std::{attr, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage};
use cw_storage_plus::Item;

use crate::require::require;

const CONTRACT_OWNER: Item<String> = Item::new("contractowner");

pub fn is_contract_owner(storage: &dyn Storage, addr: String) -> StdResult<bool> {
    let owner = CONTRACT_OWNER.load(storage)?;

    Ok(addr.eq(&owner))
}

pub fn store_owner(storage: &mut dyn Storage, addr: &String) -> StdResult<()> {
    CONTRACT_OWNER.save(storage, addr)
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    new_owner: String,
) -> StdResult<Response> {
    require(
        is_contract_owner(deps.storage, info.sender.to_string())?,
        StdError::generic_err(
            "Ownership of this contract can only be transferred by the current owner",
        ),
    )?;

    store_owner(deps.storage, &new_owner.clone())?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_owner"),
        attr("value", new_owner.clone()),
    ]))
}
