use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://www.prydwen.gg/star-rail/";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Prydwen page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let box_selector = Selector::parse(".codes .box").unwrap();
    let code_selector = Selector::parse(".code").unwrap();
    let rewards_selector = Selector::parse(".rewards").unwrap();

    let mut codes = Vec::new();
    let current_time = Utc::now();

    for box_element in document.select(&box_selector) {
        if let Some(code_element) = box_element.select(&code_selector).next() {
            let code = code_element
                .text()
                .collect::<String>()
                .trim()
                .replace("NEW!", "")
                .trim()
                .to_string();

            if code.is_empty() {
                continue;
            }

            let rewards =
                if let Some(rewards_element) = box_element.select(&rewards_selector).next() {
                    rewards_element
                        .text()
                        .collect::<String>()
                        .trim()
                        .split('+')
                        .map(|reward| reward.trim().to_string())
                        .filter(|reward| !reward.is_empty())
                        .collect()
                } else {
                    Vec::new()
                };

            codes.push(GameCode {
                id: None,
                code,
                active: true,
                date: current_time.into(),
                rewards,
                source: "prydwen".to_string(),
            });
        }
    }

    if codes.is_empty() {
        warn!("[StarRail][Codes][Prydwen] No codes found");
    } else {
        debug!("[StarRail][Codes][Prydwen] Found {} codes", codes.len());
    }

    Ok(codes)
}
