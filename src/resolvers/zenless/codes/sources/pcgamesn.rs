use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector, Element};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://www.pcgamesn.com/zenless-zone-zero/codes";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch PCGamesN page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let codes = parse_codes_from_html(&document)?;

    if codes.is_empty() {
        warn!("[Zenless][Codes][PCGamesN] No codes found");
    } else {
        debug!("[Zenless][Codes][PCGamesN] Found {} codes", codes.len());
    }

    Ok(codes)
}

fn parse_codes_from_html(document: &Html) -> anyhow::Result<Vec<GameCode>> {
    let p_selector = Selector::parse("p").unwrap();
    let li_selector = Selector::parse("li").unwrap();
    let strong_selector = Selector::parse("strong").unwrap();
    let current_time = Utc::now();

    let mut codes = Vec::new();

    let target_p = document.select(&p_selector)
        .find(|p| p.text().collect::<String>().contains("Here are all the ZZZ redeem codes:"));

    if let Some(target_p) = target_p {
        // Find the next ul element after the target paragraph
        let mut next_element = target_p.next_sibling_element();
        while let Some(element) = next_element {
            if element.value().name() == "ul" {
                for li in element.select(&li_selector) {
                    // Get code from strong tag
                    if let Some(strong) = li.select(&strong_selector).next() {
                        let code = strong.text().collect::<String>().trim().to_string();

                        // Get rewards text after the dash
                        let full_text = li.text().collect::<String>();
                        if let Some(rewards_text) = full_text.split('–').nth(1) {
                            let cleaned_text = rewards_text.trim().replace("(NEW)", "").trim().to_string();

                            // Replace ", and " with "," then split by comma
                            let rewards: Vec<String> = cleaned_text
                                .replace(", and ", ",")
                                .split(',')
                                .map(|r| r.trim().to_string())
                                .filter(|r| !r.is_empty())
                                .collect();

                            if !code.is_empty() && !rewards.is_empty() {
                                codes.push(GameCode {
                                    id: None,
                                    code,
                                    active: true,
                                    date: current_time.into(),
                                    rewards,
                                    source: "pcgamesn".to_string(),
                                });
                            }
                        }
                    }
                }
                break;
            }
            next_element = element.next_sibling_element();
        }
    }

    Ok(codes)
}