use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;
use regex::Regex;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://game8.co/games/Zenless-Zone-Zero/archives/435683";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Game8 page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let codes = parse_codes_from_html(&document)?;

    if codes.is_empty() {
        warn!("[Zenless][Codes][Game8] No codes found");
    } else {
        debug!("[Zenless][Codes][Game8] Found {} codes", codes.len());
    }

    Ok(codes)
}

fn parse_codes_from_html(document: &Html) -> anyhow::Result<Vec<GameCode>> {
    let table_selector = Selector::parse("table.a-table").unwrap();
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let input_selector = Selector::parse("input.a-clipboard__textInput").unwrap();
    let align_div_selector = Selector::parse("div.align").unwrap();
    let code_regex = Regex::new(r"^[A-Z0-9]+$").unwrap();
    let current_time = Utc::now();

    let mut codes = Vec::new();

    // Find the table and iterate through rows
    if let Some(table) = document.select(&table_selector).next() {
        for row in table.select(&tr_selector) {
            let columns: Vec<_> = row.select(&td_selector).collect();

            // Skip header row and rows that don't have exactly 2 columns
            if columns.len() != 2 {
                continue;
            }

            // Get code from first column - look for input element
            if let Some(code_col) = columns.get(0) {
                let code = if let Some(input) = code_col.select(&input_selector).next() {
                    input.value().attr("value")
                        .unwrap_or("")
                        .trim()
                        .to_uppercase()
                } else {
                    continue;
                };

                if !code_regex.is_match(&code) {
                    continue;
                }

                // Get rewards from second column - look for div.align elements
                if let Some(rewards_col) = columns.get(1) {
                    let rewards: Vec<String> = rewards_col.select(&align_div_selector)
                         .filter_map(|div| {
                             let text = div.text().collect::<String>();
                             let cleaned = text.trim()
                                 .replace('\n', " ")
                                 .split_whitespace()
                                 .collect::<Vec<&str>>()
                                 .join(" ");
                             if cleaned.is_empty() {
                                 None
                             } else {
                                 Some(cleaned)
                             }
                         })
                         .collect();

                    if !rewards.is_empty() {
                        codes.push(GameCode {
                            id: None,
                            code,
                            active: true,
                            date: current_time.into(),
                            rewards,
                            source: "game8".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(codes)
}