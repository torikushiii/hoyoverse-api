use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::Response;

use crate::games::zenless;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};
use crate::http::routes::json_response;

use super::{LangQuery, cookie_with_lang, fetch_fandom_images, resolve_lang};

#[derive(serde::Deserialize)]
struct HyvActivityResponse {
    retcode: i32,
    message: String,
    data: Option<HyvActivityData>,
}

#[derive(serde::Deserialize)]
struct HyvActivityData {
    activity_list: Vec<HyvActivity>,
}

#[derive(serde::Deserialize)]
struct HyvActivity {
    activity_id: u64,
    state: String,
    name: String,
    monochrome_cnt: u64,
    start_ts: i64,
    end_ts: i64,
}

#[derive(serde::Deserialize)]
struct HyvGachaResponse {
    retcode: i32,
    message: String,
    data: Option<HyvGachaData>,
}

#[derive(serde::Deserialize)]
struct HyvGachaData {
    avatar_gacha_schedule_list: Vec<HyvAvatarGacha>,
    weapon_gacha_schedule_list: Vec<HyvWeaponGacha>,
}

#[derive(serde::Deserialize)]
struct HyvAvatarGacha {
    gacha_type: String,
    gacha_state: String,
    start_ts: i64,
    end_ts: i64,
    version: String,
    avatar_list: Vec<HyvAvatar>,
}

#[derive(serde::Deserialize)]
struct HyvWeaponGacha {
    gacha_type: String,
    gacha_state: String,
    start_ts: i64,
    end_ts: i64,
    version: String,
    weapon_list: Vec<HyvWeapon>,
}

#[derive(serde::Deserialize)]
struct HyvAvatar {
    avatar_id: u64,
    avatar_name: String,
    full_name: String,
    rarity: String,
    icon: String,
    avatar_profession: u8,
    avatar_element_type: u16,
}

#[derive(serde::Deserialize)]
struct HyvWeapon {
    weapon_id: u64,
    rarity: String,
    icon: String,
    talent_title: String,
    profession: u8,
}

#[derive(serde::Serialize)]
struct CalendarResponse {
    events: Vec<Event>,
    banners: Vec<Banner>,
    challenges: Vec<Challenge>,
}

#[derive(serde::Serialize)]
struct Event {
    id: u64,
    name: String,
    state: String,
    image_url: Option<String>,
    start_time: i64,
    end_time: i64,
    polychrome: u64,
}

#[derive(serde::Serialize)]
struct Banner {
    banner_type: String,
    state: String,
    version: String,
    agents: Vec<Agent>,
    w_engines: Vec<WEngine>,
    start_time: i64,
    end_time: i64,
}

#[derive(serde::Serialize)]
struct Agent {
    id: u64,
    name: String,
    full_name: String,
    icon: String,
    rarity: String,
    profession: String,
    element: String,
}

#[derive(serde::Serialize)]
struct WEngine {
    id: u64,
    name: String,
    icon: String,
    rarity: String,
    profession: String,
}

#[derive(serde::Serialize)]
struct Challenge {}

fn map_profession(id: u8) -> String {
    match id {
        1 => "Attack",
        2 => "Stun",
        3 => "Anomaly",
        4 => "Support",
        5 => "Defence",
        6 => "Rupture",
        _ => "unknown",
    }
    .to_string()
}

fn map_element(id: u16) -> String {
    match id {
        200 => "Physical",
        201 => "Fire",
        202 => "Ice",
        203 => "Electric",
        204 => "Wind",
        205 => "Ether",
        _ => "unknown",
    }
    .to_string()
}

fn transform_activity(act: HyvActivity, image_url: Option<String>) -> Event {
    Event {
        id: act.activity_id,
        name: act.name,
        state: act.state,
        image_url,
        start_time: act.start_ts,
        end_time: act.end_ts,
        polychrome: act.monochrome_cnt,
    }
}

fn transform_avatar_banner(pool: HyvAvatarGacha) -> Banner {
    Banner {
        banner_type: pool.gacha_type,
        state: pool.gacha_state,
        version: pool.version,
        agents: pool
            .avatar_list
            .into_iter()
            .map(|a| Agent {
                id: a.avatar_id,
                name: a.avatar_name,
                full_name: a.full_name,
                icon: a.icon,
                rarity: a.rarity,
                profession: map_profession(a.avatar_profession),
                element: map_element(a.avatar_element_type),
            })
            .collect(),
        w_engines: Vec::new(),
        start_time: pool.start_ts,
        end_time: pool.end_ts,
    }
}

fn transform_weapon_banner(pool: HyvWeaponGacha) -> Banner {
    Banner {
        banner_type: pool.gacha_type,
        state: pool.gacha_state,
        version: pool.version,
        agents: Vec::new(),
        w_engines: pool
            .weapon_list
            .into_iter()
            .map(|w| WEngine {
                id: w.weapon_id,
                name: w.talent_title,
                icon: w.icon,
                rarity: w.rarity,
                profession: map_profession(w.profession),
            })
            .collect(),
        start_time: pool.start_ts,
        end_time: pool.end_ts,
    }
}

