use crate::{contracts::os_contracts, error::DeployError};
use adodb::{ExecuteMsgFns as ADODBExecuteMsgFns, QueryMsgFns as ADODBQueryMsgFns};
use andromeda_std::ado_base::MigrateMsg;
use andromeda_std::amp::AndrAddr;
use andromeda_std::os::*;
use cw_orch::core::contract::Contract;
use cw_orch::prelude::*;
use cw_orch_daemon::{DaemonBase, DaemonBuilder, TxSender, Wallet};
use kernel::{ExecuteMsgFns, QueryMsgFns};

use crate::chains::get_chain;
use andromeda_adodb::ADODBContract;
use andromeda_economics::EconomicsContract;
use andromeda_ibc_registry::IBCRegistryContract;
use andromeda_kernel::KernelContract;
use andromeda_vfs::VFSContract;

struct OperatingSystemDeployment {
    daemon: DaemonBase<Wallet>,
    kernel: KernelContract<DaemonBase<Wallet>>,
    adodb: ADODBContract<DaemonBase<Wallet>>,
    vfs: VFSContract<DaemonBase<Wallet>>,
    economics: EconomicsContract<DaemonBase<Wallet>>,
    ibc_registry: IBCRegistryContract<DaemonBase<Wallet>>,
}

impl OperatingSystemDeployment {
    pub fn new(chain: ChainInfo) -> Self {
        let daemon = DaemonBuilder::new(chain).build().unwrap();
        let kernel = KernelContract::new(daemon.clone());
        let adodb = ADODBContract::new(daemon.clone());
        let vfs = VFSContract::new(daemon.clone());
        let economics = EconomicsContract::new(daemon.clone());
        let ibc_registry = IBCRegistryContract::new(daemon.clone());
        Self {
            daemon,
            kernel,
            adodb,
            vfs,
            economics,
            ibc_registry,
        }
    }

    pub fn upload(&self) -> Result<(), DeployError> {
        self.kernel.upload()?;
        self.adodb.upload()?;
        self.vfs.upload()?;
        self.economics.upload()?;
        self.ibc_registry.upload()?;
        Ok(())
    }

    /// Checks if a module exists already, if it does the module is migrated to the new code id.
    /// If it doesn't exist, the module is instantiated.
    fn instantiate_or_migrate(
        &self,
        module_name: &str,
        contract: &Contract<DaemonBase<Wallet>>,
    ) -> Result<(), DeployError> {
        let sender = self.daemon.sender().address();
        let addr = self.kernel.key_address(module_name).ok();
        if let Some(addr) = addr {
            let code_id = contract.code_id().unwrap();
            contract.set_address(&addr);
            contract.migrate(&MigrateMsg {}, code_id)?;
        } else if module_name == "ibc-registry" {
            let msg = ibc_registry::InstantiateMsg {
                owner: Some(sender.to_string()),
                kernel_address: self.kernel.address().unwrap(),
                service_address: AndrAddr::from_string(sender.to_string()),
            };
            contract.instantiate(&msg, Some(&sender), None)?;
        } else {
            let msg = adodb::InstantiateMsg {
                owner: Some(sender.to_string()),
                kernel_address: self.kernel.address().unwrap().to_string(),
            };
            contract.instantiate(&msg, Some(&sender), None)?;
        };

        Ok(())
    }

    /// Instantiates OS contracts.
    /// If a kernel is provided we look to migrate the existing contracts instead of creating new ones.
    pub fn instantiate(&self, kernel_address: Option<String>) -> Result<(), DeployError> {
        let sender = self.daemon.sender().address();

        let has_kernel_address = kernel_address.as_ref().is_some_and(|addr| !addr.is_empty());
        // If kernel address is provided, use it and migrate the contract to the new version
        if has_kernel_address {
            let code_id = self.kernel.code_id().unwrap();
            self.kernel
                .set_address(&Addr::unchecked(kernel_address.unwrap()));
            self.kernel.migrate(&MigrateMsg {}, code_id)?;
        } else {
            let kernel_msg = kernel::InstantiateMsg {
                owner: Some(sender.to_string()),
                chain_name: self.daemon.chain_info().network_info.chain_name.to_string(),
            };
            self.kernel.instantiate(&kernel_msg, Some(&sender), None)?;
            println!("Kernel address: {}", self.kernel.address().unwrap());
        };

        // For each module we check if it's been instantiated already.
        // If it has, we migrate it to the new code id.
        // If it hasn't, we instantiate it.
        let modules: [(&str, &Contract<DaemonBase<Wallet>>); 4] = [
            ("adodb", self.adodb.as_instance()),
            ("vfs", self.vfs.as_instance()),
            ("economics", self.economics.as_instance()),
            ("ibc-registry", self.ibc_registry.as_instance()),
        ];

        for (module_name, contract) in modules {
            self.instantiate_or_migrate(module_name, contract)?;
        }

        Ok(())
    }

    fn register_modules(&self) -> Result<(), DeployError> {
        let modules: [(&str, &Contract<DaemonBase<Wallet>>); 4] = [
            ("adodb", self.adodb.as_instance()),
            ("vfs", self.vfs.as_instance()),
            ("economics", self.economics.as_instance()),
            ("ibc-registry", self.ibc_registry.as_instance()),
        ];

        for (module_name, contract) in modules {
            self.kernel
                .upsert_key_address(module_name, contract.address().unwrap())?;
        }

        Ok(())
    }

    /// Publishes all OS ADOs with the created ADODB.
    /// Skips publishing if the ADO version is already published.
    fn publish(&self) -> Result<(), DeployError> {
        let uploaded_contracts: [(&str, &Contract<DaemonBase<Wallet>>); 5] = [
            ("kernel", self.kernel.as_instance()),
            ("adodb", self.adodb.as_instance()),
            ("vfs", self.vfs.as_instance()),
            ("economics", self.economics.as_instance()),
            ("ibc-registry", self.ibc_registry.as_instance()),
        ];
        let deployable = os_contracts();
        for (name, contract) in uploaded_contracts {
            let (_, version, _) = deployable.iter().find(|(n, _, _)| n == name).unwrap();
            let versions = self.adodb.ado_versions(name.to_string(), None, None)?;
            if versions.contains(&format!("{}@{}", name, version)) {
                log::info!(
                    "Skipping publishing {} {} - already published",
                    name,
                    version
                );
                continue;
            }
            let code_id = contract.code_id().unwrap();
            self.adodb.publish(name, code_id, version, None, None)?;
        }
        Ok(())
    }
}

pub fn deploy(chain: String, kernel_address: Option<String>) -> Result<String, DeployError> {
    let chain = get_chain(chain);
    let os_deployment = OperatingSystemDeployment::new(chain);
    log::info!("Starting OS deployment process");

    log::info!("Uploading contracts");
    os_deployment.upload()?;

    log::info!("Instantiating/migrating contracts");
    os_deployment.instantiate(kernel_address)?;

    log::info!("Registering modules");
    os_deployment.register_modules()?;

    log::info!("Publishing contracts");
    os_deployment.publish()?;

    log::info!("OS deployment process completed");
    Ok(os_deployment.kernel.address().unwrap().to_string())
}
