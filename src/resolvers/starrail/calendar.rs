use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use tracing::{debug, error};

use crate::{
    types::{
        StarRailCalendarResponse,
        calendar::{CalendarResponse, Event, Banner, Character, Challenge, Reward},
    },
    config::Settings,
    utils::generate_ds::generate_ds,
};

const CALENDAR_URL: &str = "https://sg-public-api.hoyolab.com/event/game_record/hkrpg/api/get_act_calender";

pub async fn fetch_calendar(config: &Settings) -> Result<CalendarResponse> {
    debug!("Fetching StarRail calendar data");

    let account = config.game_accounts.starrail.first()
        .ok_or_else(|| anyhow::anyhow!("No StarRail account configured"))?;

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_str(&config.server.user_agent)?);
    headers.insert("DS", HeaderValue::from_str(&generate_ds())?);
    headers.insert("x-rpc-app_version", HeaderValue::from_static("1.5.0"));
    headers.insert("x-rpc-client_type", HeaderValue::from_static("5"));
    headers.insert("x-rpc-language", HeaderValue::from_static("en-us"));

    headers.insert(
        "Cookie",
        HeaderValue::from_str(&format!(
            "cookie_token_v2={}; account_mid_v2={}; account_id_v2={}",
            account.cookie_token_v2,
            account.account_mid_v2,
            account.account_id_v2,
        ))?
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client
        .get(CALENDAR_URL)
        .query(&[
            ("server", &account.region),
            ("role_id", &account.uid),
        ])
        .send()
        .await?;

    let calendar: StarRailCalendarResponse = response.json().await?;

    if calendar.retcode != 0 {
        error!("Failed to fetch calendar data: {}", calendar.message);
        anyhow::bail!("API error: {}", calendar.message);
    }

    let data = calendar.data.ok_or_else(|| anyhow::anyhow!("No calendar data"))?;

    let mut banners = Vec::new();
    for pool in data.avatar_card_pool_list {
        let start_time = pool.time_info.start_ts.parse::<i64>()?;
        let end_time = pool.time_info.end_ts.parse::<i64>()?;

        banners.push(Banner {
            id: pool.id,
            name: pool.name,
            version: pool.version,
            characters: pool.avatar_list.into_iter()
                .map(|char| Character {
                    id: char.item_id,
                    name: char.item_name,
                    rarity: char.rarity,
                    element: char.damage_type_name,
                    path: char.avatar_base_type,
                    icon: char.icon_url,
                })
                .collect(),
            start_time,
            end_time,
        });
    }

    let mut events = Vec::new();
    for event in data.act_list {
        if event.time_info.start_ts == "0" || event.time_info.end_ts == "0" {
            continue;
        }

        let start_time = event.time_info.start_ts.parse::<i64>()?;
        let end_time = event.time_info.end_ts.parse::<i64>()?;

        events.push(Event {
            id: event.id,
            name: event.name,
            description: event.panel_desc,
            type_name: event.act_type,
            start_time,
            end_time,
            rewards: event.reward_list.into_iter()
                .map(|reward| Reward {
                    id: reward.item_id,
                    name: reward.name,
                    icon: reward.icon,
                    rarity: reward.rarity,
                    amount: reward.num,
                })
                .collect(),
            special_reward: if event.special_reward.item_id != 0 {
                Some(Reward {
                    id: event.special_reward.item_id,
                    name: event.special_reward.name,
                    icon: event.special_reward.icon,
                    rarity: event.special_reward.rarity,
                    amount: event.special_reward.num,
                })
            } else {
                None
            },
        });
    }

    let mut challenges = Vec::new();
    for challenge in data.challenge_list {
        let start_time = challenge.time_info.start_ts.parse::<i64>()?;
        let end_time = challenge.time_info.end_ts.parse::<i64>()?;

        challenges.push(Challenge {
            id: challenge.group_id,
            name: challenge.name_mi18n,
            type_name: challenge.challenge_type,
            start_time,
            end_time,
            rewards: challenge.reward_list.into_iter()
                .map(|reward| Reward {
                    id: reward.item_id,
                    name: reward.name,
                    icon: reward.icon,
                    rarity: reward.rarity,
                    amount: reward.num,
                })
                .collect(),
            special_reward: if challenge.special_reward.item_id != 0 {
                Some(Reward {
                    id: challenge.special_reward.item_id,
                    name: challenge.special_reward.name,
                    icon: challenge.special_reward.icon,
                    rarity: challenge.special_reward.rarity,
                    amount: challenge.special_reward.num,
                })
            } else {
                None
            },
        });
    }

    Ok(CalendarResponse {
        events,
        banners,
        challenges,
    })
}