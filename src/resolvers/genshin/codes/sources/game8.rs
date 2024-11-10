use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector, ElementRef, CaseSensitivity};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://game8.co/games/Genshin-Impact/archives/304759";
    
    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Game8 page")?;

    if !response.status().is_success() {
        warn!("[Genshin][Codes][Game8] Failed to fetch data, status: {}", response.status());
        return Ok(Vec::new());
    }

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let mut codes = Vec::new();
    let current_time = Utc::now();

    let headers_selector = Selector::parse("h2, h3").unwrap();
    let list_item_selector = Selector::parse("li.a-listItem").unwrap();
    let link_selector = Selector::parse("a.a-link").unwrap();

    for header in document.select(&headers_selector) {
        let header_text = header.text().collect::<String>();
        
        if header_text.contains("Active Redeem Codes in Version") || 
           header_text.contains("Special Program Codes") {

            if let Some(list) = header.next_siblings()
                .find(|sibling| {
                    if let Some(element) = sibling.value().as_element() {
                        element.has_class("a-orderedList", CaseSensitivity::AsciiCaseInsensitive)
                    } else {
                        false
                    }
                })
                .and_then(|node| ElementRef::wrap(node))
            {
                for item in list.select(&list_item_selector) {
                    let item_text = item.text().collect::<String>();
                    
                    if item_text.contains("EXPIRED") {
                        continue;
                    }

                    let code = item.select(&link_selector)
                        .next()
                        .map(|link| link.text().collect::<String>())
                        .unwrap_or_default()
                        .trim()
                        .to_string();

                    let rewards = if let Some(rewards_text) = item_text.split('-').nth(1) {
                        let rewards_text = rewards_text.trim();
                        
                        if header_text.contains("Active Redeem Codes in Version") {
                            // Process rewards for active codes section
                            // Store the processed string to avoid temporary value issues
                            let processed_text = rewards_text.replace(" and ", ",");
                            let parts: Vec<&str> = processed_text
                                .split(',')
                                .map(|s| s.trim())
                                .collect();

                            let mut processed_rewards = Vec::new();
                            let mut i = 0;
                            while i < parts.len() {
                                let current = parts[i];
                                
                                // Check if current part is a number and next part starts with "000"
                                if i + 1 < parts.len() && 
                                   current.chars().all(|c| c.is_digit(10)) && 
                                   parts[i + 1].starts_with("000") {
                                    // Combine the number with its thousand part
                                    processed_rewards.push(format!("{},{}", current, &parts[i + 1]));
                                    i += 2;
                                } else if !current.chars().all(|c| c.is_digit(10)) {
                                    // Add non-numeric parts as is
                                    processed_rewards.push(current.to_string());
                                    i += 1;
                                } else {
                                    // Skip standalone numbers
                                    i += 1;
                                }
                            }
                            
                            processed_rewards
                        } else {
                            vec![rewards_text.to_string()]
                        }
                    } else {
                        Vec::new()
                    };

                    if !code.is_empty() && !rewards.is_empty() {
                        debug!("[Genshin][Codes][Game8] Found code: {} with rewards: {:?}", code, rewards);
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
        }
    }

    if codes.is_empty() {
        warn!("[Genshin][Codes][Game8] No codes found");
    } else {
        debug!("[Genshin][Codes][Game8] Fetched {} codes total", codes.len());
    }

    Ok(codes)
} 