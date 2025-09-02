use andromeda_socket::osmosis::Pool;
use andromeda_socket::osmosis::{PoolIdAndParams, SpenderAndParams};
use andromeda_std::amp::{messages::AMPCtx, AndrAddr, Recipient};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
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

// Spender's address and the pool's parameters
pub const SPENDER_AND_PARAMS: Item<SpenderAndParams> = Item::new("spender_and_params");

#[cw_serde]
pub struct WithdrawState {
    pub sender: String,
    pub pool_id: String,
}

// Store withdrawal state for reply handling
pub const WITHDRAW_STATE: Item<WithdrawState> = Item::new("withdraw_state");

// pool creator to pool id and the pool's parameters (keeping for backward compatibility)
pub const WITHDRAW: Map<String, PoolIdAndParams> = Map::new("withdraw");

// pool creator to pool id and the pool's parameters
pub const POOLS_CREATED: Map<String, Vec<PoolIdAndParams>> = Map::new("pools_created");

// Pool id to pool info
pub const POOL_INFO: Map<String, Pool> = Map::new("pool_info");
