use andromeda_fungible_tokens::ics20::ChannelInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

use crate::error::ContractError;
pub const ADMIN: Admin = Admin::new("admin");

pub const CONFIG: Item<Config> = Item::new("ics20_config");

// Used to pass info from the ibc_packet_receive to the reply handler
pub const REPLY_ARGS: Item<ReplyArgs> = Item::new("reply_args");

/// static info on one channel that doesn't change
pub const CHANNEL_INFO: Map<&str, ChannelInfo> = Map::new("channel_info");

/// indexed by (channel_id, denom) maintaining the balance of the channel in that currency
pub const CHANNEL_STATE: Map<(&str, &str), ChannelState> = Map::new("channel_state");

/// Every cw20 contract we allow to be sent is stored here, possibly with a gas_limit
pub const ALLOW_LIST: Map<&Addr, AllowInfo> = Map::new("allow_list");

#[cw_serde]
#[derive(Default)]
pub struct ChannelState {
    pub outstanding: Uint128,
    pub total_sent: Uint128,
}

#[cw_serde]
pub struct Config {
    pub default_timeout: u64,
    pub default_gas_limit: Option<u64>,
}

#[cw_serde]
pub struct AllowInfo {
    pub gas_limit: Option<u64>,
}

#[cw_serde]
pub struct ReplyArgs {
    pub channel: String,
    pub denom: String,
    pub amount: Uint128,
}

pub fn increase_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
) -> Result<(), ContractError> {
    CHANNEL_STATE.update(storage, (channel, denom), |orig| -> StdResult<_> {
        let mut state = orig.unwrap_or_default();
        state.outstanding += amount;
        state.total_sent += amount;
        Ok(state)
    })?;
    Ok(())
}

pub fn reduce_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
) -> Result<(), ContractError> {
    CHANNEL_STATE.update(
        storage,
        (channel, denom),
        |orig| -> Result<_, ContractError> {
            // this will return error if we don't have the funds there to cover the request (or no denom registered)
            let mut cur = orig.ok_or(ContractError::InsufficientFunds {})?;
            cur.outstanding = cur
                .outstanding
                .checked_sub(amount)
                .or(Err(ContractError::InsufficientFunds {}))?;
            Ok(cur)
        },
    )?;
    Ok(())
}

// this is like increase, but it only "un-subtracts" (= adds) outstanding, not total_sent
// calling `reduce_channel_balance` and then `undo_reduce_channel_balance` should leave state unchanged.
pub fn undo_reduce_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
) -> Result<(), ContractError> {
    CHANNEL_STATE.update(storage, (channel, denom), |orig| -> StdResult<_> {
        let mut state = orig.unwrap_or_default();
        state.outstanding += amount;
        Ok(state)
    })?;
    Ok(())
}
