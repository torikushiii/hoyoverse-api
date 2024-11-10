use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;
use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Serialize, Deserialize)]
struct HoyolabResponse {
    data: HoyolabData,
}

#[derive(Debug, Serialize, Deserialize)]
struct HoyolabData {
    modules: Vec<Module>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Module {
    exchange_group: Option<ExchangeGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExchangeGroup {
    bonuses: Vec<Bonus>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Bonus {
    exchange_code: String,
    code_status: String,
    icon_bonuses: Vec<IconBonus>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IconBonus {
    bonus_num: String,
    icon_url: String,
}

static REWARD_HASHES: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("cd6682dd2d871dc93dfa28c3f281d527_6175554878133394960", "Dennies");
    m.insert("8609070fe148c0e0e367cda25fdae632_208324374592932270", "Polychrome");
    m.insert("6ef3e419022c871257a936b1857ac9d1_411767156105350865", "W-Engine Energy Module");
    m.insert("86e1f7a5ff283d527bbc019475847174_5751095862610622324", "Senior Investigator Logs");
    m
});

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://bbs-api-os.hoyolab.com/community/painter/wapi/circle/channel/guide/material";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .header("x-rpc-app_version", "2.42.0")
        .header("x-rpc-client_type", "4")
        .query(&[("game_id", "8")]) // Zenless game_id
        .send()
        .await
        .context("Failed to fetch HoyoLab API")?;

    if !response.status().is_success() {
        warn!(
            "[Zenless][Codes][HoyoLab] Failed to fetch data: status={}, body={:?}",
            response.status(),
            response.text().await?
        );
        return Ok(Vec::new());
    }

    let hoyolab_data: HoyolabResponse = response.json().await?;
    let current_time = Utc::now();
    let mut codes = Vec::new();

    if let Some(exchange_module) = hoyolab_data.data.modules.iter()
        .find(|m| m.exchange_group.is_some()) {

        if let Some(bonuses) = exchange_module.exchange_group.as_ref()
            .map(|g| &g.bonuses) {

            for bonus in bonuses.iter().filter(|b| b.code_status == "ON") {
                let rewards: Vec<String> = bonus.icon_bonuses.iter()
                    .map(|icon_bonus| {
                        let reward_name = REWARD_HASHES.iter()
                            .find(|(hash, _)| icon_bonus.icon_url.contains(*hash))
                            .map(|(_, name)| *name)
                            .unwrap_or("Unknown");
                        format!("x{} {}", icon_bonus.bonus_num, reward_name)
                    })
                    .collect();

                codes.push(GameCode {
                    id: None,
                    code: bonus.exchange_code.clone(),
                    active: true,
                    date: current_time.into(),
                    rewards,
                    source: "hoyolab".to_string(),
                });
            }
        }
    }

    if codes.is_empty() {
        debug!("[Zenless][Codes][HoyoLab] No codes found");
    } else {
        debug!("[Zenless][Codes][HoyoLab] Found {} codes", codes.len());
    }

    Ok(codes)
}