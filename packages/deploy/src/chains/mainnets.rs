use cw_orch::{
    environment::{ChainKind, NetworkInfo},
    prelude::ChainInfo,
};

pub const ANDROMEDA_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "andromeda",
    pub_address_prefix: "andr",
    coin_type: 118u32,
};

pub const ANDROMEDA_MAINNET: ChainInfo = ChainInfo {
    chain_id: "andromeda-1",
    gas_denom: "uandr",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["https://andromeda-grpc.polkachu.com:21290"],
    lcd_url: Some("https://andromeda-api.polkachu.com"),
    network_info: ANDROMEDA_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const STARGAZE_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "stargaze",
    pub_address_prefix: "stars",
    coin_type: 118u32,
};

pub const STARGAZE_MAINNET: ChainInfo = ChainInfo {
    chain_id: "stargaze-1",
    gas_denom: "ustars",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["https://stargaze-grpc.polkachu.com:21290"],
    lcd_url: Some("https://stargaze-api.polkachu.com"),
    network_info: STARGAZE_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const NEUTRON_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "neutron",
    pub_address_prefix: "neutron",
    coin_type: 118u32,
};

pub const NEUTRON_MAINNET: ChainInfo = ChainInfo {
    chain_id: "neutron-1",
    gas_denom: "untrn",
    fcd_url: None,
    gas_price: 0.0053,
    grpc_urls: &["https://neutron-grpc.publicnode.com:443"],
    lcd_url: Some("https://neutron-api.polkachu.com"),
    network_info: NEUTRON_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const MIGALOO_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "migaloo",
    pub_address_prefix: "migaloo",
    coin_type: 118u32,
};

pub const MIGALOO_MAINNET: ChainInfo = ChainInfo {
    chain_id: "migaloo-1",
    gas_denom: "uwhale",
    fcd_url: None,
    gas_price: 1.0,
    grpc_urls: &["https://migaloo-grpc.polkachu.com:443"],
    lcd_url: Some("https://migaloo-api.polkachu.com"),
    network_info: MIGALOO_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const JUNO_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "juno",
    pub_address_prefix: "juno",
    coin_type: 118u32,
};

pub const JUNO_MAINNET: ChainInfo = ChainInfo {
    chain_id: "juno-1",
    gas_denom: "ujuno",
    fcd_url: None,
    gas_price: 0.075,
    grpc_urls: &["https://juno-grpc.polkachu.com:443"],
    lcd_url: Some("https://juno-api.polkachu.com"),
    network_info: JUNO_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const DESMOS_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "desmos",
    pub_address_prefix: "desmos",
    coin_type: 118u32,
};

pub const DESMOS_MAINNET: ChainInfo = ChainInfo {
    chain_id: "desmos",
    gas_denom: "udsm",
    fcd_url: None,
    gas_price: 0.001,
    grpc_urls: &["https://desmos-grpc.lavenderfive.com:443/"],
    lcd_url: Some("https://api.mainnet.desmos.network"),
    network_info: DESMOS_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const CHIHUAHUA_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "chihuahua",
    pub_address_prefix: "chihuahua",
    coin_type: 118u32,
};

pub const CHIHUAHUA_MAINNET: ChainInfo = ChainInfo {
    chain_id: "chihuahua-1",
    gas_denom: "uhuahua",
    fcd_url: None,
    gas_price: 1.0,
    grpc_urls: &["https://chiahuahua-grpc.polkachu.com:21290"],
    lcd_url: Some("https://chihuahua-api.polkachu.com"),
    network_info: CHIHUAHUA_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const UMEE_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "umee",
    pub_address_prefix: "umee",
    coin_type: 118u32,
};

pub const UMEE_MAINNET: ChainInfo = ChainInfo {
    chain_id: "umee-1",
    gas_denom: "uumee",
    fcd_url: None,
    gas_price: 0.1,
    grpc_urls: &["https://umee-grpc.polkachu.com:21290"],
    lcd_url: Some("https://umee-api.polkachu.com"),
    network_info: UMEE_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const NIBIRU_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "nibiru",
    pub_address_prefix: "nibi",
    coin_type: 118u32,
};

pub const NIBIRU_MAINNET: ChainInfo = ChainInfo {
    chain_id: "cataclysm-1",
    gas_denom: "unibi",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["https://nibiru-grpc.polkachu.com:21290"],
    lcd_url: Some("https://nibiru-api.polkachu.com"),
    network_info: NIBIRU_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const COREUM_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "coreum",
    pub_address_prefix: "core",
    coin_type: 118u32,
};

pub const COREUM_MAINNET: ChainInfo = ChainInfo {
    chain_id: "coreum-1",
    gas_denom: "ucore",
    fcd_url: None,
    gas_price: 0.0625,
    grpc_urls: &["https://coreum-grpc.polkachu.com:21290"],
    lcd_url: Some("https://coreum-api.polkachu.com"),
    network_info: COREUM_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const ARCHWAY_MAINNET_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "archway",
    pub_address_prefix: "archway",
    coin_type: 118u32,
};

pub const ARCHWAY_MAINNET: ChainInfo = ChainInfo {
    chain_id: "archway-1",
    gas_denom: "aarch",
    fcd_url: None,
    gas_price: 140000000000.0,
    grpc_urls: &["https://archway-grpc.polkachu.com:21290"],
    lcd_url: Some("https://archway-api.polkachu.com"),
    network_info: ARCHWAY_MAINNET_NETWORK,
    kind: ChainKind::Mainnet,
};

pub const MAINNET_CHAINS: &[ChainInfo] = &[
    ANDROMEDA_MAINNET,
    STARGAZE_MAINNET,
    NEUTRON_MAINNET,
    MIGALOO_MAINNET,
    JUNO_MAINNET,
    DESMOS_MAINNET,
    CHIHUAHUA_MAINNET,
    UMEE_MAINNET,
    NIBIRU_MAINNET,
    COREUM_MAINNET,
    ARCHWAY_MAINNET,
];
