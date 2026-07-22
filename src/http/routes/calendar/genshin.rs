use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::Response;

use crate::games::genshin;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};
use crate::http::routes::json_response;

use super::{LangQuery, cookie_with_lang, random_r, resolve_lang, try_fetch_fandom_images};

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

const SKIP_TYPES: &[&str] = &["Test Run", "In-Person", "Web"];

fn eligible_activity_ids(data: &HyvCalendarData) -> Vec<u64> {
    data.act_list
        .iter()
        .filter(|activity| !SKIP_TYPES.contains(&activity.type_name.as_str()))
        .map(|activity| activity.id)
        .collect()
}

fn canonical_activities(data: &HyvCalendarData, eligible_ids: &[u64]) -> Vec<(u64, String)> {
    let mut activities: Vec<_> = data
        .act_list
        .iter()
        .filter(|activity| eligible_ids.contains(&activity.id))
        .map(|activity| (activity.id, activity.name.clone()))
        .collect();
    activities.sort_unstable_by_key(|(id, _)| *id);
    activities.dedup_by_key(|(id, _)| *id);
    activities
}

fn fandom_cache_key(activities: &[(u64, String)]) -> String {
    let ids = activities
        .iter()
        .map(|(id, _)| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("/fandom/genshin/event-images/{ids}")
}

fn map_activity_images(
    activities: &[(u64, String)],
    fandom_images: &HashMap<String, String>,
) -> HashMap<u64, String> {
    activities
        .iter()
        .filter_map(|(id, name)| {
            fandom_images
                .get(name)
                .cloned()
                .map(|image_url| (*id, image_url))
        })
        .collect()
}

fn transform_calendar(data: HyvCalendarData, image_map: &HashMap<u64, String>) -> CalendarResponse {
    let events = data
        .act_list
        .into_iter()
        .map(|act| {
            let image_url = image_map.get(&act.id).cloned();
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

fn calendar_request(
    client: &reqwest::Client,
    uid: &str,
    region: &str,
    cookie: &str,
    lang: &str,
) -> reqwest::RequestBuilder {
    let body = serde_json::json!({
        "role_id": uid,
        "server": region,
    });
    let body_str = body.to_string();

    client
        .post(genshin::CALENDAR_API)
        .header("Cookie", cookie)
        .header("DS", generate_ds(&body_str))
        .header("x-rpc-app_version", "1.5.0")
        .header("x-rpc-client_type", "5")
        .header("x-rpc-language", lang)
        .json(&body)
}

async fn fetch_calendar_data(
    request: reqwest::RequestBuilder,
    request_lang: &str,
) -> Result<HyvCalendarData, ApiError> {
    let response = request.send().await.map_err(|error| {
        tracing::error!(error = %error, request_lang, "failed to fetch genshin calendar");
        ApiError::internal_server_error(ApiErrorCode::UPSTREAM_ERROR, "failed to fetch calendar")
    })?;
    let response: HyvResponse = response.json().await.map_err(|error| {
        tracing::error!(error = %error, request_lang, "failed to parse genshin calendar response");
        ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "failed to parse calendar response",
        )
    })?;

    if response.retcode != 0 {
        tracing::error!(retcode = response.retcode, message = %response.message, request_lang, "hoyoverse genshin calendar API error");
        return Err(ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "calendar API returned an error",
        ));
    }

    response.data.ok_or_else(|| {
        ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "calendar API returned no data",
        )
    })
}

/// GET /genshin/calendar
///
/// Returns current events, banners, and challenges for Genshin Impact.
#[tracing::instrument(skip(global))]
pub(super) async fn get_genshin_calendar(
    Query(query): Query<LangQuery>,
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    let lang = resolve_lang(query.lang)?;
    let cache_key = format!("/mihoyo/genshin/calendar/{lang}");

    let bytes = global
        .response_cache
        .get_or_try_insert(cache_key, async {
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

            let localized_cookie = cookie_with_lang(&game_config.cookie, lang);
            let localized_request = calendar_request(
                &global.http_client,
                &game_config.uid,
                &game_config.region,
                &localized_cookie,
                lang,
            );
            let data = fetch_calendar_data(localized_request, lang).await?;
            let eligible_ids = eligible_activity_ids(&data);

            let canonical_activities = if lang == "en-us" {
                canonical_activities(&data, &eligible_ids)
            } else {
                let english_cookie = cookie_with_lang(&game_config.cookie, "en-us");
                let english_request = calendar_request(
                    &global.http_client,
                    &game_config.uid,
                    &game_config.region,
                    &english_cookie,
                    "en-us",
                );
                match fetch_calendar_data(english_request, "en-us").await {
                    Ok(english_data) => canonical_activities(&english_data, &eligible_ids),
                    Err(error) => {
                        tracing::warn!(error = ?error, "failed to fetch canonical genshin activity names");
                        Vec::new()
                    }
                }
            };

            let image_map = if canonical_activities.is_empty() {
                HashMap::new()
            } else {
                let names: Vec<String> = canonical_activities
                    .iter()
                    .map(|(_, name)| name.clone())
                    .collect();
                let cache_key = fandom_cache_key(&canonical_activities);
                let fandom_bytes = global
                    .fandom_image_cache
                    .get_or_try_insert(cache_key, async {
                        let map = try_fetch_fandom_images(
                            &global.http_client,
                            "https://genshin-impact.fandom.com/api.php",
                            "File:",
                            ".png",
                            &names,
                        )
                        .await
                        .map_err(|error| {
                            tracing::warn!(error = %error, "failed to fetch genshin fandom images");
                            ApiError::internal_server_error(
                                ApiErrorCode::UPSTREAM_ERROR,
                                "failed to fetch fandom images",
                            )
                        })?;
                        let bytes = serde_json::to_vec(&map).map_err(|error| {
                            tracing::warn!(error = %error, "failed to serialize genshin fandom images");
                            ApiError::internal_server_error(
                                ApiErrorCode::UPSTREAM_ERROR,
                                "failed to cache fandom images",
                            )
                        })?;
                        Ok(Bytes::from(bytes))
                    })
                    .await;
                let fandom_images: HashMap<String, String> = match fandom_bytes {
                    Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
                    Err(error) => {
                        tracing::warn!(error = ?error, "genshin fandom images unavailable");
                        HashMap::new()
                    }
                };
                map_activity_images(&canonical_activities, &fandom_images)
            };

            let calendar = transform_calendar(data, &image_map);
            Ok(Bytes::from(
                serde_json::to_vec(&calendar)
                    .expect("CalendarResponse is always serializable"),
            ))
        })
        .await?;

    Ok(json_response(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn activity(id: u64, name: &str, type_name: &str) -> HyvActivity {
        HyvActivity {
            id,
            name: name.to_string(),
            type_name: type_name.to_string(),
            desc: String::new(),
            start_timestamp: "1".to_string(),
            end_timestamp: "2".to_string(),
            reward_list: Vec::new(),
        }
    }

    fn calendar_data(activities: Vec<HyvActivity>) -> HyvCalendarData {
        HyvCalendarData {
            act_list: activities,
            fixed_act_list: Vec::new(),
            avatar_card_pool_list: Vec::new(),
            weapon_card_pool_list: Vec::new(),
            mixed_card_pool_list: Vec::new(),
        }
    }

    #[test]
    fn maps_canonical_image_to_localized_activity() {
        let canonical = vec![(42, "English Event".to_string())];
        let fandom_images = HashMap::from([(
            "English Event".to_string(),
            "https://example.com/event.png".to_string(),
        )]);
        let image_map = map_activity_images(&canonical, &fandom_images);
        let event = transform_event(
            activity(42, "ローカライズイベント", "Event"),
            image_map.get(&42).cloned(),
        );

        assert_eq!(event.name, "ローカライズイベント");
        assert_eq!(
            event.image_url.as_deref(),
            Some("https://example.com/event.png")
        );
    }

    #[test]
    fn excludes_non_event_activity_types() {
        let data = calendar_data(vec![
            activity(1, "Playable", "Event"),
            activity(2, "Test", "Test Run"),
            activity(3, "Web", "Web"),
        ]);

        assert_eq!(eligible_activity_ids(&data), vec![1]);
    }

    #[test]
    fn fandom_cache_key_is_order_independent() {
        let first = canonical_activities(
            &calendar_data(vec![
                activity(20, "Second", "Event"),
                activity(10, "First", "Event"),
            ]),
            &[20, 10],
        );
        let reordered = canonical_activities(
            &calendar_data(vec![
                activity(10, "First", "Event"),
                activity(20, "Second", "Event"),
            ]),
            &[10, 20],
        );
        let changed = canonical_activities(
            &calendar_data(vec![
                activity(10, "First", "Event"),
                activity(20, "Second", "Event"),
                activity(30, "Third", "Event"),
            ]),
            &[10, 20, 30],
        );

        assert_eq!(fandom_cache_key(&first), fandom_cache_key(&reordered));
        assert_ne!(fandom_cache_key(&first), fandom_cache_key(&changed));
    }
}
