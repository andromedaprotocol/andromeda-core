use cw_orch::{
    environment::{ChainInfo, ChainKind, NetworkInfo, NetworkInfoOwned},
    prelude::ChainInfoOwned,
};

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

pub const ANDR_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "andr",
    pub_address_prefix: "andr",
    coin_type: 118u32,
};

pub const LOCAL_ANDR: ChainInfo = ChainInfo {
    kind: ChainKind::Local,
    chain_id: "localandromedaa-1",
    gas_denom: "uandr",
    gas_price: 0.15,
    grpc_urls: &["http://localhost:20311"],
    network_info: ANDR_NETWORK,
    lcd_url: Some("http://localhost:20211"),
    fcd_url: None,
};

pub fn chain_to_owned(chain: &ChainInfo) -> ChainInfoOwned {
    ChainInfoOwned {
        kind: chain.kind.clone(),
        chain_id: chain.chain_id.to_string(),
        gas_denom: chain.gas_denom.to_string(),
        gas_price: chain.gas_price,
        grpc_urls: chain.grpc_urls.iter().map(|url| url.to_string()).collect(),
        network_info: NetworkInfoOwned {
            chain_name: chain.network_info.chain_name.to_string(),
            pub_address_prefix: chain.network_info.pub_address_prefix.to_string(),
            coin_type: chain.network_info.coin_type,
        },
        lcd_url: chain.lcd_url.map(|url| url.to_string()),
        fcd_url: chain.fcd_url.map(|url| url.to_string()),
    }
}
pub const ALL_CHAINS: &[ChainInfo] = &[LOCAL_OSMO, LOCAL_ANDR, LOCAL_TERRA];
pub const TESTNET_MNEMONIC: &str = "family album bird seek tilt color pill danger message abuse manual tent almost ridge boost blast high comic core quantum spoon coconut oyster remove";
