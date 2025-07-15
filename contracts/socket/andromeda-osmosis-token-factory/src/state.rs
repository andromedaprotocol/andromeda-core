use andromeda_std::{common::context::ExecuteContext, error::ContractError};
use cosmwasm_std::{ensure, StdError, Uint128};
use cw_storage_plus::Item;

pub const MINT_AMOUNT: Item<Uint128> = Item::new("mint_amount");
pub const AUTHORIZED_ADDRESS: Item<String> = Item::new("authorized_address");

pub const OSMOSIS_MSG_CREATE_DENOM_ID: u64 = 1;
pub const OSMOSIS_MSG_MINT_ID: u64 = 2;
pub const OSMOSIS_MSG_BURN_ID: u64 = 3;

/// Ensures that the message comes from the kernel, and that the origin of the amp packet is the same as the authorized address
pub fn is_authorized(ctx: &ExecuteContext) -> Result<(), ContractError> {
    // Only accepts messages coming from the kernel
    let amp_packet = ctx
        .amp_ctx
        .clone()
        .ok_or(ContractError::Std(StdError::generic_err(
            "AMP context not found".to_string(),
        )))?;
    let origin = amp_packet.ctx.get_origin();

    let authorized_address = AUTHORIZED_ADDRESS.load(ctx.deps.storage)?;
    // Make sure origin is authorized
    ensure!(origin == authorized_address, ContractError::Unauthorized {});
    Ok(())
}
