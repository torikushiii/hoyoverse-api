use std::collections::HashMap;
use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::global::Global;

mod genshin;
mod starrail;

pub fn routes() -> Router<Arc<Global>> {
    Router::new()
        .route("/genshin/calendar", get(genshin::get_genshin_calendar))
        .route("/starrail/calendar", get(starrail::get_starrail_calendar))
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

async fn fetch_fandom_images(
    client: &reqwest::Client,
    api_url: &str,
    file_prefix: &str,
    file_suffix: &str,
    names: &[String],
) -> HashMap<String, String> {
    if names.is_empty() {
        return HashMap::new();
    }

    let titles: String = names
        .iter()
        .map(|n| format!("{file_prefix}{n}{file_suffix}"))
        .collect::<Vec<_>>()
        .join("|");

    let result = client
        .get(api_url)
        .query(&[
            ("action", "query"),
            ("prop", "imageinfo"),
            ("iiprop", "url"),
            ("format", "json"),
            ("titles", &titles),
        ])
        .send()
        .await;

    let resp = match result {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "failed to fetch fandom images");
            return HashMap::new();
        }
    };

    let fandom_resp: FandomResponse = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "failed to parse fandom image response");
            return HashMap::new();
        }
    };

    let query = match fandom_resp.query {
        Some(q) => q,
        None => return HashMap::new(),
    };

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
    map
}
