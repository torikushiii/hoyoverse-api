use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;
use regex::Regex;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://game8.co/games/Zenless-Zone-Zero/archives/435683";
    
    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Game8 page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let codes = parse_codes_from_html(&document)?;

    if codes.is_empty() {
        warn!("[Zenless][Codes][Game8] No codes found");
    } else {
        debug!("[Zenless][Codes][Game8] Found {} codes", codes.len());
    }

    Ok(codes)
}

fn parse_codes_from_html(document: &Html) -> anyhow::Result<Vec<GameCode>> {
    let item_selector = Selector::parse("li.a-listItem").unwrap();
    let link_selector = Selector::parse("a.a-link").unwrap();
    let code_regex = Regex::new(r"^[A-Z0-9]+$").unwrap();
    let current_time = Utc::now();
    
    let codes = document.select(&item_selector)
        .filter_map(|item| {
            let item_text = item.text().collect::<String>();
            
            // Skip items that don't contain a dash
            if !item_text.contains('-') {
                return None;
            }

            // Try to get code from a-link first, then fallback to text before dash
            let code = item.select(&link_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| {
                    item_text
                        .split('-')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string()
                });

            // Validate code format
            if !code_regex.is_match(&code) {
                return None;
            }

            // Extract rewards text after the first dash
            let rewards_text = item_text
                .split_once('-')
                .map(|(_, rewards)| rewards.trim())?;

            // Split rewards by looking ahead for a comma followed by a letter
            let mut rewards = Vec::new();
            let mut current_reward = String::new();
            let mut chars = rewards_text.chars().peekable();

            while let Some(c) = chars.next() {
                current_reward.push(c);
                
                if c == ',' {
                    // Look ahead to see if the next non-whitespace character is a letter
                    let mut peek_iter = chars.clone();
                    let next_non_whitespace = peek_iter
                        .find(|&c| !c.is_whitespace());
                    
                    if let Some(next_char) = next_non_whitespace {
                        if next_char.is_alphabetic() {
                            // This comma separates rewards
                            let reward = current_reward[..current_reward.len()-1].trim().to_string();
                            if !reward.is_empty() {
                                rewards.push(reward);
                            }
                            current_reward.clear();
                        }
                    }
                }
            }

            // Add the last reward
            let final_reward = current_reward.trim().to_string();
            if !final_reward.is_empty() {
                rewards.push(final_reward);
            }

            if rewards.is_empty() {
                return None;
            }

            Some(GameCode {
                id: None,
                code,
                active: true,
                date: current_time.into(),
                rewards,
                source: "game8".to_string(),
            })
        })
        .collect();

    Ok(codes)
} 