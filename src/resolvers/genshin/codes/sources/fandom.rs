use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, error, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://genshin-impact.fandom.com/wiki/Promotional_Code";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Fandom page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let table_selector = Selector::parse("table.wikitable.sortable.tdl3.tdl4").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();
    let code_selector = Selector::parse("code").unwrap();

    let mut codes = Vec::new();
    let current_time = Utc::now();

    let tables: Vec<ElementRef> = document.select(&table_selector).collect();

    if let Some(table) = tables.first() {
        let mut rows = table.select(&row_selector);
        rows.next();

        for row in rows {
            let cells: Vec<ElementRef> = row.select(&cell_selector).collect();
            if cells.len() >= 3 {
                let server_text = cells[1].text().collect::<String>();
                if is_supported_server(&server_text) {
                    let code_elements: Vec<String> = cells[0]
                        .select(&code_selector)
                        .map(|el| el.text().collect::<String>().trim().to_uppercase())
                        .filter(|code| !code.is_empty())
                        .collect();

                    for code in code_elements {
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
        warn!("[Genshin][Codes][Fandom] No active codes found");
        error!("[Genshin][Codes][Fandom] Tables found: {}", tables.len());
        if let Some(table) = tables.first() {
            error!(
                "[Genshin][Codes][Fandom] First table HTML: {}",
                table.html()
            );
        }
    } else {
        debug!(
            "[Genshin][Codes][Fandom] Found {} active codes",
            codes.len()
        );
    }

    Ok(codes)
}

fn parse_rewards(rewards_text: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"([^×]+)×\s*(\d+(?:,\d+)?)").unwrap();

    let rewards: Vec<String> = re
        .captures_iter(rewards_text)
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

fn is_supported_server(server_text: &str) -> bool {
    let server = server_text.to_lowercase();

    if server.contains("china") {
        return false;
    }

    let supported_keywords = [
        "all", "global", "america", "europe", "asia", "tw", "hk", "macao",
    ];

    supported_keywords
        .iter()
        .any(|keyword| server.contains(keyword))
}
