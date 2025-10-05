use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://tot.wiki/wiki/Redeem_Code";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch tot.wiki page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let tbody_selector = Selector::parse("tbody").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();

    let mut codes = Vec::new();
    let current_time = Utc::now();

    if let Some(tbody) = document.select(&tbody_selector).next() {
        for row in tbody.select(&row_selector).skip(1) {
            let cells: Vec<_> = row.select(&cell_selector).collect();

            if cells.len() > 2 {
                let code = cells[1].text().collect::<String>().trim().to_uppercase();
                let rewards_text = cells[2].text().collect::<String>();

                let mut rewards = Vec::new();
                let mut current_reward = String::new();

                for part in rewards_text.split(',') {
                    let part = part.trim();
                    if current_reward
                        .chars()
                        .last()
                        .map_or(false, |c| c.is_ascii_digit())
                        && part.chars().next().map_or(false, |c| c.is_ascii_digit())
                    {
                        // If current reward ends with number and next part starts with number,
                        // treat it as a thousands separator
                        current_reward.push(',');
                        current_reward.push_str(part);
                    } else {
                        if !current_reward.is_empty() {
                            rewards.push(current_reward.trim().to_string());
                        }
                        current_reward = part.to_string();
                    }
                }
                if !current_reward.is_empty() {
                    rewards.push(current_reward.trim().to_string());
                }

                if !code.is_empty() {
                    codes.push(GameCode {
                        id: None,
                        code,
                        active: true,
                        date: current_time.into(),
                        rewards,
                        source: "totwiki".to_string(),
                    });
                }
            }
        }
    }

    if codes.is_empty() {
        warn!("[Themis][Codes][TotWiki] No codes found");
    } else {
        debug!("[Themis][Codes][TotWiki] Found {} codes", codes.len());
    }

    Ok(codes)
}
