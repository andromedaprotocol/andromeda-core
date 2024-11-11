use reqwest::blocking::Client;
use std::env;

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