fn transform_calendar(
    activity_data: HyvActivityData,
    gacha_data: HyvGachaData,
    image_map: &HashMap<String, String>,
) -> CalendarResponse {
    let events = activity_data
        .activity_list
        .into_iter()
        .map(|act| {
            let image_url = image_map.get(&act.name).cloned();
            transform_activity(act, image_url)
        })
        .collect();

    let banners = gacha_data
        .avatar_gacha_schedule_list
        .into_iter()
        .map(transform_avatar_banner)
        .chain(
            gacha_data
                .weapon_gacha_schedule_list
                .into_iter()
                .map(transform_weapon_banner),
        )
        .collect();

    CalendarResponse {
        events,
        banners,
        challenges: Vec::new(),
    }
}

/// GET /zenless/calendar
///
/// Returns current events and banners for Zenless Zone Zero.
#[tracing::instrument(skip(global))]
pub(super) async fn get_zenless_calendar(
    Query(query): Query<LangQuery>,
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    let lang = resolve_lang(query.lang)?;
    let cache_key = format!("/mihoyo/zenless/calendar/{lang}");

    let bytes = global
        .response_cache
        .get_or_try_insert(cache_key, async {
            let game_config = global
                .config
                .validator
                .game_config(crate::games::Game::Zenless)
                .filter(|c| !c.cookie.is_empty() && !c.uid.is_empty())
                .ok_or_else(|| {
                    ApiError::internal_server_error(
                        ApiErrorCode::NOT_CONFIGURED,
                        "zenless calendar credentials not configured",
                    )
                })?;

            let cookie = cookie_with_lang(&game_config.cookie, lang);

            let activity_resp = global
                .http_client
                .get(zenless::ACTIVITY_CALENDAR_API)
                .query(&[
                    ("uid", game_config.uid.as_str()),
                    ("region", game_config.region.as_str()),
                    ("lang", lang),
                ])
                .header("Cookie", cookie.clone())
                .header("x-rpc-language", lang)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "failed to fetch zenless activity calendar");
                    ApiError::internal_server_error(
                        ApiErrorCode::UPSTREAM_ERROR,
                        "failed to fetch calendar",
                    )
                })?;

            let activity_resp: HyvActivityResponse = activity_resp.json().await.map_err(|e| {
                tracing::error!(error = %e, "failed to parse zenless activity calendar response");
                ApiError::internal_server_error(
                    ApiErrorCode::UPSTREAM_ERROR,
                    "failed to parse calendar response",
                )
            })?;

            if activity_resp.retcode != 0 {
                tracing::error!(retcode = activity_resp.retcode, message = %activity_resp.message, "hoyoverse zenless activity calendar API error");
                return Err(ApiError::internal_server_error(
                    ApiErrorCode::UPSTREAM_ERROR,
                    "calendar API returned an error",
                ));
            }

            let activity_data = activity_resp.data.ok_or_else(|| {
                ApiError::internal_server_error(
                    ApiErrorCode::UPSTREAM_ERROR,
                    "calendar API returned no data",
                )
            })?;

            let gacha_resp = global
                .http_client
                .get(zenless::GACHA_CALENDAR_API)
                .query(&[
                    ("uid", game_config.uid.as_str()),
                    ("region", game_config.region.as_str()),
                    ("lang", lang),
                ])
                .header("Cookie", cookie)
                .header("x-rpc-language", lang)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "failed to fetch zenless gacha calendar");
                    ApiError::internal_server_error(
                        ApiErrorCode::UPSTREAM_ERROR,
                        "failed to fetch calendar",
                    )
                })?;

            let gacha_resp: HyvGachaResponse = gacha_resp.json().await.map_err(|e| {
                tracing::error!(error = %e, "failed to parse zenless gacha calendar response");
                ApiError::internal_server_error(
                    ApiErrorCode::UPSTREAM_ERROR,
                    "failed to parse calendar response",
                )
            })?;

            if gacha_resp.retcode != 0 {
                tracing::error!(retcode = gacha_resp.retcode, message = %gacha_resp.message, "hoyoverse zenless gacha calendar API error");
                return Err(ApiError::internal_server_error(
                    ApiErrorCode::UPSTREAM_ERROR,
                    "calendar API returned an error",
                ));
            }

            let gacha_data = gacha_resp.data.ok_or_else(|| {
                ApiError::internal_server_error(
                    ApiErrorCode::UPSTREAM_ERROR,
                    "calendar API returned no data",
                )
            })?;

            const FANDOM_CACHE_KEY: &str = "/fandom/zenless/event-images";
            let names: Vec<String> = activity_data
                .activity_list
                .iter()
                .map(|a| a.name.clone())
                .collect();

            let fandom_bytes = global
                .fandom_image_cache
                .get_or_insert(FANDOM_CACHE_KEY.to_string(), async {
                    let map = fetch_fandom_images(
                        &global.http_client,
                        "https://zenless-zone-zero.fandom.com/api.php",
                        "File:Event ",
                        ".png",
                        &names,
                    )
                    .await;
                    Bytes::from(serde_json::to_vec(&map).unwrap_or_default())
                })
                .await;

            let image_map: HashMap<String, String> =
                serde_json::from_slice(&fandom_bytes).unwrap_or_default();

            let calendar = transform_calendar(activity_data, gacha_data, &image_map);
            Ok(Bytes::from(
                serde_json::to_vec(&calendar).expect("CalendarResponse is always serializable"),
            ))
        })
        .await?;

    Ok(json_response(bytes))
}
