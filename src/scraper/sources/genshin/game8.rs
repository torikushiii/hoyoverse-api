use std::sync::Arc;
use regex::Regex;
use crate::global::Global;

const GAME8_URL: &str = "https://game8.co/games/Genshin-Impact/archives/304759";
const EXPIRED_MARKER: &str = "Expired Genshin Impact Redeem Codes";

#[derive(Debug)]
pub struct ParsedCode {
    pub code: String,
    pub rewards: Vec<String>,
}

#[tracing::instrument(skip(global))]
pub async fn scrape(global: &Arc<Global>) -> anyhow::Result<Vec<ParsedCode>> {
    let html = global
        .http_client
        .get(GAME8_URL)
        .send()
        .await?
        .text()
        .await?;

    let codes = parse_html(&html);

    tracing::info!(count = codes.len(), "scraped codes from game8");

    Ok(codes)
}

pub fn parse_html(html: &str) -> Vec<ParsedCode> {
    let active_html = match html.find(EXPIRED_MARKER) {
        Some(pos) => &html[..pos],
        None => html,
    };

    let row_re =
        Regex::new(r"(?s)<tr>\s*<td.*?>(.*?)</td>\s*<td.*?>(.*?)</td>").expect("invalid row regex");
    let code_re = Regex::new(r#"value="([^"]+)""#).expect("invalid code regex");
    let fallback_code_re =
        Regex::new(r"gift\?code=([A-Z0-9]{8,})").expect("invalid fallback code regex");
    let reward_re = Regex::new(r#"(?s)<div class="align">.*?<a.*?>(.*?)</a>\s*(x[\d,]+)"#)
        .expect("invalid reward regex");
    let tag_re = Regex::new(r"<[^>]*>").expect("invalid tag regex");

    let mut results = Vec::new();

    for cap in row_re.captures_iter(active_html) {
        let code_td = &cap[1];
        let rewards_td = &cap[2];

        let code = code_re
            .captures(code_td)
            .map(|c| c[1].to_string())
            .or_else(|| fallback_code_re.captures(code_td).map(|c| c[1].to_string()));

        if let Some(code) = code {
            let mut rewards = Vec::new();
            for r_cap in reward_re.captures_iter(rewards_td) {
                let name_html = &r_cap[1];
                let qty = &r_cap[2];
                let name = tag_re.replace_all(name_html, "").trim().to_string();

                if !name.is_empty() {
                    let qty = qty.trim_start_matches('x');
                    rewards.push(format!("{} ×{}", name, qty));
                }
            }

            results.push(ParsedCode {
                code: code.to_uppercase(),
                rewards,
            });
        }
    }

    results.sort_by(|a, b| a.code.cmp(&b.code));
    results.dedup_by(|a, b| a.code == b.code);

    results
}
