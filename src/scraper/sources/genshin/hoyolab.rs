use crate::global::Global;
use anyhow::Context as _;
use serde::Deserialize;
use std::sync::Arc;

const HOYOLAB_URL: &str =
    "https://bbs-api-os.hoyolab.com/community/painter/wapi/circle/channel/guide/material?game_id=2";

fn item_name_from_hash(hash: &str) -> Option<&'static str> {
    match hash {
        "150a941de99e21fc96dce97cde2dae22_1631694835879620915" => Some("Primogem"),
        "46de1e881b5dff638969aed85850e388_7373589751062039567" => Some("Hero's Wit"),
        "503abf5f2f2c8b2013dde0f2197fc9ac_3214074117670348863" => Some("Mora"),
        "d3eb1267f27bead29907cb279d4365ab_4473305467748929436" => Some("Mystic Enhancement Ore"),
        _ => None,
    }
}

fn icon_url_to_hash(url: &str) -> &str {
    let filename = url.rsplit('/').next().unwrap_or(url);
    filename.split('.').next().unwrap_or(filename)
}

#[derive(Debug)]
pub struct ParsedCode {
    pub code: String,
    pub rewards: Vec<String>,
}

#[derive(Deserialize)]
struct Response {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    modules: Vec<Module>,
}

#[derive(Deserialize)]
struct Module {
    exchange_group: Option<ExchangeGroup>,
}

#[derive(Deserialize)]
struct ExchangeGroup {
    bonuses: Vec<Bonus>,
}

#[derive(Deserialize)]
struct Bonus {
    exchange_code: String,
    code_status: String,
    icon_bonuses: Vec<IconBonus>,
}

#[derive(Deserialize)]
struct IconBonus {
    bonus_num: u64,
    icon_url: String,
}

#[tracing::instrument(skip(global))]
pub async fn scrape(global: &Arc<Global>) -> anyhow::Result<Vec<ParsedCode>> {
    let resp = global
        .http_client
        .get(HOYOLAB_URL)
        .header("x-rpc-app_version", "4.8.0")
        .header("x-rpc-client_type", "4")
        .header("x-rpc-language", "en-us")
        .header("Referer", "https://www.hoyolab.com/")
        .send()
        .await?
        .json::<Response>()
        .await
        .context("failed to parse hoyolab response")?;

    let codes: Vec<ParsedCode> = resp
        .data
        .modules
        .into_iter()
        .filter_map(|m| m.exchange_group)
        .flat_map(|g| g.bonuses)
        .filter(|b| b.code_status == "ON" && !b.exchange_code.is_empty())
        .map(|b| {
            let rewards = b
                .icon_bonuses
                .iter()
                .filter_map(|ib| {
                    let hash = icon_url_to_hash(&ib.icon_url);
                    let name = item_name_from_hash(hash)?;
                    Some(format!("{} ×{}", name, ib.bonus_num))
                })
                .collect();
            ParsedCode {
                code: b.exchange_code,
                rewards,
            }
        })
        .collect();

    tracing::info!(count = codes.len(), "scraped codes from hoyolab");

    Ok(codes)
}
