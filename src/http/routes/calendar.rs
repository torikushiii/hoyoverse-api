use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::Response;
use axum::routing::get;
use axum::Router;

use crate::games::{genshin, starrail};
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};
use crate::http::routes::json_response;

pub fn routes() -> Router<Arc<Global>> {
    Router::new()
        .route("/genshin/calendar", get(get_genshin_calendar))
        .route("/starrail/calendar", get(get_starrail_calendar))
}

const DS_SALT_GENSHIN: &str = "xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs";
const DS_SALT_STARRAIL: &str = "6s25p5ox5y14umn1p61aqyyvbvvl3lrt";

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

fn generate_ds(body: &str) -> String {
    let t = chrono::Utc::now().timestamp();
    let r = random_r();
    let raw = format!("salt={DS_SALT_GENSHIN}&t={t}&r={r}&b={body}&q=");
    let hash = format!("{:x}", md5::compute(raw.as_bytes()));
    format!("{t},{r},{hash}")
}

fn generate_ds_starrail() -> String {
    let t = chrono::Utc::now().timestamp();
    let r = random_r();
    let raw = format!("salt={DS_SALT_STARRAIL}&t={t}&r={r}");
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
    act_list: Vec<HyvActivity>,
    fixed_act_list: Vec<HyvActivity>,
    avatar_card_pool_list: Vec<HyvBannerPool>,
    weapon_card_pool_list: Vec<HyvBannerPool>,
    mixed_card_pool_list: Vec<HyvBannerPool>,
}

#[derive(serde::Deserialize)]
struct HyvActivity {
    id: u64,
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    desc: String,
    start_timestamp: String,
    end_timestamp: String,
    reward_list: Vec<HyvReward>,
}

#[derive(serde::Deserialize)]
struct HyvReward {
    item_id: u64,
    name: String,
    icon: String,
    rarity: String,
    num: u64,
    homepage_show: bool,
}

#[derive(serde::Deserialize)]
struct HyvBannerPool {
    pool_id: u64,
    pool_name: String,
    version_name: String,
    avatars: Vec<HyvAvatar>,
    weapon: Vec<HyvWeapon>,
    start_timestamp: String,
    end_timestamp: String,
}

#[derive(serde::Deserialize)]
struct HyvAvatar {
    id: u64,
    name: String,
    icon: String,
    element: String,
    rarity: u8,
}

#[derive(serde::Deserialize)]
struct HyvWeapon {
    id: u64,
    name: String,
    icon: String,
    rarity: u8,
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
    weapons: Vec<Weapon>,
    start_time: i64,
    end_time: i64,
}

#[derive(serde::Serialize)]
struct Character {
    id: u64,
    name: String,
    icon: String,
    element: String,
    rarity: u8,
}

