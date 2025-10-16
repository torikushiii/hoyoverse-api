use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://game8.co/games/Genshin-Impact/archives/304759";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Game8 page")?;

    if !response.status().is_success() {
        warn!(
            "[Genshin][Codes][Game8] Failed to fetch data, status: {}",
            response.status()
        );
        return Ok(Vec::new());
    }

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let mut codes = Vec::new();
    let current_time = Utc::now();

    // New layout: table with rows; first row is headers, subsequent rows contain code and rewards
    let table_selector = Selector::parse("table.a-table").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let th_selector = Selector::parse("th").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let input_selector = Selector::parse("input.a-clipboard__textInput").unwrap();
    let link_selector = Selector::parse("a.a-link").unwrap();
    let align_selector = Selector::parse("div.align").unwrap();

    for table in document.select(&table_selector) {
        for row in table.select(&row_selector) {
            // Skip header row
            if row.select(&th_selector).next().is_some() {
                continue;
            }

            let mut tds = row.select(&td_selector);
            let code_td = if let Some(td) = tds.next() {
                td
            } else {
                continue;
            };
            let rewards_td = if let Some(td) = tds.next() {
                td
            } else {
                continue;
            };

            // Extract code from input value first, fallback to gift link
            let mut code: String = String::new();
            if let Some(input) = code_td.select(&input_selector).next() {
                if let Some(val) = input.value().attr("value") {
                    code = val.trim().to_string();
                }
            }
            if code.is_empty() {
                if let Some(link) = code_td.select(&link_selector).find(|a| {
                    a.value()
                        .attr("href")
                        .map_or(false, |href| href.contains("gift?code="))
                }) {
                    if let Some(href) = link.value().attr("href") {
                        if let Some(pos) = href.find("code=") {
                            let start = pos + 5;
                            let end = href[start..]
                                .find('&')
                                .map(|i| start + i)
                                .unwrap_or(href.len());
                            code = href[start..end].to_string();
                        }
                    }
                }
            }

            // Normalize to uppercase and ensure only A-Z0-9 characters (matches test expectations)
            code = code.trim().to_uppercase();
            if !code
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            {
                continue;
            }

            // Build rewards from each div.align (name from link text, qty from trailing `xNNN`)
            let mut rewards: Vec<String> = Vec::new();
            for align in rewards_td.select(&align_selector) {
                let name = align
                    .select(&link_selector)
                    .next()
                    .map(|a| a.text().collect::<String>().trim().to_string())
                    .unwrap_or_else(|| align.text().collect::<String>().trim().to_string());

                let align_text = align.text().collect::<String>();
                // Extract quantity like x60 or x20,000
                let qty = extract_quantity(&align_text);

                let reward = match qty {
                    Some(q) => format!("{} {}", name, q),
                    None => name,
                };

                if !reward.trim().is_empty() {
                    rewards.push(reward.trim().to_string());
                }
            }

            if !code.is_empty() && !rewards.is_empty() {
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

    if codes.is_empty() {
        warn!("[Genshin][Codes][Game8] No codes found");
    } else {
        debug!(
            "[Genshin][Codes][Game8] Fetched {} codes total",
            codes.len()
        );
    }

    Ok(codes)
}

fn extract_quantity(text: &str) -> Option<String> {
    if let Some(pos) = text.find('x') {
        let mut digits = String::new();
        for c in text[pos + 1..].chars() {
            if c.is_ascii_digit() || c == ',' {
                digits.push(c);
            } else if !digits.is_empty() {
                break;
            }
        }
        if !digits.is_empty() {
            return Some(format!("x{}", digits));
        }
    }
    None
}
