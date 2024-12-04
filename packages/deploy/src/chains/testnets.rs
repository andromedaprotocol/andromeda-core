use cw_orch::{
    environment::{ChainKind, NetworkInfo},
    prelude::ChainInfo,
};

pub const ANDROMEDA_TESTNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "andromeda",
    pub_address_prefix: "andr",
    coin_type: 118u32,
};

pub const ANDROMEDA_TESTNET: ChainInfo = ChainInfo {
    chain_id: "galileo-4",
    gas_denom: "uandr",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["http://137.184.182.11:9090/"],
    lcd_url: Some("http://137.184.182.11:1317/"),
    network_info: ANDROMEDA_TESTNET_NETWORK,
    kind: ChainKind::Testnet,
};

pub const STARGAZE_TESTNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "stargaze",
    pub_address_prefix: "stars",
    coin_type: 118u32,
};

pub const STARGAZE_TESTNET: ChainInfo = ChainInfo {
    chain_id: "elgafar-1",
    gas_denom: "ustars",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["http://grpc-1.elgafar-1.stargaze-apis.com:26660"],
    lcd_url: Some("https://rest.elgafar-1.stargaze-apis.com/"),
    network_info: STARGAZE_TESTNET_NETWORK,
    kind: ChainKind::Testnet,
};

pub const TESTNET_CHAINS: &[ChainInfo] = &[ANDROMEDA_TESTNET, STARGAZE_TESTNET];
