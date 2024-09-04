use cw_orch::environment::{ChainInfo, ChainKind, NetworkInfo};

pub const TERRA_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "terra",
    pub_address_prefix: "terra",
    coin_type: 330u32,
};

pub const LOCAL_TERRA: ChainInfo = ChainInfo {
    kind: ChainKind::Local,
    chain_id: "localterraa-1",
    gas_denom: "uluna",
    gas_price: 0.15,
    grpc_urls: &["http://localhost:20331"],
    network_info: TERRA_NETWORK,
    lcd_url: None,
    fcd_url: None,
};

pub const OSMO_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "osmo",
    pub_address_prefix: "osmo",
    coin_type: 118u32,
};

pub const LOCAL_OSMO: ChainInfo = ChainInfo {
    kind: ChainKind::Local,
    chain_id: "localosmosisa-1",
    gas_denom: "uosmo",
    gas_price: 0.15,
    grpc_urls: &["http://localhost:20321"],
    network_info: OSMO_NETWORK,
    lcd_url: Some("http://localhost:20221"),
    fcd_url: None,
};

pub const ALL_CHAINS: &[ChainInfo] = &[LOCAL_OSMO];
