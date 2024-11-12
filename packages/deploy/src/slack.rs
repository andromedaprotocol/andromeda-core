use chrono::Local;
use reqwest::blocking::Client;
use std::env;

use crate::error::DeployError;

pub fn send_notification(message: &str) -> Result<(), Box<dyn std::error::Error>> {
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

pub enum SlackNotification {
    DeploymentStarted(String, Option<String>),
    DeploymentCompleted(String, Option<String>),
    DeploymentFailed(String, Option<String>, DeployError),
    ADODeploymentStarted(String, Vec<String>),
    ADODeploymentCompleted(String, Vec<String>),
    ADODeploymentFailed(String, DeployError),
    ADOWarning(String, Vec<String>),
}

impl SlackNotification {
    pub fn send(&self) -> Result<(), Box<dyn std::error::Error>> {
        send_notification(&self.to_string())
    }
}

impl std::fmt::Display for SlackNotification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlackNotification::DeploymentStarted(chain, kernel_address) => {
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                write!(f, "🚀 *Deployment Started*\n```\n| Chain          | {} |\n| Time           | {} |\n| Kernel Address | {} |```", chain, timestamp, kernel_address.as_deref().unwrap_or("Not provided"))
            }
            SlackNotification::DeploymentCompleted(chain, kernel_address) => {
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                write!(f, "✅ *Deployment Completed*\n```\n| Chain          | {} |\n| Time           | {} |\n| Kernel Address | {} |```", chain, timestamp, kernel_address.as_deref().unwrap_or("Not provided"))
            }
            SlackNotification::DeploymentFailed(chain, kernel_address, error) => {
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                write!(f, "❌ *Deployment Failed*\n```\n| Chain          | {} |\n| Time           | {} |\n| Kernel Address | {} |\n| Error          | {} |```", chain, timestamp, kernel_address.as_deref().unwrap_or("Not provided"), error)
            }
            SlackNotification::ADODeploymentStarted(chain, contracts) => {
                write!(f, "🚀 *ADO Library Deployment Started*\n```\n| Chain          | {} |\n| Contracts      | {} |```", chain, contracts.join(", "))
            }
            SlackNotification::ADODeploymentCompleted(chain, contracts) => {
                write!(f, "✅ *ADO Library Deployment Completed*\n```\n| Chain          | {} |\n| Contracts      | {} |```", chain, contracts.join(", "))
            }
            SlackNotification::ADODeploymentFailed(chain, contracts, error) => {
                write!(f, "❌ *ADO Library Deployment Failed*\n```\n| Chain          | {} |\n| Contracts      | {} |\n| Error          | {} |```", chain, contracts.join(", "), error)
            }
            SlackNotification::ADOWarning(chain, contracts) => {
                write!(f, "⚠️ *Invalid Contracts*\n```\n| Chain          | {} |\n| Contracts      | {} |```", chain, contracts.join(", "))
            }
        }
    }
}
