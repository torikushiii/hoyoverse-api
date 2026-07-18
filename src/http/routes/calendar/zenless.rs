use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::Response;
use chrono::{FixedOffset, TimeZone, Utc};
use serde::de::DeserializeOwned;

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

#[derive(serde::Deserialize)]
struct HyvChallengeResponse<T> {
    retcode: i32,
    message: String,
    data: Option<T>,
}

#[derive(serde::Deserialize)]
struct HyvDateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

#[derive(serde::Deserialize)]
struct HyvChallengePeriod {
    start_time: Option<HyvDateTime>,
    end_time: Option<HyvDateTime>,
}

#[derive(serde::Deserialize)]
struct HyvThresholdData {
    void_front_battle_abstract_info_brief: Option<HyvChallengePeriod>,
}

#[derive(serde::Deserialize)]
struct HyvShiyuData {
    hadal_info_v2: Option<HyvShiyuPeriod>,
}

#[derive(serde::Deserialize)]
struct HyvShiyuPeriod {
    begin_time: Option<String>,
    end_time: Option<String>,
    hadal_begin_time: Option<HyvDateTime>,
    hadal_end_time: Option<HyvDateTime>,
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
struct Challenge {
    id: u64,
    name: String,
    type_name: String,
    start_time: Option<i64>,
    end_time: Option<i64>,
}

#[derive(Default)]
struct ChallengePeriod {
    start_time: Option<i64>,
    end_time: Option<i64>,
}

fn server_offset(region: &str) -> Option<FixedOffset> {
    match region {
        "prod_gf_us" => FixedOffset::west_opt(5 * 60 * 60),
        "prod_gf_eu" => FixedOffset::east_opt(60 * 60),
        "prod_gf_jp" | "prod_gf_sg" => FixedOffset::east_opt(8 * 60 * 60),
        _ => None,
    }
}

fn structured_timestamp(value: Option<&HyvDateTime>, region: &str) -> Option<i64> {
    let value = value?;
    let offset = server_offset(region)?;
    offset
        .with_ymd_and_hms(
            value.year,
            value.month,
            value.day,
            value.hour,
            value.minute,
            value.second,
        )
        .single()
        .map(|date| date.timestamp())
}

fn numeric_timestamp(value: Option<&str>) -> Option<i64> {
    value?.parse().ok().filter(|timestamp| *timestamp > 0)
}

fn normalize_period(start_time: Option<i64>, end_time: Option<i64>) -> ChallengePeriod {
    let end_time = match (start_time, end_time) {
        (Some(start), Some(end)) if end < start => None,
        (_, end) => end,
    };

    ChallengePeriod {
        start_time,
        end_time,
    }
}

fn structured_period(data: Option<HyvChallengePeriod>, region: &str) -> ChallengePeriod {
    let Some(data) = data else {
        return ChallengePeriod::default();
    };
    normalize_period(
        structured_timestamp(data.start_time.as_ref(), region),
        structured_timestamp(data.end_time.as_ref(), region),
    )
}

const SHIYU_PERIOD_SECONDS: i64 = 14 * 24 * 60 * 60;

fn hardcoded_shiyu_period(region: &str) -> ChallengePeriod {
    let anchor = HyvDateTime {
        year: 2026,
        month: 6,
        day: 26,
        hour: 4,
        minute: 0,
        second: 0,
    };
    let Some(anchor_time) = structured_timestamp(Some(&anchor), region) else {
        return ChallengePeriod::default();
    };
    let period_index = (Utc::now().timestamp() - anchor_time).div_euclid(SHIYU_PERIOD_SECONDS);
    let start_time = anchor_time + period_index * SHIYU_PERIOD_SECONDS;

    ChallengePeriod {
        start_time: Some(start_time),
        end_time: Some(start_time + SHIYU_PERIOD_SECONDS - 1),
    }
}

fn shiyu_period(data: Option<HyvShiyuData>, region: &str) -> ChallengePeriod {
    let Some(period) = data.and_then(|data| data.hadal_info_v2) else {
        return hardcoded_shiyu_period(region);
    };
    let start_time = numeric_timestamp(period.begin_time.as_deref())
        .or_else(|| structured_timestamp(period.hadal_begin_time.as_ref(), region));
    let end_time = numeric_timestamp(period.end_time.as_deref())
        .or_else(|| structured_timestamp(period.hadal_end_time.as_ref(), region));

    match normalize_period(start_time, end_time) {
        ChallengePeriod {
            start_time: None,
            end_time: None,
        } => hardcoded_shiyu_period(region),
        ChallengePeriod {
            start_time: Some(start_time),
            end_time: None,
        } => ChallengePeriod {
            start_time: Some(start_time),
            end_time: Some(start_time + SHIYU_PERIOD_SECONDS - 1),
        },
        ChallengePeriod {
            start_time: None,
            end_time: Some(end_time),
        } => ChallengePeriod {
            start_time: Some(end_time - SHIYU_PERIOD_SECONDS + 1),
            end_time: Some(end_time),
        },
        period => period,
    }
}

fn challenge_names(lang: &str) -> [&'static str; 4] {
    match lang {
        "zh-cn" => ["危局强袭战", "临界推演", "式舆防卫战", "拟境湮灭战"],
        "zh-tw" => ["危局強襲戰", "臨界推演", "式輿防衛戰", "擬境湮滅戰"],
        "de-de" => [
            "Gefährlicher Überfall",
            "Schwellensimulation",
            "Shiyu-Verteidigung",
            "Vernichtungs-Simulakrum",
        ],
        "es-es" => [
            "Incursión arriesgada",
            "Simulación de umbral",
            "Defensa shiyu",
            "Simulacro de aniquilación",
        ],
        "fr-fr" => [
            "Assaut mortel",
            "Simulation de seuil",
            "Défense de Shiyu",
            "Simulacre d'annihilation",
        ],
        "id-id" => [
            "Operasi Serbuan Maut",
            "Simulasi Ambang Batas",
            "Shiyu Defense",
            "Simulasi Pertempuran Pemusnahan",
        ],
        "ja-jp" => ["危局強襲戦", "臨界推演", "式輿防衛戦", "仮想殲滅作戦"],
        "ko-kr" => [
            "위험한 강습전",
            "임계 시뮬레이션",
            "시유 방어전",
            "모의 세계 섬멸전",
        ],
        "pt-pt" => [
            "Investida Mortal",
            "Simulação do Limiar",
            "Defesa Shiyu",
            "Simulacro da Aniquilação",
        ],
        "ru-ru" => [
            "Опасный штурм",
            "Крит. симуляция",
            "Оборона шиюй",
            "Симулякры и аннигиляция",
        ],
        "th-th" => [
            "ศึกวิกฤติ",
            "การจำลองจุดวิกฤต",
            "Shiyu Defense",
            "ศึกจำลองทำลายล้าง",
        ],
        "vi-vn" => [
            "Tập Kích Nguy Cấp",
            "Suy Đoán Chạm Ngưỡng",
            "Bảo Vệ Trụ Shiyu",
            "Chiến Hủy Diệt Giả Lập",
        ],
        _ => [
            "Deadly Assault",
            "Threshold Simulation",
            "Shiyu Defense",
            "Annihilation Simulacrum",
        ],
    }
}

fn transform_challenges(
    deadly: Option<HyvChallengePeriod>,
    threshold: Option<HyvThresholdData>,
    shiyu: Option<HyvShiyuData>,
    annihilation: Option<HyvChallengePeriod>,
    region: &str,
    lang: &str,
) -> Vec<Challenge> {
    let periods = [
        structured_period(deadly, region),
        structured_period(
            threshold.and_then(|data| data.void_front_battle_abstract_info_brief),
            region,
        ),
        shiyu_period(shiyu, region),
        structured_period(annihilation, region),
    ];
    let names = challenge_names(lang);
    let metadata = [
        (1, names[0], "deadly_assault"),
        (2, names[1], "threshold_simulation"),
        (3, names[2], "shiyu_defense"),
        (4, names[3], "annihilation_simulacrum"),
    ];

    metadata
        .into_iter()
        .zip(periods)
        .map(|((id, name, type_name), period)| Challenge {
            id,
            name: name.to_string(),
            type_name: type_name.to_string(),
            start_time: period.start_time,
            end_time: period.end_time,
        })
        .collect()
}

async fn fetch_challenge<T>(request: reqwest::RequestBuilder, challenge: &'static str) -> Option<T>
where
    T: DeserializeOwned,
{
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(error = %error, challenge, "failed to fetch zenless challenge");
            return None;
        }
    };
    let response: HyvChallengeResponse<T> = match response.json().await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(error = %error, challenge, "failed to parse zenless challenge response");
            return None;
        }
    };
    if response.retcode != 0 {
        tracing::error!(retcode = response.retcode, message = %response.message, challenge, "hoyoverse zenless challenge API error");
        return None;
    }
    if response.data.is_none() {
        tracing::error!(
            challenge,
            "hoyoverse zenless challenge API returned no data"
        );
    }
    response.data
}

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
    challenges: Vec<Challenge>,
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
        challenges,
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
                .header("Cookie", cookie.clone())
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

            let deadly_request = global
                .http_client
                .get(zenless::DEADLY_ASSAULT_API)
                .query(&[
                    ("uid", game_config.uid.as_str()),
                    ("region", game_config.region.as_str()),
                    ("schedule_type", "1"),
                    ("lang", lang),
                ])
                .header("Cookie", cookie.clone())
                .header("x-rpc-lang", lang)
                .header("x-rpc-language", lang);
            let threshold_request = global
                .http_client
                .get(zenless::THRESHOLD_SIMULATION_API)
                .query(&[
                    ("region", game_config.region.as_str()),
                    ("uid", game_config.uid.as_str()),
                    ("schedule_type", "1"),
                    ("lang", lang),
                ])
                .header("Cookie", cookie.clone())
                .header("x-rpc-lang", lang)
                .header("x-rpc-language", lang);
            let shiyu_request = global
                .http_client
                .get(zenless::SHIYU_DEFENSE_API)
                .query(&[
                    ("server", game_config.region.as_str()),
                    ("role_id", game_config.uid.as_str()),
                    ("schedule_type", "1"),
                    ("without_v2_detail", "true"),
                    ("lang", lang),
                ])
                .header("Cookie", cookie.clone())
                .header("x-rpc-lang", lang)
                .header("x-rpc-language", lang);
            let annihilation_request = global
                .http_client
                .get(zenless::ANNIHILATION_SIMULACRUM_API)
                .query(&[
                    ("region", game_config.region.as_str()),
                    ("uid", game_config.uid.as_str()),
                    ("schedule_type", "1"),
                    ("lang", lang),
                ])
                .header("Cookie", cookie)
                .header("x-rpc-lang", lang)
                .header("x-rpc-language", lang);

            let (deadly, threshold, shiyu, annihilation) = tokio::join!(
                fetch_challenge::<HyvChallengePeriod>(deadly_request, "deadly assault"),
                fetch_challenge::<HyvThresholdData>(threshold_request, "threshold simulation"),
                fetch_challenge::<HyvShiyuData>(shiyu_request, "shiyu defense"),
                fetch_challenge::<HyvChallengePeriod>(
                    annihilation_request,
                    "annihilation simulacrum"
                ),
            );
            let challenges = transform_challenges(
                deadly,
                threshold,
                shiyu,
                annihilation,
                &game_config.region,
                lang,
            );

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

            let calendar =
                transform_calendar(activity_data, gacha_data, challenges, &image_map);
            Ok(Bytes::from(
                serde_json::to_vec(&calendar).expect("CalendarResponse is always serializable"),
            ))
        })
        .await?;

    Ok(json_response(bytes))
}
