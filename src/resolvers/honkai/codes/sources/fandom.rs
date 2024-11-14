use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector};
use anyhow::Context;
use tracing::debug;
use chrono::Utc;
use regex::Regex;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://honkaiimpact3.fandom.com/wiki/Exchange_Rewards";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Fandom wiki")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let table_selector = Selector::parse("table.wikitable tbody").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();
    let reward_box_selector = Selector::parse(".infobox-half").unwrap();
    let reward_text_selector = Selector::parse(".infobox-solid div").unwrap();

    let brackets_regex = Regex::new(r"\[.*?\]").unwrap();
    let nonword_regex = Regex::new(r"[^\w]").unwrap();

    let mut codes = Vec::new();
    let current_time = Utc::now();

    if let Some(table) = document.select(&table_selector).next() {
        for row in table.select(&row_selector).skip(1) {
            let cells: Vec<_> = row.select(&cell_selector).collect();

            if cells.len() < 5 {
                continue;
            }

            let code = cells[0].text().collect::<String>();
            let code = brackets_regex.replace_all(&code, "");
            let code = nonword_regex.replace_all(&code, "").trim().to_string();

            if code.is_empty() {
                continue;
            }

            let occasion = cells[2].text().collect::<String>().trim().to_string();

            let mut rewards: Vec<String> = cells[3].select(&reward_box_selector)
                .filter_map(|reward| {
                    reward.select(&reward_text_selector)
                        .next()
                        .map(|div| {
                            div.text()
                                .collect::<String>()
                                .trim()
                                .replace('\u{a0}', " ")
                                .replace("  ", " ")
                                .to_string()
                        })
                })
                .filter(|s| !s.is_empty())
                .collect();

            if rewards.is_empty() {
                rewards = vec![occasion];
            }

            let expiration_text = cells[4].text().collect::<String>()
                .trim()
                .to_lowercase();

            let is_active = expiration_text != "yes";

            codes.push(GameCode {
                id: None,
                code,
                active: is_active,
                date: current_time.into(),
                rewards,
                source: "fandom".to_string(),
            });
        }
    }

    debug!("[Honkai][Codes][Fandom] Found {} codes", codes.len());
    Ok(codes)
}
