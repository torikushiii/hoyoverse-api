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
    #[serde(deserialize_with = "deserialize_bonus_num")]
    bonus_num: String,
    icon_url: String,
}

fn deserialize_bonus_num<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct BonusNumVisitor;

    impl<'de> Visitor<'de> for BonusNumVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(BonusNumVisitor)
}

static REWARD_HASHES: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("150a941de99e21fc96dce97cde2dae22_1631694835879620915", "Primogem");
    m.insert("46de1e881b5dff638969aed85850e388_7373589751062039567", "Hero's Wit");
    m.insert("503abf5f2f2c8b2013dde0f2197fc9ac_3214074117670348863", "Mora");
    m.insert("d3eb1267f27bead29907cb279d4365ab_4473305467748929436", "Mystic Enhancement Ore");
    m
});

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://bbs-api-os.hoyolab.com/community/painter/wapi/circle/channel/guide/material";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .header("x-rpc-app_version", "2.42.0")
        .header("x-rpc-client_type", "4")
        .query(&[("game_id", "2")])
        .send()
        .await
        .context("Failed to fetch HoyoLab API")?;

    if !response.status().is_success() {
        warn!(
            "[Genshin][Codes][HoyoLab] Failed to fetch data: status={}, body={:?}",
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
                        format!("{} ×{}", reward_name, icon_bonus.bonus_num)
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
        debug!("[Genshin][Codes][HoyoLab] No codes found");
    } else {
        debug!("[Genshin][Codes][HoyoLab] Found {} codes", codes.len());
    }

    Ok(codes)
}