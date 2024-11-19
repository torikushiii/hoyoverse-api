use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use tracing::{debug, error};
use serde_json::json;

use crate::{
    types::{
        GenshinCalendarResponse,
        calendar::{CalendarResponse, Event, Banner, Challenge, Reward},
    },
    config::Settings,
    utils::generate_ds::generate_ds,
};

const CALENDAR_URL: &str = "https://sg-public-api.hoyolab.com/event/game_record/genshin/api/act_calendar";

pub async fn fetch_calendar(config: &Settings) -> Result<CalendarResponse> {
    debug!("Fetching Genshin calendar data");

    let account = config.game_accounts.genshin.first()
        .ok_or_else(|| anyhow::anyhow!("No Genshin account configured"))?;

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

    let body = json!({
        "server": account.region,
        "role_id": account.uid,
    });

    let response = client
        .post(CALENDAR_URL)
        .json(&body)
        .send()
        .await?;

    let calendar: GenshinCalendarResponse = response.json().await?;

    if calendar.retcode != 0 {
        error!("Failed to fetch calendar data: {}", calendar.message);
        anyhow::bail!("API error: {}", calendar.message);
    }

    let data = calendar.data.ok_or_else(|| anyhow::anyhow!("No calendar data"))?;

    // Transform banners
    let mut banners = Vec::new();
    for pool in data.avatar_card_pool_list.into_iter()
        .chain(data.weapon_card_pool_list)
        .chain(data.mixed_card_pool_list) {
            let start_time = pool.start_timestamp.parse::<i64>()?;
            let end_time = pool.end_timestamp.parse::<i64>()?;

            banners.push(Banner {
                id: pool.id,
                name: pool.name,
                version: pool.version,
                characters: Vec::new(), // Genshin API doesn't provide character details for now
                start_time,
                end_time,
            });
    }

    // Transform events (act_list takes priority)
    let mut events = Vec::new();
    let mut seen_event_names = std::collections::HashSet::new();

    // Process act_list first (primary events)
    for event in data.act_list {
        let start_time = event.start_timestamp.parse::<i64>()?;
        let end_time = event.end_timestamp.parse::<i64>()?;

        seen_event_names.insert(event.name.clone());
        events.push(Event {
            id: event.id,
            name: event.name,
            description: event.desc,
            type_name: event.event_type,
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
            special_reward: None,
        });
    }

    // Add selected_act_list events that aren't already included
    for event in data.selected_act_list {
        if !seen_event_names.contains(&event.name) {
            let start_time = event.start_timestamp.parse::<i64>()?;
            let end_time = event.end_timestamp.parse::<i64>()?;

            events.push(Event {
                id: event.id,
                name: event.name,
                description: event.desc,
                type_name: event.event_type,
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
                special_reward: None,
            });
        }
    }

    // Transform fixed_act_list to challenges
    let mut challenges = Vec::new();
    for challenge in data.fixed_act_list {
        let start_time = challenge.start_timestamp.parse::<i64>()?;
        let end_time = challenge.end_timestamp.parse::<i64>()?;

        challenges.push(Challenge {
            id: challenge.id,
            name: challenge.name,
            type_name: challenge.event_type,
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
            special_reward: None,
        });
    }

    Ok(CalendarResponse {
        events,
        banners,
        challenges,
    })
}