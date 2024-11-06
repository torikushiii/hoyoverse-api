use crate::types::GameCode;
use reqwest::Client;
use scraper::{Html, Selector};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;

pub async fn fetch_codes() -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://www.eurogamer.net/honkai-star-rail-codes-livestream-active-working-how-to-redeem-9321";
    
    let response = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
        .send()
        .await
        .context("Failed to fetch Eurogamer page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    let mut codes = Vec::new();
    let current_time = Utc::now();

    fn parse_rewards(rewards_str: &str) -> Vec<String> {
        rewards_str
            .trim()
            .split(" and ")
            .flat_map(|s| {
                // First split by comma and space ", " to avoid splitting numbers
                s.split(", ")
            })
            .map(|s| s.replace("(new!)", "").trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    let list_selector = Selector::parse("#content_above > div.page_content > article > div > div > ul:nth-child(14) > li").unwrap();
    for item in document.select(&list_selector) {
        let text = item.text().collect::<String>().trim().to_string();
        if let Some((code, rewards_str)) = text.split_once(':') {
            let code = code.trim().to_string();
            let rewards = parse_rewards(rewards_str);

            if !code.is_empty() {
                codes.push(GameCode {
                    id: None,
                    code: code.to_string(),
                    active: true,
                    date: current_time.into(),
                    rewards: rewards.clone(),
                    source: "eurogamer".to_string(),
                });
            }
        }
    }

    // Parse table codes
    let table_selector = Selector::parse("table").unwrap();
    if let Some(table) = document.select(&table_selector).next() {
        let mut current_code = String::new();
        let mut current_rewards = Vec::new();
        let mut count = 0;

        for cell in table.text().collect::<String>().split('\n')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .skip(3) // Skip header rows
        {
            match count % 3 {
                0 => current_code = cell.to_string(),
                1 => current_rewards = parse_rewards(cell),
                2 => {
                    if !current_code.is_empty() {
                        codes.push(GameCode {
                            id: None,
                            code: current_code.clone(),
                            active: true,
                            date: current_time.into(),
                            rewards: current_rewards.clone(),
                            source: "eurogamer".to_string(),
                        });
                    }
                },
                _ => unreachable!(),
            }
            count += 1;
        }
    }

    if codes.is_empty() {
        warn!("No codes found on Eurogamer");
    } else {
        debug!("Found {} codes on Eurogamer", codes.len());
    }

    Ok(codes)
} 