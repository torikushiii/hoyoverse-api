use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://game8.co/games/Honkai-Star-Rail/archives/410296";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Game8 page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let title_selector = Selector::parse("h2").unwrap();
    let table_selector = Selector::parse("table.a-table").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();
    let input_selector = Selector::parse("input.a-clipboard__textInput").unwrap();
    let reward_div_selector = Selector::parse("div.align").unwrap();

    let mut codes = Vec::new();
    let current_time = Utc::now();

    // Find the correct section by title
    for title in document.select(&title_selector) {
        if title
            .text()
            .collect::<String>()
            .contains("Active Redeem Codes for")
        {
            if let Some(table) = document.select(&table_selector).next() {
                for row in table.select(&row_selector).skip(1) {
                    let cells: Vec<_> = row.select(&cell_selector).collect();

                    if cells.len() >= 2 {
                        let code_cell = &cells[0];
                        let rewards_cell = &cells[1];

                        if let Some(input_element) = code_cell.select(&input_selector).next() {
                            if let Some(code_value) = input_element.value().attr("value") {
                                let code = code_value.trim().to_string();
                                let mut rewards = Vec::new();

                                for reward_div in rewards_cell.select(&reward_div_selector) {
                                    let reward_text = reward_div.text().collect::<String>();

                                    let reward_text = reward_text
                                        .lines()
                                        .map(|line| line.trim())
                                        .filter(|line| !line.is_empty())
                                        .collect::<Vec<_>>()
                                        .join(" ");

                                    if !reward_text.is_empty() {
                                        rewards.push(reward_text);
                                    }
                                }

                                codes.push(GameCode {
                                    id: None,
                                    code: code,
                                    active: true,
                                    date: current_time.into(),
                                    rewards: rewards,
                                    source: "game8".to_string(),
                                });
                            }
                        }
                    }
                }
            }
            break; // We found and processed the section we wanted
        }
    }

    if codes.is_empty() {
        warn!("[StarRail][Codes][Game8] No codes found");
    } else {
        debug!("[StarRail][Codes][Game8] Found {} codes", codes.len());
    }

    Ok(codes)
}
