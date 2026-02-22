//! Standalone test for the Discord webhook notifier.
//!
//! Reads the webhook URL from config.toml and sends a test embed to verify
//! the webhook is reachable and formatted correctly.
//!
//! Run with: cargo run --bin test-notifier

use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct Config {
    notifications: NotificationsConfig,
}

#[derive(Deserialize)]
struct NotificationsConfig {
    discord_webhook: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_str = std::fs::read_to_string("config.toml")
        .map_err(|_| anyhow::anyhow!("config.toml not found — run from project root"))?;
    let config: Config = toml::from_str(&config_str)?;

    if config.notifications.discord_webhook.is_empty() {
        anyhow::bail!("discord_webhook is empty in config.toml [notifications]");
    }

    println!("Sending test notification to Discord...");

    let payload = json!({
        "embeds": [{
            "title": "New Honkai: Star Rail Codes",
            "color": 0x9C59D1,
            "fields": [
                {
                    "name": "`TESTCODE123`",
                    "value": "Stellar Jade ×60, Credit ×5000\nSource: fandom",
                    "inline": false
                },
                {
                    "name": "`ANOTHERCODE`",
                    "value": "Stellar Jade ×30\nSource: game8",
                    "inline": false
                }
            ],
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "footer": {
                "text": "this is a test notification"
            }
        }]
    });

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;

    let resp = client
        .post(&config.notifications.discord_webhook)
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if status.is_success() {
        println!("Success! status={status}");
    } else {
        println!("Failed! status={status}");
        println!("Response body: {body}");
    }

    Ok(())
}
