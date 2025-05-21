use cw_orch::{
    environment::{ChainKind, NetworkInfo},
    prelude::ChainInfo,
};

pub const ANDROMEDA_TESTNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "andromeda-testnet",
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
    chain_name: "stargaze-testnet",
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

pub const OSMOSIS_TESTNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "osmosis-testnet",
    pub_address_prefix: "osmo",
    coin_type: 118u32,
};

pub const OSMOSIS_TESTNET: ChainInfo = ChainInfo {
    chain_id: "osmo-test-5",
    gas_denom: "uosmo",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["https://grpc.osmotest5.osmosis.zone:443"],
    lcd_url: Some("https://lcd.osmotest5.osmosis.zone:443"),
    network_info: OSMOSIS_TESTNET_NETWORK,
    kind: ChainKind::Testnet,
};

pub const ARCHWAY_TESTNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "archway-testnet",
    pub_address_prefix: "archway",
    coin_type: 118u32,
};

pub const ARCHWAY_TESTNET: ChainInfo = ChainInfo {
    chain_id: "constantine-3",
    gas_denom: "aconst",
    fcd_url: None,
    gas_price: 140000000000.0,
    grpc_urls: &["http://grpc.constantine.archway.io:443/"],
    lcd_url: Some("https://api.constantine.archway.io/"),
    network_info: ARCHWAY_TESTNET_NETWORK,
    kind: ChainKind::Testnet,
};

pub const NEUTRON_TESTNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "neutron-testnet",
    pub_address_prefix: "neutron",
    coin_type: 118u32,
};

/// <https://github.com/cosmos/chain-registry/blob/master/testnets/neutrontestnet/chain.json>
pub const NEUTRON_TESTNET: ChainInfo = ChainInfo {
    kind: ChainKind::Testnet,
    chain_id: "pion-1",
    gas_denom: "untrn",
    gas_price: 0.02,
    grpc_urls: &["http://grpc-palvus.pion-1.ntrn.tech:80"],
    network_info: NEUTRON_TESTNET_NETWORK,
    lcd_url: Some("https://rest-palvus.pion-1.ntrn.tech"),
    fcd_url: None,
};

pub const TESTNET_CHAINS: &[ChainInfo] = &[
    ANDROMEDA_TESTNET,
    STARGAZE_TESTNET,
    OSMOSIS_TESTNET,
    ARCHWAY_TESTNET,
    NEUTRON_TESTNET,
];
