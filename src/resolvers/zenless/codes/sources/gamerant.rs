use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url =
        "https://gamerant.com/zenless-zone-zero-zzz-code-livestream-redeem-codes-free-polychrome/";

    let response = client
        .get(url)
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
    let table_selector = Selector::parse("table").unwrap();
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let li_selector = Selector::parse("li").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let current_time = Utc::now();

    let mut codes = Vec::new();

    // Find the table and iterate through all rows with td elements
    if let Some(table) = document.select(&table_selector).next() {
        for row in table.select(&tr_selector) {
            let columns: Vec<_> = row.select(&td_selector).collect();

            // Skip rows that don't have exactly 2 columns (code and rewards)
            if columns.len() != 2 {
                continue;
            }

            // Get code from first column - look for anchor tag
            if let Some(code_col) = columns.get(0) {
                let code = if let Some(anchor) = code_col.select(&a_selector).next() {
                    anchor.text().collect::<String>().trim().to_uppercase()
                } else {
                    code_col
                        .text()
                        .collect::<String>()
                        .trim()
                        .replace("(PC Only)", "")
                        .trim()
                        .to_uppercase()
                };

                // Get rewards from second column
                if let Some(rewards_col) = columns.get(1) {
                    let rewards: Vec<String> = rewards_col
                        .select(&li_selector)
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
