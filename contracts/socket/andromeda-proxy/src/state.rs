use andromeda_std::{common::context::ExecuteContext, error::ContractError};
use cosmwasm_std::ensure;
use cw_storage_plus::Item;

/// List of addresses that are allowed to operate this ADO
pub const ADMINS: Item<Vec<String>> = Item::new("admins");

/// If message is sent through AMP, it checks if the orignial sender is authorized.
/// If it's a direct message, it checks if the latest sender of the message is authorized.
pub(crate) fn authorize(ctx: &ExecuteContext) -> Result<(), ContractError> {
    // Fetch original sender of the amp packet (if available)
    let sender = ctx
        .amp_ctx
        .clone()
        .and_then(|pkt| pkt.ctx.get_previous_hops().first().cloned())
        .map(|hop| hop.address.to_string())
        .unwrap_or(ctx.info.sender.to_string());

    let admins = ADMINS.load(ctx.deps.storage)?;

    // Check authority
    ensure!(admins.contains(&sender), ContractError::Unauthorized {});
    Ok(())
}

pub(crate) const REPLY_ID: u64 = 1;
pub(crate) const BATCH_REPLY_ID_FAIL_ON_ERROR: u64 = 101;
pub(crate) const BATCH_REPLY_ID_IGNORE_ERROR: u64 = 201;
pub(crate) fn get_reply_id(fail_on_error: Option<bool>) -> u64 {
    match fail_on_error {
        Some(true) => BATCH_REPLY_ID_FAIL_ON_ERROR,
        Some(false) => BATCH_REPLY_ID_IGNORE_ERROR,
        None => REPLY_ID,
    }
}
