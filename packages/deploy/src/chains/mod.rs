pub mod devnets;
pub mod mainnets;
pub mod testnets;

use cw_orch::prelude::ChainInfo;
use devnets::DEVNET_CHAINS;
use mainnets::MAINNET_CHAINS;
use testnets::TESTNET_CHAINS;

pub fn get_chain(chain: String) -> ChainInfo {
    [MAINNET_CHAINS, TESTNET_CHAINS, DEVNET_CHAINS]
        .concat()
        .iter()
        .find(|c| c.chain_id == chain || c.network_info.chain_name == chain)
        .unwrap()
        .clone()
}
