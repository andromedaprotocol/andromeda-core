use andromeda_std::os::kernel::{ChannelInfo, Ics20PacketInfo, RefundData};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};

pub const TRIGGER_KEY: &str = "trigger_key";

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
pub const _ENV_VARIABLES: Map<&str, String> = Map::new("kernel_env_variables");
pub const CURR_CHAIN: Item<String> = Item::new("kernel_curr_chain");

//Temporary storage for creating a new ADO to assign a new owner
pub const ADO_OWNER: Item<Addr> = Item::new("ado_owner");

// Mapping from chain name to channel info
pub const CHAIN_TO_CHANNEL: Map<&str, ChannelInfo> = Map::new("kernel_channels");
// Mapping from channel id to chain name
pub const CHANNEL_TO_CHAIN: Map<&str, String> = Map::new("kernel_channel_name");

/// Used to store the most recent outgoing IBC hooks packet
///
/// Removed when a reply is received for the packet
pub const OUTGOING_IBC_HOOKS_PACKETS: Item<Vec<IBCHooksPacketSendState>> =
    Item::new("OUTGOING_IBC_HOOKS_PACKETS");
pub const OUTGOING_IBC_PACKETS: Map<(&String, u64), OutgoingPacket> =
    Map::new("outgoing_ibc_packets");
pub const IBC_FUND_RECOVERY: Map<&Addr, Vec<Coin>> = Map::new("ibc_fund_recovery");

/// Used to temporarily store the most recent ExecuteMsg with the corresponding Coin to be sent in a reply for ICS20 transfer
pub const PENDING_MSG_AND_FUNDS: Item<Ics20PacketInfo> = Item::new("pending_execute_msg");

/// Used to store sequence/channel against an ExecuteMsg, to be sent after an ack of ICS20
/// Key is (channel_id, sequence)
pub const CHANNEL_TO_EXECUTE_MSG: Map<(String, u64), Ics20PacketInfo> =
    Map::new("channel_to_execute_msg");

/// Used to temporarily store the most recent ExecuteMsg with the corresponding Coin to be sent in a reply for ICS20 transfer
pub const REFUND_DATA: Item<RefundData> = Item::new("refund_data");
