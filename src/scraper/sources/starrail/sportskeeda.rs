use crate::global::Global;
use regex::Regex;
use std::sync::{Arc, LazyLock};

const SPORTSKEEDA_URL: &str =
    "https://www.sportskeeda.com/esports/honkai-star-rail-hsr-4-0-redeem-codes";
const EXPIRED_MARKER: &str = "Expired Honkai Star Rail";

static LI_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<li>\s*<strong>([A-Za-z0-9]+):?</strong>:?\s*([^<]+)</li>")
        .expect("invalid regex")
});

#[derive(Debug)]
pub struct ParsedCode {
    pub code: String,
    pub rewards: Vec<String>,
}

#[tracing::instrument(skip(global))]
pub async fn scrape(global: &Arc<Global>) -> anyhow::Result<Vec<ParsedCode>> {
    let html = global
        .http_client
        .get(SPORTSKEEDA_URL)
        .send()
        .await?
        .text()
        .await?;

    let codes = parse_html(&html);

    tracing::info!(count = codes.len(), "scraped codes from sportskeeda");

    Ok(codes)
}

pub fn parse_html(html: &str) -> Vec<ParsedCode> {
    let active_html = match html.find(EXPIRED_MARKER) {
        Some(pos) => &html[..pos],
        None => html,
    };

    let mut results = Vec::new();

    for cap in LI_RE.captures_iter(active_html) {
        let code = cap[1].trim().to_uppercase();
        // Split rewards on commas, but be careful not to break thousands separators
        // (e.g. "10,000"). Rules per part after splitting on ',':
        //   - starts with uppercase             → always a new reward item
        //   - no leading space + starts digit   → thousands continuation, rejoin
        //   - leading space + starts digit      → new reward item (e.g. "50,000 Credit")
        let mut rewards: Vec<String> = Vec::new();
        let mut current = String::new();
        for part in cap[2].split(',') {
            let has_leading_space = part.starts_with(' ');
            let trimmed = part.trim();
            if current.is_empty() {
                current = trimmed.to_string();
                continue;
            }
            let is_new_item = match trimmed.chars().next() {
                Some(c) if c.is_uppercase() => true,
                Some(c) if c.is_ascii_digit() => has_leading_space,
                _ => false,
            };
            if is_new_item {
                rewards.push(current.clone());
                current = trimmed.to_string();
            } else {
                current.push(',');
                current.push_str(trimmed);
            }
        }
        if !current.is_empty() {
            rewards.push(current);
        }
        let rewards: Vec<String> = rewards.into_iter().filter(|r| !r.is_empty()).collect();

        if !code.is_empty() {
            results.push(ParsedCode { code, rewards });
        }
    }

    results
}
