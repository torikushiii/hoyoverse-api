use reqwest::Client;
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info};

use crate::{config::Settings, types::GameCode};

const COLORS: [(u32, &str); 4] = [
    (0x9370DB, "starrail"), // Purple-pinkish (Medium Purple)
    (0x00FFFF, "genshin"),  // Cyan
    (0xFFD700, "themis"),   // Yellow (Gold)
    (0xD2691E, "zenless"),  // Orange-brownish (Chocolate)
];

#[derive(Debug, Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    fields: Vec<EmbedField>,
}

#[derive(Debug, Serialize)]
struct EmbedField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Debug, Serialize)]
struct WebhookPayload {
    embeds: Vec<DiscordEmbed>,
}

pub struct WebhookService {
    client: Client,
    config: Arc<Settings>,
}

impl WebhookService {
    pub fn new(config: Arc<Settings>) -> Self {
        Self {
            client: Client::builder()
                .user_agent(&config.server.user_agent)
                .build()
                .expect("Failed to create HTTP client"),
            config,
        }
    }

    pub async fn send_new_code_notification(
        &self,
        code: &GameCode,
        game_type: &str,
    ) -> anyhow::Result<()> {
        if let Some(webhook_url) = self.config.discord.webhook_url.as_ref() {
            let color = COLORS
                .iter()
                .find(|(_, game)| *game == game_type)
                .map(|(color, _)| *color)
                .unwrap_or(0x808080); // Default gray color

            let game_name = match game_type {
                "starrail" => "Honkai: Star Rail",
                "genshin" => "Genshin Impact",
                "themis" => "Tears of Themis",
                "zenless" => "Zenless Zone Zero",
                _ => game_type,
            };

            let rewards_text = if code.rewards.is_empty() {
                "Unknown".to_string()
            } else {
                code.rewards.join("\n")
            };

            let embed = DiscordEmbed {
                title: format!("New {} Code!", game_name),
                description: format!(
                    "A new redemption code has been discovered for {}!",
                    game_name
                ),
                color,
                fields: vec![
                    EmbedField {
                        name: "Code".to_string(),
                        value: format!("`{}`", code.code),
                        inline: true,
                    },
                    EmbedField {
                        name: "Rewards".to_string(),
                        value: rewards_text,
                        inline: true,
                    },
                ],
            };

            let payload = WebhookPayload {
                embeds: vec![embed],
            };

            match self.client.post(webhook_url).json(&payload).send().await {
                Ok(response) if response.status().is_success() => {
                    info!(
                        "[{}] Successfully sent webhook notification for code {}",
                        game_type, code.code
                    );
                    Ok(())
                }
                Ok(response) => {
                    error!(
                        "[{}] Failed to send webhook notification. Status: {}",
                        game_type,
                        response.status()
                    );
                    Err(anyhow::anyhow!("Failed to send webhook notification"))
                }
                Err(e) => {
                    error!("[{}] Error sending webhook notification: {}", game_type, e);
                    Err(anyhow::anyhow!("Error sending webhook notification"))
                }
            }
        } else {
            Ok(()) // Silently succeed if no webhook URL is configured
        }
    }

    pub async fn send_invalid_credentials_notification(
        &self,
        game_type: &str,
    ) -> anyhow::Result<()> {
        if let Some(webhook_url) = self.config.discord.webhook_url.as_ref() {
            let color = 0xFF0000;

            let game_name = match game_type {
                "starrail" => "Honkai: Star Rail",
                "genshin" => "Genshin Impact",
                "themis" => "Tears of Themis",
                "zenless" => "Zenless Zone Zero",
                _ => game_type,
            };

            let embed = DiscordEmbed {
                title: format!("⚠️ Invalid Credentials - {}", game_name),
                description: format!(
                    "The account credentials for {} are invalid or expired. Code verification for this game will be skipped until credentials are updated.",
                    game_name
                ),
                color,
                fields: vec![],
            };

            let payload = WebhookPayload {
                embeds: vec![embed],
            };

            match self.client.post(webhook_url).json(&payload).send().await {
                Ok(response) if response.status().is_success() => Ok(()),
                Ok(response) => {
                    error!(
                        "[{}] Failed to send invalid credentials notification. Status: {}",
                        game_type,
                        response.status()
                    );
                    Err(anyhow::anyhow!(
                        "Failed to send invalid credentials notification"
                    ))
                }
                Err(e) => {
                    error!(
                        "[{}] Error sending invalid credentials notification: {}",
                        game_type, e
                    );
                    Err(anyhow::anyhow!(
                        "Error sending invalid credentials notification"
                    ))
                }
            }
        } else {
            Ok(())
        }
    }
}
