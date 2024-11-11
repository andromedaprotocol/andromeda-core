use cw_orch::core::serde_json;
use reqwest::blocking::Client;
use std::env;

use andromeda_deploy::adodb;
use andromeda_deploy::os;
use chrono::Local;
use dotenv::dotenv;

fn send_slack_notification(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let webhook_url = env::var("SLACK_WEBHOOK_URL").ok();
    if webhook_url.is_none() {
        return Ok(());
    }

    let payload = serde_json::json!({
        "text": message,
        "blocks": [
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": message
                }
            }
        ]
    });

    Client::new()
        .post(webhook_url.unwrap())
        .json(&payload)
        .send()?;

    Ok(())
}

fn main() {
    dotenv().ok();

    let chain = env::var("DEPLOYMENT_CHAIN").expect("DEPLOYMENT_CHAIN must be set");
    let kernel_address = env::var("DEPLOYMENT_KERNEL_ADDRESS").ok();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Send start notification
    let start_message = format!(
        "🚀 *Deployment Started*\n```\n| Chain          | {} |\n| Time           | {} |\n| Kernel Address | {} |```",
        chain,
        timestamp,
        kernel_address.as_deref().unwrap_or("Not provided")
    );

    if let Err(e) = send_slack_notification(&start_message) {
        eprintln!("Failed to send Slack notification: {}", e);
    }

    let deploy_os_res = os::deploy(chain.clone(), kernel_address.clone());

    if let Err(e) = deploy_os_res {
        println!("Error deploying OS: {}", e);
        let error_message = format!(
            "❌ *Deployment Failed*\n```\n| Chain          | {} |\n| Time           | {} |\n| Kernel Address | {} |\n| Error          | {} |```",
            chain,
            timestamp,
            kernel_address.as_deref().unwrap_or("Not provided"),
            e
        );

        if let Err(notification_error) = send_slack_notification(&error_message) {
            eprintln!("Failed to send Slack notification: {}", notification_error);
        }
        std::process::exit(1);
    }

    let kernel_address = deploy_os_res.unwrap();

    // Send completion notification
    let completion_timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let completion_message = format!(
        "✅ *Deployment Completed*\n```\n| Chain          | {} |\n| Time           | {} |\n| Kernel Address | {} |```",
        chain,
        completion_timestamp,
        kernel_address
    );

    if let Err(e) = send_slack_notification(&completion_message) {
        eprintln!("Failed to send Slack notification: {}", e);
    }

    adodb::deploy(chain, kernel_address, None).unwrap();
}
