use crate::global::Global;
use anyhow::Context as _;
use serde::Deserialize;
use std::sync::Arc;

const HOYOLAB_URL: &str =
    "https://bbs-api-os.hoyolab.com/community/painter/wapi/circle/channel/guide/material?game_id=8";

fn item_name_from_hash(hash: &str) -> Option<&'static str> {
    match hash {
        "cd6682dd2d871dc93dfa28c3f281d527_6175554878133394960" => Some("Dennies"),
        "8609070fe148c0e0e367cda25fdae632_208324374592932270" => Some("Polychrome"),
        "6ef3e419022c871257a936b1857ac9d1_411767156105350865" => Some("W-Engine Energy Module"),
        "86e1f7a5ff283d527bbc019475847174_5751095862610622324" => Some("Senior Investigator Logs"),
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