#[derive(serde::Serialize)]
struct Weapon {
    id: u64,
    name: String,
    icon: String,
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

fn map_rewards(reward_list: Vec<HyvReward>) -> (Vec<Reward>, Option<Reward>) {
    let mut special_reward = None;
    let mut rewards = Vec::new();

    for r in reward_list {
        let reward = Reward {
            id: r.item_id,
            name: r.name,
            icon: r.icon,
            rarity: r.rarity,
            amount: r.num,
        };
        if r.homepage_show {
            special_reward = Some(reward);
        } else {
            rewards.push(reward);
        }
    }

    (rewards, special_reward)
}

fn transform_event(act: HyvActivity) -> Event {
    let start_time = act.start_timestamp.parse().unwrap_or(0);
    let end_time = act.end_timestamp.parse().unwrap_or(0);
    let (rewards, special_reward) = map_rewards(act.reward_list);

    Event {
        id: act.id,
        name: act.name,
        description: act.desc,
        image_url: None,
        type_name: act.type_name,
        start_time,
        end_time,
        rewards,
        special_reward,
    }
}

fn transform_challenge(act: HyvActivity) -> Challenge {
    let start_time = act.start_timestamp.parse().unwrap_or(0);
    let end_time = act.end_timestamp.parse().unwrap_or(0);
    let (rewards, special_reward) = map_rewards(act.reward_list);

    Challenge {
        id: act.id,
        name: act.name,
        type_name: act.type_name,
        start_time,
        end_time,
        rewards,
        special_reward,
    }
}

fn transform_banner(pool: HyvBannerPool) -> Banner {
    Banner {
        id: pool.pool_id,
        name: pool.pool_name,
        version: pool.version_name,
        characters: pool
            .avatars
            .into_iter()
            .map(|a| Character {
                id: a.id,
                name: a.name,
                icon: a.icon,
                element: a.element,
                rarity: a.rarity,
            })
            .collect(),
        weapons: pool
            .weapon
            .into_iter()
            .map(|w| Weapon {
                id: w.id,
                name: w.name,
                icon: w.icon,
                rarity: w.rarity,
            })
            .collect(),
        start_time: pool.start_timestamp.parse().unwrap_or(0),
        end_time: pool.end_timestamp.parse().unwrap_or(0),
    }
}

fn transform_calendar(data: HyvCalendarData) -> CalendarResponse {
    let events = data.act_list.into_iter().map(transform_event).collect();

    let challenges = data
        .fixed_act_list
        .into_iter()
        .map(transform_challenge)
        .collect();

    let banners = data
        .avatar_card_pool_list
        .into_iter()
        .chain(data.weapon_card_pool_list)
        .chain(data.mixed_card_pool_list)
        .map(transform_banner)
        .collect();

    CalendarResponse {
        events,
        banners,
        challenges,
    }
}

/// GET /mihoyo/genshin/calendar
///
/// Returns current events, banners, and challenges for Genshin Impact.
#[tracing::instrument(skip(global))]
async fn get_genshin_calendar(
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    const CACHE_KEY: &str = "/mihoyo/genshin/calendar";

    if let Some(bytes) = global.response_cache.get(CACHE_KEY).await {
        return Ok(json_response(bytes));
    }

    let game_config = global
        .config
        .validator
        .game_config(crate::games::Game::Genshin)
        .filter(|c| !c.cookie.is_empty() && !c.uid.is_empty())
        .ok_or_else(|| {
            ApiError::internal_server_error(
                ApiErrorCode::NOT_CONFIGURED,
                "genshin calendar credentials not configured",
            )
        })?;

    let body = serde_json::json!({
        "role_id": game_config.uid,
        "server": game_config.region,
    });
    let body_str = body.to_string();
    let ds = generate_ds(&body_str);

    let resp = global
        .http_client
        .post(genshin::CALENDAR_API)
        .header("Cookie", &game_config.cookie)
        .header("DS", ds)
        .header("x-rpc-app_version", "1.5.0")
        .header("x-rpc-client_type", "5")
        .header("x-rpc-language", "en-us")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch genshin calendar");
            ApiError::internal_server_error(
                ApiErrorCode::UPSTREAM_ERROR,
                "failed to fetch calendar",
            )
        })?;

    let hyv_resp: HyvResponse = resp.json().await.map_err(|e| {
        tracing::error!(error = %e, "failed to parse genshin calendar response");
        ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "failed to parse calendar response",
        )
    })?;

    if hyv_resp.retcode != 0 {
        tracing::error!(retcode = hyv_resp.retcode, message = %hyv_resp.message, "hoyoverse calendar API error");
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

    let calendar = transform_calendar(data);
    let bytes = Bytes::from(
        serde_json::to_vec(&calendar).expect("CalendarResponse is always serializable"),
    );
    global
        .response_cache
        .insert(CACHE_KEY.to_string(), bytes.clone())
        .await;

    Ok(json_response(bytes))
}

#[derive(serde::Deserialize)]
struct HyvSRResponse {
    retcode: i32,
    message: String,
    data: Option<HyvSRCalendarData>,
}

