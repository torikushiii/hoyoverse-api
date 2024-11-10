use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector, ElementRef};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;
use regex::Regex;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://genshin-impact.fandom.com/wiki/Promotional_Code";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Fandom page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let active_codes_selector = Selector::parse("#Active_Codes").unwrap();
    let table_selector = Selector::parse("#mw-content-text > div > table").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();
    let code_selector = Selector::parse("code").unwrap();

    let mut codes = Vec::new();
    let current_time = Utc::now();

    if let Some(active_section) = document.select(&active_codes_selector).next() {
        if let Some(table) = document.select(&table_selector)
            .find(|t| {
                t.html().as_bytes().as_ptr() as usize >
                active_section.html().as_bytes().as_ptr() as usize
            })
        {
            let mut rows = table.select(&row_selector);
            rows.next();

            for row in rows {
                let cells: Vec<ElementRef> = row.select(&cell_selector).collect();
                if cells.len() >= 3 {
                    let code = extract_code(&cells[0], &code_selector);
                    let server = cells[1].text().collect::<String>().trim().to_lowercase();

                    // Only process codes for "all" servers and non-empty codes
                    if server == "all" && !code.is_empty() {
                        // Parse rewards using regex to match item names followed by "×" and numbers
                        let rewards_text = cells[2].text().collect::<String>();
                        if let Some(rewards) = parse_rewards(&rewards_text) {
                            codes.push(GameCode {
                                id: None,
                                code,
                                active: true,
                                date: current_time.into(),
                                rewards,
                                source: "fandom".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    if codes.is_empty() {
        // sometimes it returns empty for some reason
        warn!("[Genshin][Codes][Fandom] No active codes found");
    } else {
        debug!("[Genshin][Codes][Fandom] Found {} active codes", codes.len());
    }

    Ok(codes)
}

fn extract_code(cell: &ElementRef, code_selector: &Selector) -> String {
    // First try to find the code element
    if let Some(code_element) = cell.select(code_selector).next() {
        code_element.text().collect::<String>().trim().to_string()
    } else {
        // Fallback: split by newline or quote and take the first part
        cell.text()
            .collect::<String>()
            .split(|c| c == '\n' || c == '"')
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    }
}

fn parse_rewards(rewards_text: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"([^×]+)×\s*(\d+(?:,\d+)?)").unwrap();

    let rewards: Vec<String> = re.captures_iter(rewards_text)
        .map(|cap| {
            let item = cap[1].trim();
            let amount = &cap[2];
            format!("{} ×{}", item, amount)
        })
        .collect();

    if rewards.is_empty() {
        None
    } else {
        Some(rewards)
    }
}