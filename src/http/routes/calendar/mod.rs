use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context as _;
use axum::Router;
use axum::routing::get;

use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};

mod genshin;
mod starrail;
mod zenless;

pub fn routes() -> Router<Arc<Global>> {
    Router::new()
        .route("/genshin/calendar", get(genshin::get_genshin_calendar))
        .route("/starrail/calendar", get(starrail::get_starrail_calendar))
        .route("/zenless/calendar", get(zenless::get_zenless_calendar))
}

fn random_r() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    (0..6)
        .map(|i| CHARSET[(nanos.wrapping_add(i * 7919)) % CHARSET.len()] as char)
        .collect()
}

const DEFAULT_LANG: &str = "en-us";

const SUPPORTED_LANGS: &[&str] = &[
    "en-us", "zh-cn", "zh-tw", "de-de", "es-es", "fr-fr", "id-id", "it-it", "ja-jp", "ko-kr",
    "pt-pt", "ru-ru", "th-th", "tr-tr", "vi-vn",
];

const LANG_ALIASES: &[(&str, &str)] = &[
    ("en", "en-us"),
    ("us", "en-us"),
    ("zh", "zh-cn"),
    ("cn", "zh-cn"),
    ("tw", "zh-tw"),
    ("de", "de-de"),
    ("es", "es-es"),
    ("fr", "fr-fr"),
    ("id", "id-id"),
    ("it", "it-it"),
    ("ja", "ja-jp"),
    ("jp", "ja-jp"),
    ("ko", "ko-kr"),
    ("kr", "ko-kr"),
    ("pt", "pt-pt"),
    ("ru", "ru-ru"),
    ("th", "th-th"),
    ("tr", "tr-tr"),
    ("vi", "vi-vn"),
];

#[derive(Debug, serde::Deserialize)]
struct LangQuery {
    lang: Option<String>,
}

fn resolve_lang(lang: Option<String>) -> Result<&'static str, ApiError> {
    let lang = lang.as_deref().unwrap_or(DEFAULT_LANG);
    let lang = LANG_ALIASES
        .iter()
        .find(|(alias, _)| *alias == lang)
        .map(|(_, full)| *full)
        .unwrap_or(lang);
    SUPPORTED_LANGS
        .iter()
        .copied()
        .find(|&l| l == lang)
        .ok_or_else(|| {
            ApiError::bad_request(ApiErrorCode::INVALID_LANGUAGE, "unsupported language")
        })
}

fn cookie_with_lang(cookie: &str, lang: &str) -> String {
    let mut parts: Vec<String> = cookie
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty() && !part.starts_with("mi18nLang="))
        .map(ToOwned::to_owned)
        .collect();
    parts.insert(0, format!("mi18nLang={lang}"));
    parts.join("; ")
}

#[derive(serde::Deserialize)]
struct FandomResponse {
    query: Option<FandomQuery>,
}

#[derive(serde::Deserialize)]
struct FandomQuery {
    pages: HashMap<String, FandomPage>,
}

#[derive(serde::Deserialize)]
struct FandomPage {
    title: String,
    #[serde(default)]
    imageinfo: Vec<FandomImageInfo>,
}

#[derive(serde::Deserialize)]
struct FandomImageInfo {
    url: String,
}

async fn try_fetch_fandom_images(
    client: &reqwest::Client,
    api_url: &str,
    file_prefix: &str,
    file_suffix: &str,
    names: &[String],
) -> anyhow::Result<HashMap<String, String>> {
    if names.is_empty() {
        return Ok(HashMap::new());
    }

    let titles: String = names
        .iter()
        .map(|n| format!("{file_prefix}{n}{file_suffix}"))
        .collect::<Vec<_>>()
        .join("|");

    let resp = client
        .get(api_url)
        .query(&[
            ("action", "query"),
            ("prop", "imageinfo"),
            ("iiprop", "url"),
            ("format", "json"),
            ("titles", &titles),
        ])
        .send()
        .await
        .context("failed to fetch fandom images")?;

    let fandom_resp: FandomResponse = resp
        .json()
        .await
        .context("failed to parse fandom image response")?;
    let query = fandom_resp
        .query
        .context("fandom image response contained no query")?;

    let mut map = HashMap::new();
    for page in query.pages.values() {
        if let Some(info) = page.imageinfo.first() {
            let name = page
                .title
                .strip_prefix(file_prefix)
                .and_then(|s| s.strip_suffix(file_suffix))
                .unwrap_or(&page.title);
            map.insert(name.to_string(), info.url.clone());
        }
    }
    if map.is_empty() {
        anyhow::bail!("fandom image response contained no images");
    }
    Ok(map)
}

async fn fetch_fandom_images(
    client: &reqwest::Client,
    api_url: &str,
    file_prefix: &str,
    file_suffix: &str,
    names: &[String],
) -> HashMap<String, String> {
    match try_fetch_fandom_images(client, api_url, file_prefix, file_suffix, names).await {
        Ok(map) => map,
        Err(error) => {
            tracing::warn!(error = %error, "failed to fetch fandom images");
            HashMap::new()
        }
    }
}
