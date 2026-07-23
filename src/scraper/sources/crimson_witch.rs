use std::collections::HashSet;

use anyhow::Context as _;
use chrono::{DateTime, Utc};
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use serde_json::Value;

const NEXT_PUSH_MARKER: &str = "self.__next_f.push(";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrimsonCode {
    pub code: String,
    pub rewards: Vec<String>,
}

#[derive(serde::Deserialize)]
struct WireCode {
    code: String,
    #[serde(default)]
    code_variants: Option<String>,
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    expires: Option<String>,
    #[serde(default)]
    rewards: Vec<WireReward>,
}

#[derive(serde::Deserialize)]
struct WireReward {
    item: Option<String>,
    qty: Option<Value>,
}

pub async fn scrape(client: &reqwest::Client, url: &str) -> anyhow::Result<Vec<CrimsonCode>> {
    let html = client
        .get(url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36",
        )
        .header(
            ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        )
        .header(ACCEPT_LANGUAGE, "en-US,en;q=0.9")
        .send()
        .await
        .context("failed to fetch Crimson Witch codes")?
        .error_for_status()
        .context("Crimson Witch returned an error status")?
        .text()
        .await
        .context("failed to read Crimson Witch response")?;

    parse_html_at(&html, Utc::now())
}

fn parse_html_at(html: &str, now: DateTime<Utc>) -> anyhow::Result<Vec<CrimsonCode>> {
    let rows = extract_initial_codes(html)?;
    let mut seen = HashSet::new();
    let mut codes = Vec::new();

    for row in rows {
        let row: WireCode =
            serde_json::from_value(row).context("failed to parse Crimson Witch code record")?;
        if !is_current(&row, now) {
            continue;
        }

        let rewards = row
            .rewards
            .into_iter()
            .filter_map(format_reward)
            .collect::<Vec<_>>();
        let candidates = std::iter::once(row.code).chain(
            row.code_variants
                .into_iter()
                .flat_map(|variants| split_variants(&variants)),
        );

        for candidate in candidates {
            let code = candidate.trim().to_uppercase();
            if code.is_empty() || !seen.insert(code.clone()) {
                continue;
            }
            codes.push(CrimsonCode {
                code,
                rewards: rewards.clone(),
            });
        }
    }

    if codes.is_empty() {
        anyhow::bail!("Crimson Witch returned no current codes");
    }
    Ok(codes)
}

fn is_current(row: &WireCode, now: DateTime<Utc>) -> bool {
    if row
        .start_date
        .as_deref()
        .and_then(parse_timestamp)
        .is_some_and(|start| start > now)
    {
        return false;
    }
    row.expires
        .as_deref()
        .and_then(parse_timestamp)
        .is_none_or(|expires| expires > now)
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}

fn split_variants(value: &str) -> Vec<String> {
    value
        .split([';', ','])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn format_reward(reward: WireReward) -> Option<String> {
    let item = reward.item?.trim().to_string();
    if item.is_empty() {
        return None;
    }
    let quantity = match reward.qty? {
        Value::Number(value) => value.to_string(),
        Value::String(value) if !value.trim().is_empty() => value,
        _ => return None,
    };
    Some(format!("{item} ×{quantity}"))
}

fn extract_initial_codes(html: &str) -> anyhow::Result<Vec<Value>> {
    let mut offset = 0;
    while let Some(relative) = html[offset..].find(NEXT_PUSH_MARKER) {
        let marker_end = offset + relative + NEXT_PUSH_MARKER.len();
        let Some(array_start) = html[marker_end..].find('[').map(|index| marker_end + index) else {
            break;
        };
        let Some(argument) = extract_balanced_json(html, array_start) else {
            offset = marker_end;
            continue;
        };
        if let Ok(value) = serde_json::from_str::<Value>(argument)
            && let Some(rows) = find_initial_codes(&value)
        {
            return Ok(rows);
        }
        offset = array_start + argument.len();
    }
    anyhow::bail!("Crimson Witch response contained no initialCodes payload")
}

fn find_initial_codes(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::Object(object) => {
            if let Some(Value::Array(rows)) = object.get("initialCodes") {
                return Some(rows.clone());
            }
            object.values().find_map(find_initial_codes)
        }
        Value::Array(values) => values.iter().find_map(find_initial_codes),
        Value::String(value) if value.contains("initialCodes") => {
            parse_embedded_json(value).and_then(|parsed| find_initial_codes(&parsed))
        }
        _ => None,
    }
}

