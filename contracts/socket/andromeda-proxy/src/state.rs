use andromeda_std::{common::context::ExecuteContext, error::ContractError};
use cosmwasm_std::{ensure, Addr, Uint128};
use cw_storage_plus::{Item, Map};

/// Maps cw20_address -> locked_amount
pub const LOCKED: Map<Addr, Uint128> = Map::new("locked");

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
