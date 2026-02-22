use std::sync::Arc;

use serde_json::json;

use crate::games::Game;
use crate::global::Global;

pub async fn notify_new_codes(global: &Arc<Global>, game: Game, codes: &[(String, Vec<String>, String)]) {
    let Some(webhook_url) = &global.discord_webhook else {
        return;
    };

    let fields: Vec<serde_json::Value> = codes
        .iter()
        .map(|(code, rewards, source)| {
            let value = if rewards.is_empty() {
                format!("Source: {source}")
            } else {
                format!("{}\nSource: {source}", rewards.join(", "))
            };
            json!({
                "name": format!("`{code}`"),
                "value": value,
                "inline": false,
            })
        })
        .collect();

    let payload = json!({
        "embeds": [{
            "title": format!("New {} Codes", game.display_name()),
            "color": game.embed_color(),
            "fields": fields,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }]
    });

    match global.http_client.post(webhook_url).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(game = game.slug(), count = codes.len(), "discord notification sent");
        }
        Ok(resp) => {
            tracing::warn!(game = game.slug(), status = %resp.status(), "discord notification failed");
        }
        Err(e) => {
            tracing::warn!(game = game.slug(), error = %e, "discord notification request failed");
        }
    }
}
