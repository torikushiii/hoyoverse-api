use std::sync::Arc;

use mongodb::bson::doc;
use regex::Regex;

use crate::database::redemption_code::RedemptionCode;
use crate::games::Game;
use crate::global::Global;

const TOT_WIKI_URL: &str = "https://tot.wiki/wiki/Redeem_Code";
const SOURCE: &str = "tot_wiki";

#[derive(Debug)]
pub struct ParsedCode {
    pub code: String,
    pub rewards: Vec<String>,
}

#[tracing::instrument(skip(global))]
pub async fn scrape(global: &Arc<Global>) -> anyhow::Result<Vec<ParsedCode>> {
    let html = global
        .http_client
        .get(TOT_WIKI_URL)
        .send()
        .await?
        .text()
        .await?;

    let codes = parse_html(&html);

    tracing::info!(count = codes.len(), "scraped codes from tot_wiki");

    Ok(codes)
}

pub fn parse_html(html: &str) -> Vec<ParsedCode> {
    let table_start = match html.find(r#"class="wikitable""#) {
        Some(pos) => pos,
        None => return Vec::new(),
    };
    let table_end = match html[table_start..].find("</table>") {
        Some(pos) => table_start + pos,
        None => html.len(),
    };
    let table_html = &html[table_start..table_end];

    let row_re = Regex::new(
        r"(?s)<tr>\s*<td[^>]*>.*?</td>\s*<td[^>]*>\s*(.*?)\s*</td>\s*<td[^>]*>(.*?)</td>",
    )
    .expect("invalid row regex");
    let tag_re = Regex::new(r"<[^>]*>").expect("invalid tag regex");

    let mut results = Vec::new();

    for cap in row_re.captures_iter(table_html) {
        let code_field = cap[1].trim().to_string();

        if code_field.is_empty() {
            continue;
        }

        let rewards_html = &cap[2];
        let rewards_text = tag_re.replace_all(rewards_html, "");
        let rewards = parse_rewards(&rewards_text);

        // A cell can contain multiple codes separated by ", "
        for code in code_field
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
        {
            results.push(ParsedCode {
                code,
                rewards: rewards.clone(),
            });
        }
    }

    results
}

fn parse_rewards(text: &str) -> Vec<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut items: Vec<String> = Vec::new();
    for part in normalized.split(',') {
        let trimmed = part.trim_start();
        if !items.is_empty() && trimmed.starts_with(|c: char| c.is_ascii_digit()) {
            let last = items.last_mut().unwrap();
            last.push(',');
            last.push_str(part);
        } else {
            items.push(part.to_string());
        }
    }

    items
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|item| {
            if let Some((name, qty)) = item.rsplit_once(" x") {
                format!("{} ×{}", name.trim(), qty.trim())
            } else {
                item.to_string()
            }
        })
        .collect()
}

#[tracing::instrument(skip(global))]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<usize> {
    let scraped = scrape(global).await?;
    let collection = RedemptionCode::collection(&global.db, Game::Themis);

    let mut new_count = 0;

    for parsed in &scraped {
        let exists = collection
            .count_documents(doc! { "code": &parsed.code })
            .await?
            > 0;

        if exists {
            continue;
        }

        let doc = RedemptionCode {
            code: parsed.code.clone(),
            active: true,
            date: bson::DateTime::now(),
            rewards: parsed.rewards.clone(),
            source: SOURCE.to_string(),
        };

        collection.insert_one(doc).await?;
        tracing::info!(code = parsed.code, "new code discovered");
        new_count += 1;
    }

    tracing::info!(
        new = new_count,
        total = scraped.len(),
        "tot_wiki scrape complete"
    );

    Ok(new_count)
}
