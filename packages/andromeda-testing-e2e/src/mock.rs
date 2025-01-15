use andromeda_adodb::ADODBContract;
use andromeda_economics::EconomicsContract;
use andromeda_kernel::KernelContract;
use andromeda_std::os::{
    adodb::{self, ExecuteMsgFns},
    economics,
    kernel::{self, ExecuteMsgFns as KernelExecuteMsgFns},
    vfs,
};
use andromeda_vfs::VFSContract;
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, Wallet};

use crate::faucet::fund;

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
    pub adodb_contract: ADODBContract<DaemonBase<Wallet>>,
    pub vfs_contract: VFSContract<DaemonBase<Wallet>>,
    pub economics_contract: EconomicsContract<DaemonBase<Wallet>>,
}

impl MockAndromeda {
    pub fn new(daemon: &DaemonBase<Wallet>) -> MockAndromeda {
        let chain_name: String = daemon.chain_info().network_info.chain_name.to_string();

        // Upload and instantiate os ADOs
        let kernel_contract = KernelContract::new(daemon.clone());
        kernel_contract.upload().unwrap();
        kernel_contract
            .clone()
            .instantiate(
                &kernel::InstantiateMsg {
                    chain_name,
                    owner: None,
                },
                None,
                None,
            )
            .unwrap();

        let adodb_contract = ADODBContract::new(daemon.clone());
        adodb_contract.upload().unwrap();
        adodb_contract
            .clone()
            .instantiate(
                &adodb::InstantiateMsg {
                    kernel_address: kernel_contract.addr_str().unwrap(),
                    owner: None,
                },
                None,
                None,
            )
            .unwrap();

        let vfs_contract = VFSContract::new(daemon.clone());
        vfs_contract.upload().unwrap();
        vfs_contract
            .clone()
            .instantiate(
                &vfs::InstantiateMsg {
                    kernel_address: kernel_contract.addr_str().unwrap(),
                    owner: None,
                },
                None,
                None,
            )
            .unwrap();

        let economics_contract = EconomicsContract::new(daemon.clone());
        economics_contract.upload().unwrap();
        economics_contract
            .clone()
            .instantiate(
                &economics::InstantiateMsg {
                    kernel_address: kernel_contract.addr_str().unwrap(),
                    owner: None,
                },
                None,
                None,
            )
            .unwrap();

        adodb_contract
            .clone()
            .publish(
                "adodb".to_string(),
                adodb_contract.code_id().unwrap(),
                "0.1.0".to_string(),
                None,
                None,
            )
            .unwrap();

        adodb_contract
            .clone()
            .publish(
                "vfs".to_string(),
                vfs_contract.code_id().unwrap(),
                "0.1.0".to_string(),
                None,
                None,
            )
            .unwrap();

        adodb_contract
            .clone()
            .publish(
                "kernel".to_string(),
                kernel_contract.code_id().unwrap(),
                "0.1.0".to_string(),
                None,
                None,
            )
            .unwrap();

        // update kernel
        kernel_contract
            .clone()
            .upsert_key_address("adodb".to_string(), adodb_contract.addr_str().unwrap())
            .unwrap();
        // .upsert_key_address("adodb".to_string(), adodb_contract.addr_str().unwrap());
        kernel_contract
            .clone()
            .upsert_key_address("vfs".to_string(), vfs_contract.addr_str().unwrap())
            .unwrap();
        kernel_contract
            .clone()
            .upsert_key_address(
                "economics".to_string(),
                economics_contract.addr_str().unwrap(),
            )
            .unwrap();

        MockAndromeda {
            kernel_contract,
            adodb_contract,
            vfs_contract,
            economics_contract,
        }
    }
}
