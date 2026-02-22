use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::Response;

use crate::games::starrail;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};
use crate::http::routes::json_response;

use super::{fetch_fandom_images, random_r};

const DS_SALT: &str = "6s25p5ox5y14umn1p61aqyyvbvvl3lrt";

fn generate_ds() -> String {
    let t = chrono::Utc::now().timestamp();
    let r = random_r();
    let raw = format!("salt={DS_SALT}&t={t}&r={r}");
    let hash = format!("{:x}", md5::compute(raw.as_bytes()));
    format!("{t},{r},{hash}")
}

#[derive(serde::Deserialize)]
struct HyvResponse {
    retcode: i32,
    message: String,
    data: Option<HyvCalendarData>,
}

#[derive(serde::Deserialize)]
struct HyvCalendarData {
    avatar_card_pool_list: Vec<HyvAvatarPool>,
    equip_card_pool_list: Vec<HyvEquipPool>,
    act_list: Vec<HyvActivity>,
    challenge_list: Vec<HyvChallenge>,
}

#[derive(serde::Deserialize)]
struct HyvTimeInfo {
    start_ts: String,
    end_ts: String,
}

#[derive(serde::Deserialize)]
struct HyvAvatarPool {
    id: String,
    name: String,
    version: String,
    time_info: HyvTimeInfo,
    avatar_list: Vec<HyvCharacter>,
}

#[derive(serde::Deserialize)]
struct HyvEquipPool {
    id: String,
    name: String,
    version: String,
    time_info: HyvTimeInfo,
    equip_list: Vec<HyvLightCone>,
}

#[derive(serde::Deserialize)]
struct HyvCharacter {
    item_id: String,
    item_name: String,
    rarity: String,
    damage_type: String,
    avatar_base_type: String,
    icon_url: String,
}

#[derive(serde::Deserialize)]
struct HyvLightCone {
    item_id: String,
    item_name: String,
    rarity: String,
    avatar_base_type: String,
    item_url: String,
}

#[derive(serde::Deserialize)]
struct HyvActivity {
    id: u64,
    name: String,
    panel_desc: String,
    act_type: String,
    reward_list: Vec<HyvReward>,
    special_reward: Option<HyvReward>,
    time_info: HyvTimeInfo,
}

#[derive(serde::Deserialize)]
struct HyvChallenge {
    group_id: u64,
    name_mi18n: String,
    challenge_type: String,
    reward_list: Vec<HyvReward>,
    special_reward: Option<HyvReward>,
    time_info: HyvTimeInfo,
}

#[derive(serde::Deserialize)]
struct HyvReward {
    item_id: u64,
    name: String,
    icon: String,
    rarity: String,
    num: u64,
}

#[derive(serde::Serialize)]
struct CalendarResponse {
    events: Vec<Event>,
    banners: Vec<Banner>,
    challenges: Vec<Challenge>,
}

#[derive(serde::Serialize)]
struct Reward {
    id: u64,
    name: String,
    icon: String,
    rarity: String,
    amount: u64,
}

#[derive(serde::Serialize)]
struct Event {
    id: u64,
    name: String,
    description: String,
    image_url: Option<String>,
    type_name: String,
    start_time: i64,
    end_time: i64,
    rewards: Vec<Reward>,
    special_reward: Option<Reward>,
}

#[derive(serde::Serialize)]
struct Banner {
    id: u64,
    name: String,
    version: String,
    characters: Vec<Character>,
    light_cones: Vec<LightCone>,
    start_time: i64,
    end_time: i64,
}

#[derive(serde::Serialize)]
struct Character {
    id: u64,
    name: String,
    icon: String,
    element: String,
    path: String,
    rarity: u8,
}

#[derive(serde::Serialize)]
struct LightCone {
    id: u64,
    name: String,
    icon: String,
    path: String,
    rarity: u8,
}

#[derive(serde::Serialize)]
struct Challenge {
    id: u64,
    name: String,
    type_name: String,
    start_time: i64,
    end_time: i64,
    rewards: Vec<Reward>,
    special_reward: Option<Reward>,
}

fn map_reward(r: HyvReward) -> Reward {
    Reward {
        id: r.item_id,
        name: r.name,
        icon: r.icon,
        rarity: r.rarity,
        amount: r.num,
    }
}

fn map_special_reward(r: Option<HyvReward>) -> Option<Reward> {
    r.filter(|r| r.item_id != 0).map(map_reward)
}

fn transform_event(act: HyvActivity, image_url: Option<String>) -> Event {
    Event {
        id: act.id,
        name: act.name,
        description: act.panel_desc,
        image_url,
        type_name: act.act_type,
        start_time: act.time_info.start_ts.parse().unwrap_or(0),
        end_time: act.time_info.end_ts.parse().unwrap_or(0),
        rewards: act.reward_list.into_iter().map(map_reward).collect(),
        special_reward: map_special_reward(act.special_reward),
    }
}

fn transform_challenge(challenge: HyvChallenge) -> Challenge {
    Challenge {
        id: challenge.group_id,
        name: challenge.name_mi18n,
        type_name: challenge.challenge_type,
        start_time: challenge.time_info.start_ts.parse().unwrap_or(0),
        end_time: challenge.time_info.end_ts.parse().unwrap_or(0),
        rewards: challenge.reward_list.into_iter().map(map_reward).collect(),
        special_reward: map_special_reward(challenge.special_reward),
    }
}

