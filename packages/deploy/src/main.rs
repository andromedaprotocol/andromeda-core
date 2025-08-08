use andromeda_deploy::build;
use andromeda_deploy::report::DeploymentReport;
use andromeda_deploy::slack::SlackNotification;
use andromeda_deploy::validate;
use andromeda_deploy::vercel;
use std::env;
use std::fs;

use andromeda_deploy::adodb;
use andromeda_deploy::os;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();
    let kernel_address = env::var("DEPLOYMENT_KERNEL_ADDRESS").ok().unwrap();

    let chain = dotenv::var("DEPLOYMENT_CHAIN").expect("DEPLOYMENT_CHAIN must be set");
    let mut should_upload_after_deploy = false;

    let blobs = vercel::list_commit_blobs().await;

    match blobs {
        Ok(blobs) if !blobs.is_empty() => {
            log::info!("Found prebuilt artifacts on Vercel for this commit. Restoring...");
            vercel::download_blobs_to_artifacts(&blobs).await.unwrap();

            let contracts_to_deploy = fs::read_dir("artifacts")
                .unwrap()
                .map(|file| file.unwrap().file_name().to_str().unwrap().to_string())
                .collect::<Vec<String>>();

            let adodb_res = adodb::deploy(
                chain.clone(),
                kernel_address.clone(),
                Some(contracts_to_deploy),
            );
            if let Err(e) = adodb_res {
                println!("Error deploying ADODB: {}", e);
                SlackNotification::ADODeploymentFailed(chain.clone(), e)
                    .send()
                    .unwrap();
                std::process::exit(1);
            }

            let deployed_contracts = adodb_res.unwrap();
            DeploymentReport {
                chain_id: chain.clone(),
                contracts: deployed_contracts,
                kernel_address: kernel_address.clone(),
            }
            .write_to_json()
            .unwrap();
        }
        Ok(_) => {
            validate::run();
            build::run();
            should_upload_after_deploy = true;
        }
        Err(e) => {
            println!("Failed to list blobs: {}", e);
            std::process::exit(1);
        }
    }

    let mut kernel_address = env::var("DEPLOYMENT_KERNEL_ADDRESS").ok();

    // Send start notification
    SlackNotification::DeploymentStarted(chain.clone(), kernel_address.clone())
        .send()
        .unwrap();

    let should_deploy_os = env::var("DEPLOY_OS").unwrap_or_default().to_lowercase() == "true";
    if should_deploy_os {
        let deploy_os_res = os::deploy(chain.clone(), kernel_address.clone());

        if let Err(e) = deploy_os_res {
            println!("Error deploying OS: {}", e);
            SlackNotification::DeploymentFailed(chain.clone(), kernel_address.clone(), e)
                .send()
                .unwrap();
            std::process::exit(1);
        }

        kernel_address = Some(deploy_os_res.unwrap());

        // Send completion notification
        SlackNotification::DeploymentCompleted(chain.clone(), kernel_address.clone())
            .send()
            .unwrap();
    }

    let kernel_address = kernel_address.expect("Kernel address must be set or OS deployed");
    let contracts_to_deploy = env::var("DEPLOY_CONTRACTS")
        .ok()
        .unwrap_or_default()
        .split(',')
        .map(|s| {
            s.to_string()
                .trim()
                .to_lowercase()
                .replace("andromeda_", "")
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();

    let adodb_res = adodb::deploy(
        chain.clone(),
        kernel_address.clone(),
        Some(contracts_to_deploy),
    );
    if let Err(e) = adodb_res {
        println!("Error deploying ADODB: {}", e);
        SlackNotification::ADODeploymentFailed(chain.clone(), e)
            .send()
            .unwrap();
        std::process::exit(1);
    }

    let deployed_contracts = adodb_res.unwrap();
    DeploymentReport {
        chain_id: chain.clone(),
        contracts: deployed_contracts,
        kernel_address: kernel_address.clone(),
    }
    .write_to_json()
    .unwrap();

    // Upload artifacts only if we built them in this run (cache miss path)
    if should_upload_after_deploy {
        if let Err(e) = vercel::upload_wasm_folder("artifacts").await {
            println!("Error uploading artifacts to Vercel Blob: {}", e);
            // Non-fatal
        }
    }
}
