use crate::global::Global;
use regex::Regex;
use std::sync::{Arc, LazyLock};

const GAME8_URL: &str = "https://game8.co/games/Zenless-Zone-Zero/archives/435683";
const EXPIRED_MARKER: &str = "All Expired ZZZ Codes";

static ROW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<tr>\s*<td.*?>(.*?)</td>\s*<td.*?>(.*?)</td>").expect("invalid row regex")
});
static CODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"value=['"]([^'"]+)['"]"#).expect("invalid code regex"));
static FALLBACK_CODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"redemption\?code=([A-Za-z0-9]+)").expect("invalid fallback code regex")
});
static REWARD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<div class=['"]align['"]>.*?<a.*?>(.*?)</a>\s*x?\s*([\d,]+)"#)
        .expect("invalid reward regex")
});
static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]*>").expect("invalid tag regex"));

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

    let mut results = Vec::new();

    for cap in ROW_RE.captures_iter(active_html) {
        let code_td = &cap[1];
        let rewards_td = &cap[2];

        let code = CODE_RE
            .captures(code_td)
            .map(|c| c[1].to_string())
            .or_else(|| FALLBACK_CODE_RE.captures(code_td).map(|c| c[1].to_string()));

        if let Some(code) = code {
            let mut rewards = Vec::new();
            for r_cap in REWARD_RE.captures_iter(rewards_td) {
                let name_html = &r_cap[1];
                let qty = &r_cap[2];
                let name = TAG_RE.replace_all(name_html, "").trim().to_string();

                if !name.is_empty() {
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
