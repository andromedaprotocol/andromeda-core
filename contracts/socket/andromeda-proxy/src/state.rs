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
    let original_sender = ctx.amp_ctx.clone().map(|pkt| pkt.ctx.get_origin());
    let admins = ADMINS.load(ctx.deps.storage)?;

    let sender_to_check = match original_sender {
        Some(sender) => sender,
        // The user could eventually authorize his wallet address on the chain, so the message doesn't have to come from as an AMP Packet
        None => ctx.info.sender.to_string(),
    };

    // Check authority
    ensure!(
        admins.contains(&sender_to_check),
        ContractError::Unauthorized {}
    );
    Ok(())
}
