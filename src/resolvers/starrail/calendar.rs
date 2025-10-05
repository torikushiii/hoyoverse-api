use anyhow::Result;
use mongodb::bson::doc;
use regex::escape;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use tracing::{debug, error};

use crate::{
    config::Settings,
    db::MongoConnection,
    types::{
        calendar::{
            Challenge, Character, Event, LightCone, Reward, StarRailBanner,
            StarRailCalendarResponse,
        },
        starrail::{GameChallenge, GameEvent},
        StarRailCalendarResponse as ApiResponse,
    },
    utils::generate_ds::generate_ds,
};

const CALENDAR_URL: &str =
    "https://sg-public-api.hoyolab.com/event/game_record/hkrpg/api/get_act_calender";

async fn get_event_image(mongo: &MongoConnection, event_name: &str) -> Option<String> {
    let events = mongo.collection::<mongodb::bson::Document>("events");

    if let Ok(Some(event)) = events
        .find_one(doc! {
            "name": {
                "$regex": format!(".*{}.*", escape(event_name)),
                "$options": "i"
            },
            "game": "starrail"
        })
        .await
    {
        event.get_str("imageUrl").ok().map(String::from)
    } else {
        None
    }
}

pub async fn fetch_calendar(
    config: &Settings,
    mongo: &MongoConnection,
) -> Result<StarRailCalendarResponse> {
    debug!("Fetching StarRail calendar data");

    let account = config
        .game_accounts
        .starrail
        .first()
        .ok_or_else(|| anyhow::anyhow!("No StarRail account configured"))?;

    debug!(
        server = %account.region,
        role_id = %account.uid,
        "Preparing StarRail calendar request"
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&config.server.user_agent)?,
    );
    headers.insert("DS", HeaderValue::from_str(&generate_ds())?);
    headers.insert("x-rpc-app_version", HeaderValue::from_static("1.5.0"));
    headers.insert("x-rpc-client_type", HeaderValue::from_static("5"));
    headers.insert("x-rpc-language", HeaderValue::from_static("en-us"));

    headers.insert(
        "Cookie",
        HeaderValue::from_str(&format!(
            "cookie_token_v2={}; account_mid_v2={}; account_id_v2={}",
            account.cookie_token_v2, account.account_mid_v2, account.account_id_v2,
        ))?,
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let query_params = [
        ("server", account.region.as_str()),
        ("role_id", account.uid.as_str()),
    ];

    let response = client.get(CALENDAR_URL).query(&query_params).send().await?;

    let status = response.status();
    let response_text = response.text().await?;

    debug!(
        status = %status,
        "Received StarRail calendar response"
    );

    if !status.is_success() {
        error!(
            status = %status,
            body = %response_text,
            "StarRail calendar request returned non-success status"
        );
        anyhow::bail!(
            "Calendar request failed with status {}: {}",
            status,
            response_text
        );
    }

    let calendar: ApiResponse = serde_json::from_str(&response_text).map_err(|err| {
        error!(
            status = %status,
            body = %response_text,
            error = %err,
            "Failed to decode StarRail calendar response body"
        );
        anyhow::Error::from(err)
    })?;

    if calendar.retcode != 0 {
        error!(
            status = %status,
            retcode = calendar.retcode,
            message = %calendar.message,
            body = %response_text,
            "StarRail calendar API returned error retcode"
        );
        anyhow::bail!("API error: {}", calendar.message);
    }

    let data = calendar
        .data
        .ok_or_else(|| anyhow::anyhow!("No calendar data"))?;

    let mut banners = Vec::new();

    for pool in data.avatar_card_pool_list {
        let start_time = pool.time_info.start_ts.parse::<i64>()?;
        let end_time = pool.time_info.end_ts.parse::<i64>()?;

        let characters = pool
            .avatar_list
            .into_iter()
            .map(|char| Character {
                id: char.item_id,
                name: char.item_name,
                rarity: char.rarity,
                element: char.damage_type,
                path: Some(char.avatar_base_type),
                icon: char.icon_url,
            })
            .collect();

        banners.push(StarRailBanner {
            id: pool.id,
            name: pool.name,
            version: pool.version,
            characters,
            light_cones: Vec::new(),
            start_time,
            end_time,
        });
    }

    for pool in data.equip_card_pool_list {
        let start_time = pool.time_info.start_ts.parse::<i64>()?;
        let end_time = pool.time_info.end_ts.parse::<i64>()?;

        let light_cones = pool
            .equip_list
            .into_iter()
            .map(|cone| LightCone {
                id: cone.item_id,
                name: cone.item_name,
                rarity: cone.rarity,
                path: cone.avatar_base_type,
                icon: cone.icon_url,
            })
            .collect();

        banners.push(StarRailBanner {
            id: pool.id,
            name: pool.name,
            version: pool.version,
            characters: Vec::new(),
            light_cones,
            start_time,
            end_time,
        });
    }

    let mut events = Vec::new();
    for event in data.act_list {
        let GameEvent {
            id,
            name,
            panel_desc,
            act_type,
            reward_list,
            special_reward,
            time_info,
            ..
        } = event;

        if time_info.start_ts == "0" || time_info.end_ts == "0" {
            continue;
        }

        let start_time = time_info.start_ts.parse::<i64>()?;
        let end_time = time_info.end_ts.parse::<i64>()?;

        events.push(Event {
            id,
            name: name.clone(),
            description: panel_desc,
            image_url: get_event_image(mongo, &name).await,
            type_name: act_type,
            start_time,
            end_time,
            rewards: reward_list
                .into_iter()
                .map(|reward| Reward {
                    id: reward.item_id,
                    name: reward.name,
                    icon: reward.icon,
                    rarity: reward.rarity,
                    amount: reward.num,
                })
                .collect(),
            special_reward: special_reward.and_then(|reward| {
                if reward.item_id != 0 {
                    Some(Reward {
                        id: reward.item_id,
                        name: reward.name,
                        icon: reward.icon,
                        rarity: reward.rarity,
                        amount: reward.num,
                    })
                } else {
                    None
                }
            }),
        });
    }

    let mut challenges = Vec::new();
    for challenge in data.challenge_list {
        let GameChallenge {
            group_id,
            name_mi18n,
            challenge_type,
            reward_list,
            special_reward,
            time_info,
            ..
        } = challenge;

        let start_time = time_info.start_ts.parse::<i64>()?;
        let end_time = time_info.end_ts.parse::<i64>()?;

        challenges.push(Challenge {
            id: group_id,
            name: name_mi18n,
            type_name: challenge_type,
            start_time,
            end_time,
            rewards: reward_list
                .into_iter()
                .map(|reward| Reward {
                    id: reward.item_id,
                    name: reward.name,
                    icon: reward.icon,
                    rarity: reward.rarity,
                    amount: reward.num,
                })
                .collect(),
            special_reward: special_reward.and_then(|reward| {
                if reward.item_id != 0 {
                    Some(Reward {
                        id: reward.item_id,
                        name: reward.name,
                        icon: reward.icon,
                        rarity: reward.rarity,
                        amount: reward.num,
                    })
                } else {
                    None
                }
            }),
        });
    }

    debug!(
        events = events.len(),
        banners = banners.len(),
        challenges = challenges.len(),
        "StarRail calendar data parsed successfully"
    );

    Ok(StarRailCalendarResponse {
        events,
        banners,
        challenges,
    })
}
