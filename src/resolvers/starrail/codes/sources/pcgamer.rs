use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://www.pcgamer.com/honkai-star-rail-codes/";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch PCGamer page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    let current_time = Utc::now();

    let mut codes = Vec::new();

    let section_selector = Selector::parse("#section-active-honkai-star-rail-codes").unwrap();
    let ul_selector = Selector::parse("ul").unwrap();
    let li_selector = Selector::parse("li").unwrap();
    let strong_selector = Selector::parse("strong").unwrap();

    if let Some(_section) = document.select(&section_selector).next() {
        for ul in document.select(&ul_selector) {
            let mut found_codes = false;

            for li in ul.select(&li_selector) {
                if let Some(strong_element) = li.select(&strong_selector).next() {
                    let code_text = strong_element.text().collect::<String>().trim().to_string();

                    if code_text.len() >= 6 && code_text.chars().all(|c| c.is_alphanumeric()) {
                        found_codes = true;

                        let full_text = li.text().collect::<String>();
                        let mut rewards_text = full_text.replace(&code_text, "");
                        rewards_text = rewards_text.trim().to_string();

                        let rewards_text = rewards_text
                            .strip_prefix("-")
                            .unwrap_or(&rewards_text)
                            .trim();
                        let rewards = parse_rewards(rewards_text);

                        codes.push(GameCode {
                            id: None,
                            code: code_text,
                            active: true,
                            date: current_time.into(),
                            rewards,
                            source: "pcgamer".to_string(),
                        });
                    }
                }
            }

            if found_codes {
                break;
            }
        }
    }

    if codes.is_empty() {
        warn!("[StarRail][Codes][PCGamer] No codes found");
    } else {
        debug!("[StarRail][Codes][PCGamer] Found {} codes", codes.len());
    }

    Ok(codes)
}

fn parse_rewards(rewards_str: &str) -> Vec<String> {
    rewards_str
        .split(" and ")
        .flat_map(|part| part.split(", "))
        .map(|reward| {
            let cleaned = reward.replace("(NEW)", "").trim().to_string();

            format_reward(&cleaned)
        })
        .filter(|reward| !reward.is_empty())
        .collect()
}

fn format_reward(reward: &str) -> String {
    // Convert number words to digits and format as "Nx Item"
    let reward = reward.trim();

    // Handle patterns like "Three Traveler's Guide" -> "3x Traveler's Guide"
    let re =
        Regex::new(r"(?i)^(one|two|three|four|five|six|seven|eight|nine|ten|\d+)\s+(.+)$").unwrap();

    if let Some(caps) = re.captures(reward) {
        let number_str = &caps[1];
        let item_name = &caps[2];

        let number = match number_str.to_lowercase().as_str() {
            "one" => "1",
            "two" => "2",
            "three" => "3",
            "four" => "4",
            "five" => "5",
            "six" => "6",
            _ => number_str, // Already a digit
        };

        let formatted_item = item_name
            .split_whitespace()
            .map(|word| match word.to_lowercase().as_str() {
                "consumables" => "Consumables".to_string(),
                "purple" => "Purple".to_string(),
                _ => word.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ");

        format!("{}x {}", number, formatted_item)
    } else {
        // If no number pattern found, return as-is
        reward.to_string()
    }
}
