use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::Response;
use axum::routing::get;
use axum::Router;

use crate::games::Game;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};
use crate::http::routes::json_response;
use serde::Deserialize;

const PAGE_SIZE: u32 = 15;

fn deserialize_string_to_i64<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<i64>().map_err(serde::de::Error::custom)
}

const EVENTS_API: &str =
    "https://bbs-api-os.hoyolab.com/community/community_contribution/wapi/event/list";
const NEWS_API: &str = "https://bbs-api-os.hoyolab.com/community/post/wapi/getNewsList";

pub fn routes() -> Router<Arc<Global>> {
    Router::new()
        .route("/:game/news/events", get(get_events))
        .route("/:game/news/notices", get(get_notices))
        .route("/:game/news/info", get(get_info))
}

#[derive(serde::Deserialize)]
struct HylEventResponse {
    retcode: i32,
    message: String,
    data: Option<HylEventData>,
}

#[derive(serde::Deserialize)]
struct HylEventData {
    list: Vec<HylEvent>,
}

#[derive(serde::Deserialize)]
struct HylEvent {
    id: String,
    name: String,
    desc: String,
    #[serde(deserialize_with = "deserialize_string_to_i64")]
    create_at: i64,
    banner_url: String,
    web_path: String,
}

#[derive(serde::Deserialize)]
struct HylNewsResponse {
    retcode: i32,
    message: String,
    data: Option<HylNewsData>,
}

#[derive(serde::Deserialize)]
struct HylNewsData {
    list: Vec<HylNewsItem>,
}

#[derive(serde::Deserialize)]
struct HylNewsItem {
    post: HylPost,
    image_list: Vec<HylImage>,
}

#[derive(serde::Deserialize)]
struct HylPost {
    post_id: String,
    subject: String,
    desc: String,
    created_at: i64,
}

#[derive(serde::Deserialize)]
struct HylImage {
    url: String,
}

#[derive(serde::Serialize)]
struct NewsItem {
    id: String,
    title: String,
    description: String,
    created_at: i64,
    banner: Option<String>,
    url: String,
    #[serde(rename = "type")]
    type_name: &'static str,
}

async fn fetch_events(client: &reqwest::Client, gid: u32) -> Result<Vec<NewsItem>, ApiError> {
    let resp = client
        .get(EVENTS_API)
        .query(&[
            ("page_size", PAGE_SIZE.to_string()),
            ("size", PAGE_SIZE.to_string()),
            ("gids", gid.to_string()),
            ("is_all", "1".to_string()),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, gid, "failed to fetch hoyolab events");
            ApiError::internal_server_error(ApiErrorCode::UPSTREAM_ERROR, "failed to fetch events")
        })?;

    let hyl: HylEventResponse = resp.json().await.map_err(|e| {
        tracing::error!(error = %e, gid, "failed to parse hoyolab events response");
        ApiError::internal_server_error(ApiErrorCode::UPSTREAM_ERROR, "failed to parse events")
    })?;

    if hyl.retcode != 0 {
        tracing::error!(retcode = hyl.retcode, message = %hyl.message, gid, "hoyolab events API error");
        return Err(ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "events API returned an error",
        ));
    }

    let items = hyl
        .data
        .ok_or_else(|| {
            ApiError::internal_server_error(
                ApiErrorCode::UPSTREAM_ERROR,
                "events API returned no data",
            )
        })?
        .list
        .into_iter()
        .map(|e| NewsItem {
            id: e.id,
            title: e.name,
            description: e.desc,
            created_at: e.create_at,
            banner: Some(e.banner_url).filter(|s| !s.is_empty()),
            url: format!("https://www.hoyolab.com{}", e.web_path),
            type_name: "event",
        })
        .collect();

    Ok(items)
}

async fn fetch_news(
    client: &reqwest::Client,
    gid: u32,
    news_type: u8,
    type_name: &'static str,
) -> Result<Vec<NewsItem>, ApiError> {
    let resp = client
        .get(NEWS_API)
        .query(&[
            ("gids", gid.to_string()),
            ("page_size", PAGE_SIZE.to_string()),
            ("type", news_type.to_string()),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, gid, news_type, "failed to fetch hoyolab news");
            ApiError::internal_server_error(ApiErrorCode::UPSTREAM_ERROR, "failed to fetch news")
        })?;

    let hyl: HylNewsResponse = resp.json().await.map_err(|e| {
        tracing::error!(error = %e, gid, news_type, "failed to parse hoyolab news response");
        ApiError::internal_server_error(ApiErrorCode::UPSTREAM_ERROR, "failed to parse news")
    })?;

    if hyl.retcode != 0 {
        tracing::error!(retcode = hyl.retcode, message = %hyl.message, gid, news_type, "hoyolab news API error");
        return Err(ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "news API returned an error",
        ));
    }

    let items = hyl
        .data
        .ok_or_else(|| {
            ApiError::internal_server_error(
                ApiErrorCode::UPSTREAM_ERROR,
                "news API returned no data",
            )
        })?
        .list
        .into_iter()
        .map(|item| {
            let banner = item.image_list.into_iter().next().map(|img| img.url);
            let url = format!("https://www.hoyolab.com/article/{}", item.post.post_id);
            NewsItem {
                id: item.post.post_id,
                title: item.post.subject,
                description: item.post.desc,
                created_at: item.post.created_at,
                banner,
                url,
                type_name,
            }
        })
        .collect();

    Ok(items)
}

fn resolve_game(slug: &str) -> Result<Game, ApiError> {
    Game::from_slug(slug)
        .ok_or_else(|| ApiError::not_found(ApiErrorCode::ROUTE_NOT_FOUND, "unknown game"))
}

fn cache_response(items: &[NewsItem]) -> Bytes {
    Bytes::from(serde_json::to_vec(items).expect("Vec<NewsItem> is always serializable"))
}

#[tracing::instrument(skip(global))]
async fn get_events(
    Path(game): Path<String>,
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    let game = resolve_game(&game)?;
    let cache_key = format!("/hoyolab/{}/news/events", game.slug());

    if let Some(bytes) = global.news_cache.get(&cache_key).await {
        return Ok(json_response(bytes));
    }

    let items = fetch_events(&global.http_client, game.hoyolab_gid()).await?;
    let bytes = cache_response(&items);
    global.news_cache.insert(cache_key, bytes.clone()).await;

    Ok(json_response(bytes))
}

#[tracing::instrument(skip(global))]
async fn get_notices(
    Path(game): Path<String>,
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    let game = resolve_game(&game)?;
    let cache_key = format!("/hoyolab/{}/news/notices", game.slug());

    if let Some(bytes) = global.news_cache.get(&cache_key).await {
        return Ok(json_response(bytes));
    }

    let items = fetch_news(&global.http_client, game.hoyolab_gid(), 1, "notice").await?;
    let bytes = cache_response(&items);
    global.news_cache.insert(cache_key, bytes.clone()).await;

    Ok(json_response(bytes))
}

#[tracing::instrument(skip(global))]
async fn get_info(
    Path(game): Path<String>,
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    let game = resolve_game(&game)?;
    let cache_key = format!("/hoyolab/{}/news/info", game.slug());

    if let Some(bytes) = global.news_cache.get(&cache_key).await {
        return Ok(json_response(bytes));
    }

    let items = fetch_news(&global.http_client, game.hoyolab_gid(), 3, "info").await?;
    let bytes = cache_response(&items);
    global.news_cache.insert(cache_key, bytes.clone()).await;

    Ok(json_response(bytes))
}