fn parse_embedded_json(value: &str) -> Option<Value> {
    for (index, character) in value.char_indices() {
        if character != '[' && character != '{' {
            continue;
        }
        let Some(candidate) = extract_balanced_json(value, index) else {
            continue;
        };
        if let Ok(parsed) = serde_json::from_str(candidate) {
            return Some(parsed);
        }
    }
    None
}

fn extract_balanced_json(value: &str, start: usize) -> Option<&str> {
    let bytes = value.as_bytes();
    let opening = *bytes.get(start)?;
    if opening != b'[' && opening != b'{' {
        return None;
    }

    let mut stack = vec![opening];
    let mut in_string = false;
    let mut escaped = false;

    for (index, &byte) in bytes.iter().enumerate().skip(start + 1) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'[' | b'{' => stack.push(byte),
            b']' => {
                if stack.pop() != Some(b'[') {
                    return None;
                }
            }
            b'}' => {
                if stack.pop() != Some(b'{') {
                    return None;
                }
            }
            _ => {}
        }

        if stack.is_empty() {
            return value.get(start..=index);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::*;

    fn html_with_codes(codes: &str) -> String {
        let payload = format!(r#"16:["$","component",null,{{"initialCodes":{codes}}}]"#);
        let push =
            serde_json::to_string(&Value::Array(vec![Value::from(1), Value::String(payload)]))
                .unwrap();
        format!("<script>self.__next_f.push({push})</script>")
    }

    #[test]
    fn parses_variants_rewards_and_normalization() {
        let html = html_with_codes(
            r#"[{"code":" testcode ","code_variants":"altOne; altTwo,ALTONE","start_date":null,"expires":null,"rewards":[{"item":"Primogem","qty":60},{"item":"Mora","qty":"10000"}]}]"#,
        );
        let now = Utc.with_ymd_and_hms(2026, 7, 23, 0, 0, 0).unwrap();
        let codes = parse_html_at(&html, now).unwrap();

        assert_eq!(
            codes,
            vec![
                CrimsonCode {
                    code: "TESTCODE".to_string(),
                    rewards: vec!["Primogem ×60".to_string(), "Mora ×10000".to_string()],
                },
                CrimsonCode {
                    code: "ALTONE".to_string(),
                    rewards: vec!["Primogem ×60".to_string(), "Mora ×10000".to_string()],
                },
                CrimsonCode {
                    code: "ALTTWO".to_string(),
                    rewards: vec!["Primogem ×60".to_string(), "Mora ×10000".to_string()],
                },
            ]
        );
    }

    #[test]
    fn filters_future_and_expired_codes() {
        let html = html_with_codes(
            r#"[
                {"code":"CURRENT","start_date":null,"expires":null,"rewards":[]},
                {"code":"FUTURE","start_date":"2026-07-24T00:00:00+00:00","expires":null,"rewards":[]},
                {"code":"EXPIRED","start_date":null,"expires":"2026-07-22T23:59:59+00:00","rewards":[]}
            ]"#,
        );
        let now = Utc.with_ymd_and_hms(2026, 7, 23, 0, 0, 0).unwrap();
        let codes = parse_html_at(&html, now).unwrap();

        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].code, "CURRENT");
    }

    #[test]
    fn handles_multiple_push_chunks_and_nested_strings() {
        let first = r#"<script>self.__next_f.push([1,"0:[{\"message\":\"ignore } ]\"}]")</script>"#;
        let second = html_with_codes(
            r#"[{"code":"FOUND","code_variants":null,"rewards":[{"item":"Credit","qty":5000}],"region_locked":"$undefined"}]"#,
        );
        let now = Utc.with_ymd_and_hms(2026, 7, 23, 0, 0, 0).unwrap();
        let codes = parse_html_at(&format!("{first}{second}"), now).unwrap();

        assert_eq!(codes[0].code, "FOUND");
        assert_eq!(codes[0].rewards, vec!["Credit ×5000"]);
    }

    #[test]
    fn rejects_missing_payload() {
        let error = extract_initial_codes("<html></html>").unwrap_err();
        assert!(error.to_string().contains("initialCodes"));
    }
}