#[derive(serde::Deserialize)]
struct HyvSRCalendarData {
    avatar_card_pool_list: Vec<HyvAvatarPool>,
    equip_card_pool_list: Vec<HyvEquipPool>,
    act_list: Vec<HyvSRActivity>,
    challenge_list: Vec<HyvSRChallenge>,
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
    avatar_list: Vec<HyvSRCharacter>,
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
struct HyvSRCharacter {
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
struct HyvSRActivity {
    id: u64,
    name: String,
    panel_desc: String,
    act_type: String,
    reward_list: Vec<HyvSRReward>,
    special_reward: Option<HyvSRReward>,
    time_info: HyvTimeInfo,
}

#[derive(serde::Deserialize)]
struct HyvSRChallenge {
    group_id: u64,
    name_mi18n: String,
    challenge_type: String,
    reward_list: Vec<HyvSRReward>,
    special_reward: Option<HyvSRReward>,
    time_info: HyvTimeInfo,
}

#[derive(serde::Deserialize)]
struct HyvSRReward {
    item_id: u64,
    name: String,
    icon: String,
    rarity: String,
    num: u64,
}

#[derive(serde::Serialize)]
struct StarRailCalendarResponse {
    events: Vec<SREvent>,
    banners: Vec<SRBanner>,
    challenges: Vec<SRChallenge>,
}

#[derive(serde::Serialize)]
struct SRReward {
    id: u64,
    name: String,
    icon: String,
    rarity: String,
    amount: u64,
}

#[derive(serde::Serialize)]
struct SREvent {
    id: u64,
    name: String,
    description: String,
    image_url: Option<String>,
    type_name: String,
    start_time: i64,
    end_time: i64,
    rewards: Vec<SRReward>,
    special_reward: Option<SRReward>,
}

#[derive(serde::Serialize)]
struct SRBanner {
    id: u64,
    name: String,
    version: String,
    characters: Vec<SRCharacter>,
    light_cones: Vec<SRLightCone>,
    start_time: i64,
    end_time: i64,
}

#[derive(serde::Serialize)]
struct SRCharacter {
    id: u64,
    name: String,
    icon: String,
    element: String,
    path: String,
    rarity: u8,
}

#[derive(serde::Serialize)]
struct SRLightCone {
    id: u64,
    name: String,
    icon: String,
    path: String,
    rarity: u8,
}

#[derive(serde::Serialize)]
struct SRChallenge {
    id: u64,
    name: String,
    type_name: String,
    start_time: i64,
    end_time: i64,
    rewards: Vec<SRReward>,
    special_reward: Option<SRReward>,
}

fn map_sr_reward(r: HyvSRReward) -> SRReward {
    SRReward {
        id: r.item_id,
        name: r.name,
        icon: r.icon,
        rarity: r.rarity,
        amount: r.num,
    }
}

fn map_sr_special_reward(r: Option<HyvSRReward>) -> Option<SRReward> {
    r.filter(|r| r.item_id != 0).map(map_sr_reward)
}

fn transform_sr_event(act: HyvSRActivity) -> SREvent {
    SREvent {
        id: act.id,
        name: act.name,
        description: act.panel_desc,
        image_url: None,
        type_name: act.act_type,
        start_time: act.time_info.start_ts.parse().unwrap_or(0),
        end_time: act.time_info.end_ts.parse().unwrap_or(0),
        rewards: act.reward_list.into_iter().map(map_sr_reward).collect(),
        special_reward: map_sr_special_reward(act.special_reward),
    }
}

fn transform_sr_challenge(challenge: HyvSRChallenge) -> SRChallenge {
    SRChallenge {
        id: challenge.group_id,
        name: challenge.name_mi18n,
        type_name: challenge.challenge_type,
        start_time: challenge.time_info.start_ts.parse().unwrap_or(0),
        end_time: challenge.time_info.end_ts.parse().unwrap_or(0),
        rewards: challenge
            .reward_list
            .into_iter()
            .map(map_sr_reward)
            .collect(),
        special_reward: map_sr_special_reward(challenge.special_reward),
    }
}

fn transform_sr_avatar_banner(pool: HyvAvatarPool) -> SRBanner {
    SRBanner {
        id: pool.id.parse().unwrap_or(0),
        name: pool.name,
        version: pool.version,
        characters: pool
            .avatar_list
            .into_iter()
            .map(|c| SRCharacter {
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

fn transform_sr_equip_banner(pool: HyvEquipPool) -> SRBanner {
    SRBanner {
        id: pool.id.parse().unwrap_or(0),
        name: pool.name,
        version: pool.version,
        characters: Vec::new(),
        light_cones: pool
            .equip_list
            .into_iter()
            .map(|c| SRLightCone {
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

fn transform_sr_calendar(data: HyvSRCalendarData) -> StarRailCalendarResponse {
    let banners = data
        .avatar_card_pool_list
        .into_iter()
        .map(transform_sr_avatar_banner)
        .chain(
            data.equip_card_pool_list
                .into_iter()
                .map(transform_sr_equip_banner),
        )
        .collect();

    let events = data
        .act_list
        .into_iter()
        .filter(|e| e.time_info.start_ts != "0" && e.time_info.end_ts != "0")
        .map(transform_sr_event)
        .collect();

    let challenges = data
        .challenge_list
        .into_iter()
        .map(transform_sr_challenge)
        .collect();

    StarRailCalendarResponse {
        events,
        banners,
        challenges,
    }
}

/// GET /mihoyo/starrail/calendar
///
/// Returns current events, banners, and challenges for Honkai: Star Rail.
#[tracing::instrument(skip(global))]
async fn get_starrail_calendar(
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

    let ds = generate_ds_starrail();

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

    let hyv_resp: HyvSRResponse = resp.json().await.map_err(|e| {
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

    let calendar = transform_sr_calendar(data);
    let bytes = Bytes::from(
        serde_json::to_vec(&calendar).expect("StarRailCalendarResponse is always serializable"),
    );
    global
        .response_cache
        .insert(CACHE_KEY.to_string(), bytes.clone())
        .await;

    Ok(json_response(bytes))
}
