use crate::global::Global;
use anyhow::Context as _;
use std::sync::Arc;

const FANDOM_API: &str = "https://zenless-zone-zero.fandom.com/api.php";

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
            ("page", "Redemption_Code"),
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

/// Parse the wikitext to extract Redemption Code Row entries.
///
/// Format: {{Redemption Code Row|CODE|ref=<ref>...</ref>|SERVER|{{Item List|item*qty;item*qty|mode=br}}|date|expiry}}
pub fn parse_wikitext(wikitext: &str) -> Vec<ParsedCode> {
    let mut codes = Vec::new();
    let marker = "{{Redemption Code Row";

    let mut search_from = 0;
    while let Some(start) = wikitext[search_from..].find(marker) {
        let abs_start = search_from + start;

        // Skip sub-templates like {{Redemption Code Row/...}}
        if wikitext[abs_start..].starts_with("{{Redemption Code Row/") {
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

    let parts = split_top_level_pipes(&cleaned);

    if parts.iter().any(|p| p.contains("notacode")) {
        return None;
    }

    let fields: Vec<&str> = parts
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter(|s| !s.starts_with("ref="))
        .collect();

    if fields.len() < 3 {
        return None;
    }

    let code_field = fields[0];
    let server_field = fields[1];

    if server_field == "CN" {
        return None;
    }

    if code_field.is_empty() {
        return None;
    }

    let rewards = extract_rewards(&fields);

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

fn split_top_level_pipes(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            depth += 1;
            current.push_str("{{");
            i += 2;
        } else if i + 1 < bytes.len() && bytes[i] == b'}' && bytes[i + 1] == b'}' {
            depth -= 1;
            current.push_str("}}");
            i += 2;
        } else if bytes[i] == b'|' && depth == 0 {
            parts.push(std::mem::take(&mut current));
            i += 1;
        } else {
            current.push(bytes[i] as char);
            i += 1;
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

fn extract_rewards(fields: &[&str]) -> Vec<String> {
    for field in fields {
        if let Some(start) = field.find("{{Item List|") {
            let after = &field[start + "{{Item List|".len()..];
            if let Some(end) = after.find("}}") {
                let inner = &after[..end];
                let items_part = inner.split('|').next().unwrap_or("");
                return parse_reward_items(items_part);
            }
        }
    }

    if fields.len() > 2 {
        return parse_reward_items(fields[2]);
    }

    Vec::new()
}

fn parse_reward_items(items: &str) -> Vec<String> {
    items
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
        .collect()
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
