use crate::slack::send_notification;
use crate::{chains::get_chain, contracts::all_contracts, error::DeployError};
use adodb::ExecuteMsgFns;
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
) -> Result<(), DeployError> {
    let chain = get_chain(chain);
    let daemon = DaemonBuilder::new(chain.clone()).build().unwrap();
    let kernel = KernelContract::new(daemon.clone());
    kernel.set_address(&Addr::unchecked(kernel_address.clone()));

    let adodb = ADODBContract::new(daemon.clone());
    let adodb_addr = kernel.key_address("adodb")?;
    adodb.set_address(&adodb_addr);

    let all_contracts = all_contracts();

    let contracts_to_deploy = contracts.unwrap_or_default();
    let invalid_contracts = contracts_to_deploy
        .iter()
        .filter(|name| !all_contracts.iter().any(|(n, _, _)| &n == name))
        .cloned()
        .collect::<Vec<String>>();
    if !invalid_contracts.is_empty() {
        let error_message = format!(
            "⚠️ *Deployment Warning*\n```\n| Invalid Contracts | {} |\n```",
            invalid_contracts.join(", ")
        );
        send_notification(&error_message).unwrap();
    }

    let valid_contracts = contracts_to_deploy
        .iter()
        .filter(|name| all_contracts.iter().any(|(n, _, _)| &n == name))
        .cloned()
        .collect::<Vec<String>>();

    let deployment_msg = format!(
        "🚀 *ADO Library Deployment Started*\n```\n| Chain          | {} |\n| Kernel Address | {} |\n| Contracts      | {} |```",
        chain.chain_id, kernel_address, valid_contracts.join(", ")
    );
    send_notification(&deployment_msg).unwrap();

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
