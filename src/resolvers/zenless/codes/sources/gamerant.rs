use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://gamerant.com/zenless-zone-zero-zzz-code-livestream-redeem-codes-free-polychrome/";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch GameRant page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let codes = parse_codes_from_html(&document)?;

    if codes.is_empty() {
        warn!("[Zenless][Codes][GameRant] No codes found");
    } else {
        debug!("[Zenless][Codes][GameRant] Found {} codes", codes.len());
    }

    Ok(codes)
}

fn parse_codes_from_html(document: &Html) -> anyhow::Result<Vec<GameCode>> {
    let tbody_selector = Selector::parse("tbody").unwrap();
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let li_selector = Selector::parse("li").unwrap();
    let current_time = Utc::now();

    let mut codes = Vec::new();

    if let Some(tbody) = document.select(&tbody_selector).next() {
        for row in tbody.select(&tr_selector) {
            let mut columns = row.select(&td_selector);

            // Get code from first column
            if let Some(code_col) = columns.next() {
                let code = code_col.text()
                    .collect::<String>()
                    .trim()
                    .replace("(PC Only)", "")
                    .trim()
                    .to_string();

                // Get rewards from second column
                if let Some(rewards_col) = columns.next() {
                    let rewards: Vec<String> = rewards_col.select(&li_selector)
                        .map(|li| li.text().collect::<String>().trim().to_string())
                        .filter(|reward| !reward.is_empty())
                        .collect();

                    if !code.is_empty() && !rewards.is_empty() {
                        codes.push(GameCode {
                            id: None,
                            code,
                            active: true,
                            date: current_time.into(),
                            rewards,
                            source: "gamerant".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(codes)
}