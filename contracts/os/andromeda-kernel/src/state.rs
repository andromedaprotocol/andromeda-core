use andromeda_std::os::kernel::ChannelInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct IBCHooksPacketSendState {
    pub channel_id: String,
    pub recovery_addr: Addr,
    pub amount: Coin,
}

#[cw_serde]
pub struct OutgoingPacket {
    pub recovery_addr: Addr,
    pub amount: Coin,
}

pub const KERNEL_ADDRESSES: Map<&str, Addr> = Map::new("kernel_addresses");
pub const ENV_VARIABLES: Map<&str, String> = Map::new("kernel_env_variables");
pub const CURR_CHAIN: Item<String> = Item::new("kernel_curr_chain");

//Temporary storage for creating a new ADO to assign a new owner
pub const ADO_OWNER: Item<Addr> = Item::new("ado_owner");

pub const CHANNELS: Map<&str, ChannelInfo> = Map::new("kernel_channels");

/// Used to store the most recent outgoing IBC hooks packet
///
/// Removed when a reply is received for the packet
pub const OUTGOING_IBC_HOOKS_PACKETS: Item<Vec<IBCHooksPacketSendState>> =
    Item::new("OUTGOING_IBC_HOOKS_PACKETS");
pub const OUTGOING_IBC_PACKETS: Map<(&String, u64), OutgoingPacket> =
    Map::new("outgoing_ibc_packets");
pub const IBC_FUND_RECOVERY: Map<&Addr, Vec<Coin>> = Map::new("ibc_fund_recovery");
