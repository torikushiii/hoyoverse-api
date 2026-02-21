use crate::global::Global;
use anyhow::Context as _;
use std::sync::Arc;
const FANDOM_API: &str = "https://genshin-impact.fandom.com/api.php";

#[derive(Debug)]
pub struct ParsedCode {
    pub code: String,
    pub rewards: Vec<String>,
}

#[tracing::instrument(skip(global))]
pub async fn scrape(global: &Arc<Global>) -> anyhow::Result<Vec<ParsedCode>> {
    let resp = global
        .http_client
        .get(FANDOM_API)
        .query(&[
            ("action", "parse"),
            ("page", "Promotional_Code"),
            ("format", "json"),
            ("prop", "wikitext"),
        ])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let wikitext = resp["parse"]["wikitext"]["*"]
        .as_str()
        .context("failed to extract wikitext")?;

    let codes = parse_wikitext(wikitext);

    tracing::info!(count = codes.len(), "scraped codes from fandom");

    Ok(codes)
}

/// Parse the wikitext to extract Code Row entries.
///
/// Format: {{Code Row|CODE1;CODE2|SERVER|reward1*qty;reward2*qty|date|expiry}}
pub fn parse_wikitext(wikitext: &str) -> Vec<ParsedCode> {
    let mut codes = Vec::new();
    let marker = "{{Code Row";

    let mut search_from = 0;
    while let Some(start) = wikitext[search_from..].find(marker) {
        let abs_start = search_from + start;

        if wikitext[abs_start..].starts_with("{{Code Row/") {
            search_from = abs_start + marker.len();
            continue;
        }

        let content_start = abs_start + marker.len();
        let mut depth = 1;
        let mut i = content_start;
        let bytes = wikitext.as_bytes();

        while i < bytes.len() - 1 && depth > 0 {
            if bytes[i] == b'{' && bytes[i + 1] == b'{' {
                depth += 1;
                i += 2;
            } else if bytes[i] == b'}' && bytes[i + 1] == b'}' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                i += 2;
            } else {
                i += 1;
            }
        }

        if depth == 0 {
            let inner = &wikitext[content_start..i];

            if let Some(parsed) = parse_code_row(inner) {
                codes.extend(parsed);
            }
        }

        search_from = i + 2;
    }

    codes
}

fn parse_code_row(inner: &str) -> Option<Vec<ParsedCode>> {
    let cleaned = strip_html_comments(inner).replace(['\n', '\t'], "");

    let parts: Vec<&str> = cleaned
        .split('|')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() < 3 {
        return None;
    }

    let code_field = parts[0];
    let server_field = parts[1];
    let rewards_field = parts[2];

    if server_field == "CN" {
        return None;
    }

    if code_field.contains("notacode") || code_field.is_empty() {
        return None;
    }

    if parts.iter().any(|p| p.contains("notacode")) {
        return None;
    }

    let rewards: Vec<String> = rewards_field
        .split(';')
        .map(|r| {
            let r = r.trim();
            if let Some((name, qty)) = r.rsplit_once('*') {
                format!("{} ×{}", name.trim(), qty.trim())
            } else {
                r.to_string()
            }
        })
        .filter(|r| !r.is_empty())
        .collect();

    let parsed: Vec<ParsedCode> = code_field
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|code_str| ParsedCode {
            code: code_str.to_string(),
            rewards: rewards.clone(),
        })
        .collect();

    Some(parsed)
}

fn strip_html_comments(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut rest = s;

    while let Some(start) = rest.find("<!--") {
        result.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find("-->") {
            rest = &rest[start + end + 3..];
        } else {
            break;
        }
    }

    result.push_str(rest);
    result
}