fn transform_avatar_banner(pool: HyvAvatarPool) -> Banner {
    Banner {
        id: pool.id.parse().unwrap_or(0),
        name: pool.name,
        version: pool.version,
        characters: pool
            .avatar_list
            .into_iter()
            .map(|c| Character {
                id: c.item_id.parse().unwrap_or(0),
                name: c.item_name,
                icon: c.icon_url,
                element: c.damage_type,
                path: c.avatar_base_type,
                rarity: c.rarity.parse().unwrap_or(0),
            })
            .collect(),
        light_cones: Vec::new(),
        start_time: pool.time_info.start_ts.parse().unwrap_or(0),
        end_time: pool.time_info.end_ts.parse().unwrap_or(0),
    }
}

fn transform_equip_banner(pool: HyvEquipPool) -> Banner {
    Banner {
        id: pool.id.parse().unwrap_or(0),
        name: pool.name,
        version: pool.version,
        characters: Vec::new(),
        light_cones: pool
            .equip_list
            .into_iter()
            .map(|c| LightCone {
                id: c.item_id.parse().unwrap_or(0),
                name: c.item_name,
                icon: c.item_url,
                path: c.avatar_base_type,
                rarity: c.rarity.parse().unwrap_or(0),
            })
            .collect(),
        start_time: pool.time_info.start_ts.parse().unwrap_or(0),
        end_time: pool.time_info.end_ts.parse().unwrap_or(0),
    }
}

fn transform_calendar(
    data: HyvCalendarData,
    image_map: &HashMap<String, String>,
) -> CalendarResponse {
    let banners = data
        .avatar_card_pool_list
        .into_iter()
        .map(transform_avatar_banner)
        .chain(
            data.equip_card_pool_list
                .into_iter()
                .map(transform_equip_banner),
        )
        .collect();

    let events = data
        .act_list
        .into_iter()
        .filter(|e| e.time_info.start_ts != "0" && e.time_info.end_ts != "0")
        .map(|act| {
            let image_url = image_map.get(&act.name).cloned();
            transform_event(act, image_url)
        })
        .collect();

    let challenges = data
        .challenge_list
        .into_iter()
        .map(transform_challenge)
        .collect();

    CalendarResponse {
        events,
        banners,
        challenges,
    }
}

/// GET /starrail/calendar
///
/// Returns current events, banners, and challenges for Honkai: Star Rail.
#[tracing::instrument(skip(global))]
pub(super) async fn get_starrail_calendar(
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    const CACHE_KEY: &str = "/mihoyo/starrail/calendar";

    if let Some(bytes) = global.response_cache.get(CACHE_KEY).await {
        return Ok(json_response(bytes));
    }

    let game_config = global
        .config
        .validator
        .game_config(crate::games::Game::Starrail)
        .filter(|c| !c.cookie.is_empty() && !c.uid.is_empty())
        .ok_or_else(|| {
            ApiError::internal_server_error(
                ApiErrorCode::NOT_CONFIGURED,
                "starrail calendar credentials not configured",
            )
        })?;

    let ds = generate_ds();

    let resp = global
        .http_client
        .get(starrail::CALENDAR_API)
        .query(&[
            ("server", &game_config.region),
            ("role_id", &game_config.uid),
        ])
        .header("Cookie", &game_config.cookie)
        .header("DS", ds)
        .header("x-rpc-app_version", "1.5.0")
        .header("x-rpc-client_type", "5")
        .header("x-rpc-language", "en-us")
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch starrail calendar");
            ApiError::internal_server_error(
                ApiErrorCode::UPSTREAM_ERROR,
                "failed to fetch calendar",
            )
        })?;

    let hyv_resp: HyvResponse = resp.json().await.map_err(|e| {
        tracing::error!(error = %e, "failed to parse starrail calendar response");
        ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "failed to parse calendar response",
        )
    })?;

    if hyv_resp.retcode != 0 {
        tracing::error!(retcode = hyv_resp.retcode, message = %hyv_resp.message, "hoyoverse starrail calendar API error");
        return Err(ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "calendar API returned an error",
        ));
    }

    let data = hyv_resp.data.ok_or_else(|| {
        ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "calendar API returned no data",
        )
    })?;

    const FANDOM_CACHE_KEY: &str = "/fandom/starrail/event-images";
    let image_map: HashMap<String, String> =
        if let Some(bytes) = global.fandom_image_cache.get(FANDOM_CACHE_KEY).await {
            serde_json::from_slice(&bytes).unwrap_or_default()
        } else {
            let names: Vec<String> = data
                .act_list
                .iter()
                .filter(|a| a.time_info.start_ts != "0" && a.time_info.end_ts != "0")
                .map(|a| a.name.clone())
                .collect();
            let map = fetch_fandom_images(
                &global.http_client,
                "https://honkai-star-rail.fandom.com/api.php",
                "File:Event ",
                ".png",
                &names,
            )
            .await;
            let bytes = Bytes::from(serde_json::to_vec(&map).unwrap_or_default());
            global
                .fandom_image_cache
                .insert(FANDOM_CACHE_KEY.to_string(), bytes)
                .await;
            map
        };

    let calendar = transform_calendar(data, &image_map);
    let bytes = Bytes::from(
        serde_json::to_vec(&calendar).expect("CalendarResponse is always serializable"),
    );
    global
        .response_cache
        .insert(CACHE_KEY.to_string(), bytes.clone())
        .await;

    Ok(json_response(bytes))
}
