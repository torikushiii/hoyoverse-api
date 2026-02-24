use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::Response;

use crate::games::genshin;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};
use crate::http::routes::json_response;

use super::{fetch_fandom_images, random_r};

const DS_SALT: &str = "xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs";

fn generate_ds(body: &str) -> String {
    let t = chrono::Utc::now().timestamp();
    let r = random_r();
    let raw = format!("salt={DS_SALT}&t={t}&r={r}&b={body}&q=");
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

fn transform_event(act: HyvActivity, image_url: Option<String>) -> Event {
    let start_time = act.start_timestamp.parse().unwrap_or(0);
    let end_time = act.end_timestamp.parse().unwrap_or(0);
    let (rewards, special_reward) = map_rewards(act.reward_list);

    Event {
        id: act.id,
        name: act.name,
        description: act.desc,
        image_url,
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

fn transform_calendar(
    data: HyvCalendarData,
    image_map: &HashMap<String, String>,
) -> CalendarResponse {
    let events = data
        .act_list
        .into_iter()
        .map(|act| {
            let image_url = image_map.get(&act.name).cloned();
            transform_event(act, image_url)
        })
        .collect();

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

/// GET /genshin/calendar
///
/// Returns current events, banners, and challenges for Genshin Impact.
#[tracing::instrument(skip(global))]
pub(super) async fn get_genshin_calendar(
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    const CACHE_KEY: &str = "/mihoyo/genshin/calendar";

    let bytes = global
        .response_cache
        .get_or_try_insert(CACHE_KEY.to_string(), async {
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

            const FANDOM_CACHE_KEY: &str = "/fandom/genshin/event-images";
            const SKIP_TYPES: &[&str] = &["Test Run", "In-Person", "Web"];
            let names: Vec<String> = data
                .act_list
                .iter()
                .filter(|a| !SKIP_TYPES.contains(&a.type_name.as_str()))
                .map(|a| a.name.clone())
                .collect();

            let fandom_bytes = global
                .fandom_image_cache
                .get_or_insert(FANDOM_CACHE_KEY.to_string(), async {
                    let map = fetch_fandom_images(
                        &global.http_client,
                        "https://genshin-impact.fandom.com/api.php",
                        "File:",
                        ".png",
                        &names,
                    )
                    .await;
                    Bytes::from(serde_json::to_vec(&map).unwrap_or_default())
                })
                .await;

            let image_map: HashMap<String, String> =
                serde_json::from_slice(&fandom_bytes).unwrap_or_default();

            let calendar = transform_calendar(data, &image_map);
            Ok(Bytes::from(
                serde_json::to_vec(&calendar)
                    .expect("CalendarResponse is always serializable"),
            ))
        })
        .await?;

    Ok(json_response(bytes))
}
