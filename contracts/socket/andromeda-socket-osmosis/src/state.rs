use andromeda_std::{
    amp::{messages::AMPCtx, AndrAddr, Recipient},
    common::context::ExecuteContext,
    error::ContractError,
};
use cosmwasm_std::{ensure, Addr, StdError, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct ForwardReplyState {
    /// Recipient
    pub recipient: Recipient,
    /// Refund Address
    pub refund_addr: Addr,
    /// Amp ctx to be used for ibc communication
    pub amp_ctx: Option<AMPCtx>,
    /// Offered denom to the osmosis
    pub from_denom: String,
    /// Asked denom returning from the osmosis
    pub to_denom: String,
}

pub const FORWARD_REPLY_STATE: Item<ForwardReplyState> = Item::new("forward_reply_state");

pub const SWAP_ROUTER: Item<AndrAddr> = Item::new("swap_router");

pub const PREV_BALANCE: Item<Uint128> = Item::new("prev_balance");

pub const SPENDER: Item<String> = Item::new("spender");

// pool creator to pool id
pub const WITHDRAW: Map<String, String> = Map::new("withdraw");

pub const MINT_AMOUNT: Item<Uint128> = Item::new("mint_amount");

pub const AUTHORIZED_ADDRESS: Item<String> = Item::new("authorized_address");

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
