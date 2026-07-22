use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::Response;

use crate::games::starrail;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};
use crate::http::routes::json_response;

use super::{LangQuery, cookie_with_lang, random_r, resolve_lang, try_fetch_fandom_images};

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

fn eligible_activity_ids(data: &HyvCalendarData) -> Vec<u64> {
    data.act_list
        .iter()
        .filter(|activity| activity.time_info.start_ts != "0" && activity.time_info.end_ts != "0")
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
    format!("/fandom/starrail/event-images/{ids}")
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
            let image_url = image_map.get(&act.id).cloned();
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

fn calendar_request(
    client: &reqwest::Client,
    uid: &str,
    region: &str,
    cookie: &str,
    lang: &str,
) -> reqwest::RequestBuilder {
    client
        .get(starrail::CALENDAR_API)
        .query(&[("server", region), ("role_id", uid)])
        .header("Cookie", cookie)
        .header("DS", generate_ds())
        .header("x-rpc-app_version", "1.5.0")
        .header("x-rpc-client_type", "5")
        .header("x-rpc-language", lang)
}

async fn fetch_calendar_data(
    request: reqwest::RequestBuilder,
    request_lang: &str,
) -> Result<HyvCalendarData, ApiError> {
    let response = request.send().await.map_err(|error| {
        tracing::error!(error = %error, request_lang, "failed to fetch starrail calendar");
        ApiError::internal_server_error(ApiErrorCode::UPSTREAM_ERROR, "failed to fetch calendar")
    })?;
    let response: HyvResponse = response.json().await.map_err(|error| {
        tracing::error!(error = %error, request_lang, "failed to parse starrail calendar response");
        ApiError::internal_server_error(
            ApiErrorCode::UPSTREAM_ERROR,
            "failed to parse calendar response",
        )
    })?;

    if response.retcode != 0 {
        tracing::error!(retcode = response.retcode, message = %response.message, request_lang, "hoyoverse starrail calendar API error");
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

/// GET /starrail/calendar
///
/// Returns current events, banners, and challenges for Honkai: Star Rail.
#[tracing::instrument(skip(global))]
pub(super) async fn get_starrail_calendar(
    Query(query): Query<LangQuery>,
    State(global): State<Arc<Global>>,
) -> Result<Response<Body>, ApiError> {
    let lang = resolve_lang(query.lang)?;
    let cache_key = format!("/mihoyo/starrail/calendar/{lang}");

    let bytes = global
        .response_cache
        .get_or_try_insert(cache_key, async {
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
                        tracing::warn!(error = ?error, "failed to fetch canonical starrail activity names");
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
                            "https://honkai-star-rail.fandom.com/api.php",
                            "File:Event ",
                            ".png",
                            &names,
                        )
                        .await
                        .map_err(|error| {
                            tracing::warn!(error = %error, "failed to fetch starrail fandom images");
                            ApiError::internal_server_error(
                                ApiErrorCode::UPSTREAM_ERROR,
                                "failed to fetch fandom images",
                            )
                        })?;
                        let bytes = serde_json::to_vec(&map).map_err(|error| {
                            tracing::warn!(error = %error, "failed to serialize starrail fandom images");
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
                        tracing::warn!(error = ?error, "starrail fandom images unavailable");
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

    fn activity(id: u64, name: &str, start_ts: &str, end_ts: &str) -> HyvActivity {
        HyvActivity {
            id,
            name: name.to_string(),
            panel_desc: String::new(),
            act_type: "Event".to_string(),
            reward_list: Vec::new(),
            special_reward: None,
            time_info: HyvTimeInfo {
                start_ts: start_ts.to_string(),
                end_ts: end_ts.to_string(),
            },
        }
    }

    fn calendar_data(activities: Vec<HyvActivity>) -> HyvCalendarData {
        HyvCalendarData {
            avatar_card_pool_list: Vec::new(),
            equip_card_pool_list: Vec::new(),
            act_list: activities,
            challenge_list: Vec::new(),
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
            activity(42, "ローカライズイベント", "1", "2"),
            image_map.get(&42).cloned(),
        );

        assert_eq!(event.name, "ローカライズイベント");
        assert_eq!(
            event.image_url.as_deref(),
            Some("https://example.com/event.png")
        );
    }

    #[test]
    fn excludes_zero_timestamp_activities() {
        let data = calendar_data(vec![
            activity(1, "Playable", "1", "2"),
            activity(2, "No Start", "0", "2"),
            activity(3, "No End", "1", "0"),
        ]);

        assert_eq!(eligible_activity_ids(&data), vec![1]);
    }

    #[test]
    fn fandom_cache_key_is_order_independent() {
        let first = canonical_activities(
            &calendar_data(vec![
                activity(20, "Second", "1", "2"),
                activity(10, "First", "1", "2"),
            ]),
            &[20, 10],
        );
        let reordered = canonical_activities(
            &calendar_data(vec![
                activity(10, "First", "1", "2"),
                activity(20, "Second", "1", "2"),
            ]),
            &[10, 20],
        );
        let changed = canonical_activities(
            &calendar_data(vec![
                activity(10, "First", "1", "2"),
                activity(20, "Second", "1", "2"),
                activity(30, "Third", "1", "2"),
            ]),
            &[10, 20, 30],
        );

        assert_eq!(fandom_cache_key(&first), fandom_cache_key(&reordered));
        assert_ne!(fandom_cache_key(&first), fandom_cache_key(&changed));
    }
}
