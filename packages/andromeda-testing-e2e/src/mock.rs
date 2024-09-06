use andromeda_std::os::kernel::QueryMsg;
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, Wallet};
use cw_orch_interchain::prelude::*;

use crate::{
    adodb::AdodbContract,
    chains::{ALL_CHAINS, TESTNET_MNEMONIC},
    economics::EconomicsContract,
    faucet::fund,
    kernel::KernelContract,
    vfs::VfsContract,
};

pub fn setup_interchain_env() -> DaemonInterchainEnv<ChannelCreationValidator> {
    let interchain_info: Vec<(ChainInfo, Option<String>)> = ALL_CHAINS
        .iter()
        .map(|chain| (chain.clone(), Some(TESTNET_MNEMONIC.to_string())))
        .collect();
    DaemonInterchainEnv::new(interchain_info, &ChannelCreationValidator).unwrap()
}

pub fn mock_app(chain: ChainInfo, mnemonic: &str) -> DaemonBase<Wallet> {
    let daemon = Daemon::builder(chain.clone()) // set the network to use
        .mnemonic(mnemonic)
        .build()
        .unwrap();

    fund(&daemon, &chain, 10000000000000);

    daemon
}

pub struct MockAndromeda {
    pub kernel_contract: KernelContract<DaemonBase<Wallet>>,
    pub adodb_contract: AdodbContract<DaemonBase<Wallet>>,
    pub vfs_contract: VfsContract<DaemonBase<Wallet>>,
    pub economics_contract: EconomicsContract<DaemonBase<Wallet>>,
}

impl MockAndromeda {
    pub fn install(daemon: &DaemonBase<Wallet>) -> MockAndromeda {
        let chain_name: String = daemon.chain_info().network_info.chain_name.to_string();

        // Upload and instantiate os ADOs
        let kernel_contract = KernelContract::new(daemon.clone());
        kernel_contract.upload().unwrap();
        kernel_contract.clone().init(chain_name);

        let adodb_contract = AdodbContract::new(daemon.clone());
        adodb_contract.upload().unwrap();
        adodb_contract
            .clone()
            .init(kernel_contract.addr_str().unwrap());

        let vfs_contract = VfsContract::new(daemon.clone());
        vfs_contract.upload().unwrap();
        vfs_contract
            .clone()
            .init(kernel_contract.addr_str().unwrap());

        let economics_contract = EconomicsContract::new(daemon.clone());
        economics_contract.upload().unwrap();
        economics_contract
            .clone()
            .init(kernel_contract.addr_str().unwrap());

        // register code ids in ado db
        adodb_contract.clone().execute_publish(
            adodb_contract.code_id().unwrap(),
            "adodb".to_string(),
            "0.1.0".to_string(),
        );

        adodb_contract.clone().execute_publish(
            vfs_contract.code_id().unwrap(),
            "vfs".to_string(),
            "0.1.0".to_string(),
        );

        adodb_contract.clone().execute_publish(
            kernel_contract.code_id().unwrap(),
            "kernel".to_string(),
            "0.1.0".to_string(),
        );

        // update kernel
        kernel_contract
            .clone()
            .execute_store_key_address("adodb".to_string(), adodb_contract.addr_str().unwrap());
        kernel_contract
            .clone()
            .execute_store_key_address("vfs".to_string(), vfs_contract.addr_str().unwrap());
        kernel_contract.clone().execute_store_key_address(
            "economics".to_string(),
            economics_contract.addr_str().unwrap(),
        );

        MockAndromeda {
            kernel_contract,
            adodb_contract,
            vfs_contract,
            economics_contract,
        }
    }

    pub fn new(daemon: &DaemonBase<Wallet>, kernel_address: impl Into<String>) -> MockAndromeda {
        let kernel_contract = KernelContract::new(daemon.clone());
        kernel_contract.set_address(&Addr::unchecked(kernel_address));

        let adodb_address: String = kernel_contract
            .query(&QueryMsg::KeyAddress {
                key: "adodb".to_string(),
            })
            .unwrap();
        let adodb_contract = AdodbContract::new(daemon.clone());
        adodb_contract.set_address(&Addr::unchecked(adodb_address));

        let vfs_address: String = kernel_contract
            .query(&QueryMsg::KeyAddress {
                key: "vfs".to_string(),
            })
            .unwrap();
        let vfs_contract = VfsContract::new(daemon.clone());
        vfs_contract.set_address(&Addr::unchecked(vfs_address));

        let economics_address: String = kernel_contract
            .query(&QueryMsg::KeyAddress {
                key: "economics".to_string(),
            })
            .unwrap();
        let economics_contract = EconomicsContract::new(daemon.clone());
        economics_contract.set_address(&Addr::unchecked(economics_address));

        MockAndromeda {
            kernel_contract,
            adodb_contract,
            vfs_contract,
            economics_contract,
        }
    }
}
