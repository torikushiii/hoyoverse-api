use crate::global::Global;
use anyhow::Context as _;
use serde::Deserialize;
use std::sync::Arc;

const HOYOLAB_URL: &str =
    "https://bbs-api-os.hoyolab.com/community/painter/wapi/circle/channel/guide/material?game_id=6";

fn item_name_from_hash(hash: &str) -> Option<&'static str> {
    match hash {
        "77cb5426637574ba524ac458fa963da0_6409817950389238658" => Some("Stellar Jade"),
        "7cb0e487e051f177d3f41de8d4bbc521_2556290033227986328" => Some("Refined Aether"),
        "508229a94e4fa459651f64c1cd02687a_6307505132287490837" => Some("Traveler's Guide"),
        "0b12bdf76fa4abc6b4d1fdfc0fb4d6f5_4521150989210768295" => Some("Credit"),
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
