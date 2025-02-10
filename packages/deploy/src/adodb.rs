use crate::slack::SlackNotification;
use crate::{chains::get_chain, contracts::all_contracts, error::DeployError};
use adodb::ExecuteMsgFns;
use adodb::QueryMsgFns as ADODBQueryMsgFns;
use andromeda_adodb::ADODBContract;
use andromeda_kernel::KernelContract;
use andromeda_std::os::*;
use cw_orch::prelude::*;
use cw_orch_daemon::DaemonBuilder;
use kernel::QueryMsgFns;

pub fn deploy(
    chain: String,
    kernel_address: String,
    contracts: Option<Vec<String>>,
) -> Result<Vec<(String, String, u64)>, DeployError> {
    let chain = get_chain(chain);
    let daemon = DaemonBuilder::new(chain.clone()).build().unwrap();

    log::info!("Setting kernel address to {}", kernel_address);
    let kernel = KernelContract::new(daemon.clone());
    kernel.set_address(&Addr::unchecked(kernel_address.clone()));

    let adodb = ADODBContract::new(daemon.clone());
    let adodb_addr = kernel.key_address("adodb")?;
    log::info!("Setting ADODB address to {}", adodb_addr);
    adodb.set_address(&adodb_addr);

    log::info!("Getting all contracts");
    let all_contracts = all_contracts();

    let contracts_to_deploy = contracts.unwrap_or_default();
    log::info!("Checking for invalid contracts");
    let invalid_contracts = contracts_to_deploy
        .iter()
        .filter(|name| !all_contracts.iter().any(|(n, _, _)| &n == name))
        .cloned()
        .collect::<Vec<String>>();
    if !invalid_contracts.is_empty() {
        SlackNotification::ADOWarning(chain.chain_id.to_string(), invalid_contracts.clone())
            .send()
            .unwrap();
    }

    log::info!("Filtering valid contracts");
    let valid_contracts: Vec<String> = if contracts_to_deploy.is_empty() {
        all_contracts.iter().map(|(n, _, _)| n.clone()).collect()
    } else {
        contracts_to_deploy
            .iter()
            .filter(|name| all_contracts.iter().any(|(n, _, _)| &n == name))
            .cloned()
            .collect()
    };

    SlackNotification::ADODeploymentStarted(chain.chain_id.to_string(), valid_contracts.clone())
        .send()
        .unwrap();

    log::info!("Deploying contracts");
    let mut deployed_contracts: Vec<(String, String, u64)> = vec![];
    for (name, version, upload) in all_contracts {
        if !contracts_to_deploy.is_empty() && !contracts_to_deploy.contains(&name) {
            log::info!(
                "Skipping {} {} - not included in deploy list",
                name,
                version
            );
            continue;
        }
        let versions = adodb.ado_versions(&name, None, None)?;
        log::info!("{} Versions: {:?}", name, versions);
        if versions.contains(&format!("{}@{}", name, version)) {
            log::info!("Skipping {} {} - already deployed", name, version);
            continue;
        }

        if version.contains("-a.") {
            log::info!("Skipping {} {} - alpha version", name, version);
            continue;
        }

        log::info!("Deploying {} {}", name, version);
        let code_id = upload(&daemon)?;
        let res = adodb.publish(name.clone(), code_id, version.clone(), None, None);
        if let Err(e) = res {
            log::error!("Error deploying {}: {}", name, e);
            continue;
        }
        deployed_contracts.push((name, version, code_id));
    }

    SlackNotification::ADODeploymentCompleted(
        chain.chain_id.to_string(),
        deployed_contracts.clone(),
    )
    .send()
    .unwrap();

    Ok(deployed_contracts)
}
