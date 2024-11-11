use adodb::ExecuteMsgFns;
use andromeda_adodb::ADODBContract;
use andromeda_kernel::KernelContract;
use andromeda_std::os::*;
use cw_orch::prelude::*;
use cw_orch_daemon::DaemonBuilder;
use kernel::QueryMsgFns;

use crate::{chains::get_chain, contracts::all_contracts, error::DeployError};

pub fn deploy(
    chain: String,
    kernel_address: String,
    contracts: Option<Vec<String>>,
) -> Result<(), DeployError> {
    let chain = get_chain(chain);
    let daemon = DaemonBuilder::new(chain).build().unwrap();
    let kernel = KernelContract::new(daemon.clone());
    kernel.set_address(&Addr::unchecked(kernel_address));

    let adodb = ADODBContract::new(daemon.clone());
    let adodb_addr = kernel.key_address("adodb")?;
    adodb.set_address(&adodb_addr);

    let all_contracts = all_contracts();

    let contracts_to_deploy = contracts.unwrap_or_default();
    contracts_to_deploy.iter().for_each(|name| {
        let contract = all_contracts.iter().find(|(n, _, _)| n == name);
        if contract.is_none() {
            log::warn!("Contract {} not found", name);
        }
    });
    for (name, version, upload) in all_contracts {
        if !contracts_to_deploy.is_empty() && !contracts_to_deploy.contains(&name) {
            continue;
        }

        println!("Deploying {} {}", name, version);
        let code_id = upload(&daemon)?;
        adodb.publish(name, code_id, version, None, None)?;
    }

    Ok(())
}
