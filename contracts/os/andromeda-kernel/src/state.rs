use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const CHAIN_NAME_KEY: &str = "chain_name";

#[cw_serde]
pub struct ChannelInfo {
    pub kernel_address: String,
    pub ics20_channel_id: Option<String>,
    pub direct_channel_id: Option<String>,
    pub supported_modules: Vec<String>,
}

pub const KERNEL_ADDRESSES: Map<&str, Addr> = Map::new("kernel_addresses");
pub const ENV_VARIABLES: Map<&str, String> = Map::new("kernel_env_variables");

//Temporary storage for creating a new ADO to assign a new owner
pub const ADO_OWNER: Item<Addr> = Item::new("ado_owner");

pub const CHANNELS: Map<&str, ChannelInfo> = Map::new("kernel_channels");
