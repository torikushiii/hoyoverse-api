use anyhow::Context as _;
use std::sync::Arc;

use crate::global::Global;

const FANDOM_API: &str = "https://honkaiimpact3.fandom.com/api.php";

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
            ("page", "Exchange_Rewards"),
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

/// Parse only the active codes section (before `==Legacy`).
///
/// Each row looks like:
/// `|'''CODE'''||Feb 9, 26||Occasion||{{Item|Asterite|rarity=4|size=70|quantity=500}}...`
pub fn parse_wikitext(wikitext: &str) -> Vec<ParsedCode> {
    let active_end = wikitext
        .find("==Legacy")
        .or_else(|| wikitext.find("== Legacy"))
        .unwrap_or(wikitext.len());
    let active_section = &wikitext[..active_end];

    let mut codes = Vec::new();

    for line in active_section.lines() {
        let line = line.trim();

        // Skip non-data lines: separators, captions, headers, non-table lines
        if !line.starts_with('|')
            || line.starts_with("|-")
            || line.starts_with("|+")
            || line.starts_with("|{")
        {
            continue;
        }

        // Split into columns — expect at least 4 (Code | Date | Occasion | Rewards)
        let cols: Vec<&str> = line.splitn(5, "||").collect();
        if cols.len() < 4 {
            continue;
        }

        // Column 0 has a leading `|` from the wikitext row syntax
        let code_cell = cols[0].trim_start_matches('|').trim();
        let code = match extract_bold(code_cell) {
            Some(c) if !c.is_empty() => c.to_uppercase(),
            _ => continue,
        };

        let rewards = parse_item_templates(cols[cols.len() - 1]);

        codes.push(ParsedCode { code, rewards });
    }

    codes
}

/// Extract text from `'''...'''` bold markup. Returns `None` if not bold.
fn extract_bold(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.starts_with("'''") && s.ends_with("'''") && s.len() > 6 {
        Some(&s[3..s.len() - 3])
    } else {
        None
    }
}

/// Parse `{{Item|name|rarity=N|size=70|quantity=N}}` templates into reward strings.
fn parse_item_templates(s: &str) -> Vec<String> {
    let mut rewards = Vec::new();
    let mut rest = s;

    while let Some(start) = rest.find("{{Item|") {
        rest = &rest[start + "{{Item|".len()..];

        let Some(end) = find_closing_braces(rest) else {
            break;
        };

        let inner = &rest[..end];
        let parts: Vec<&str> = inner.split('|').collect();

        if let Some(name) = parts.first() {
            let name = name.trim();
            let quantity = parts
                .iter()
                .find(|p| p.starts_with("quantity="))
                .and_then(|p| p.strip_prefix("quantity="))
                .unwrap_or("");

            if !name.is_empty() {
                if !quantity.is_empty() {
                    rewards.push(format!("{} ×{}", name, quantity));
                } else {
                    rewards.push(name.to_string());
                }
            }
        }

        rest = &rest[end + 2..];
    }

    rewards
}

/// Find the byte index of the `}}` that closes the outermost `{{`.
fn find_closing_braces(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 1usize;
    let mut i = 0;

    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
            i += 2;
        } else {
            i += 1;
        }
    }

    None
}
