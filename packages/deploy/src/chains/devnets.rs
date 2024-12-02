use cw_orch::{
    environment::{ChainKind, NetworkInfo},
    prelude::ChainInfo,
};

pub const OSMOSIS_DEVNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "osmosis-devnet",
    pub_address_prefix: "osmo",
    coin_type: 118u32,
};

pub const OSMOSIS_DEVNET: ChainInfo = ChainInfo {
    chain_id: "localosmosisa-1",
    gas_denom: "uosmo",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["http://164.90.212.168:20321/"],
    lcd_url: Some("http://164.90.212.168:20221/"),
    network_info: OSMOSIS_DEVNET_NETWORK,
    kind: ChainKind::Testnet,
};

pub const WASM_DEVNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "wasm-devnet",
    pub_address_prefix: "wasm",
    coin_type: 118u32,
};

pub const WASM_DEVNET: ChainInfo = ChainInfo {
    chain_id: "localwasma-1",
    gas_denom: "ustake",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["http://164.90.212.168:20341/"],
    lcd_url: Some("http://164.90.212.168:20241/"),
    network_info: WASM_DEVNET_NETWORK,
    kind: ChainKind::Testnet,
};

pub const ANDROMEDA_DEVNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "andromeda-devnet",
    pub_address_prefix: "andr",
    coin_type: 118u32,
};

pub const ANDROMEDA_DEVNET: ChainInfo = ChainInfo {
    chain_id: "localandromedaa-1",
    gas_denom: "uandr",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["http://164.90.212.168:20311/"],
    lcd_url: Some("http://164.90.212.168:20211/"),
    network_info: ANDROMEDA_DEVNET_NETWORK,
    kind: ChainKind::Testnet,
};

pub const DEVNET_CHAINS: &[ChainInfo] = &[OSMOSIS_DEVNET, WASM_DEVNET, ANDROMEDA_DEVNET];
