//! Combined test for Genshin Impact, Honkai: Star Rail, and Zenless Zone Zero activity calendars.
//!
//! Reads credentials from config.toml, calls each game's HoYoLab calendar endpoint,
//! and prints events, banners, and challenges in a unified format.
//!
//! Run with: cargo run --bin test-calendar

use serde::Deserialize;

const DEFAULT_LANG: &str = "en-us";

#[derive(Deserialize)]
struct Config {
    validator: ValidatorConfig,
}

#[derive(Deserialize)]
struct ValidatorConfig {
    genshin: GenshinConfig,
    starrail: StarRailConfig,
    zenless: ZenlessConfig,
}

#[derive(Deserialize)]
struct GenshinConfig {
    cookie: String,
    uid: String,
    #[serde(default = "genshin_default_region")]
    region: String,
}

fn genshin_default_region() -> String {
    "os_usa".to_string()
}

#[derive(Deserialize)]
struct StarRailConfig {
    cookie: String,
    uid: String,
    #[serde(default = "starrail_default_region")]
    region: String,
}

fn starrail_default_region() -> String {
    "prod_official_usa".to_string()
}

#[derive(Deserialize)]
struct ZenlessConfig {
    cookie: String,
    uid: String,
    #[serde(default = "zenless_default_region")]
    region: String,
}

fn zenless_default_region() -> String {
    "prod_gf_us".to_string()
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

fn map_profession(id: u8) -> &'static str {
    match id {
        1 => "attack",
        2 => "stun",
        3 => "anomaly",
        4 => "support",
        5 => "defence",
        6 => "rupture",
        _ => "unknown",
    }
}

fn map_element(id: u16) -> &'static str {
    match id {
        200 => "Physical",
        201 => "Fire",
        202 => "Ice",
        203 => "Electric",
        204 => "Wind",
        205 => "Ether",
        _ => "unknown",
    }
}

fn generate_ds_genshin(body: &str) -> String {
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

fn cookie_with_lang(cookie: &str, lang: &str) -> String {
    let mut parts: Vec<String> = cookie
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty() && !part.starts_with("mi18nLang="))
        .map(ToOwned::to_owned)
        .collect();
    parts.insert(0, format!("mi18nLang={lang}"));
    parts.join("; ")
}

const GENSHIN_CALENDAR_API: &str =
    "https://sg-public-api.hoyolab.com/event/game_record/genshin/api/act_calendar";

#[derive(Deserialize)]
struct GenshinResponse {
    retcode: i32,
    message: String,
    data: Option<GenshinCalendarData>,
}

#[derive(Deserialize)]
struct GenshinCalendarData {
    act_list: Vec<GenshinActivity>,
    fixed_act_list: Vec<GenshinActivity>,
    avatar_card_pool_list: Vec<GenshinBannerPool>,
    weapon_card_pool_list: Vec<GenshinBannerPool>,
    mixed_card_pool_list: Vec<GenshinBannerPool>,
}

#[derive(Deserialize)]
struct GenshinActivity {
    id: u64,
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    start_timestamp: String,
    end_timestamp: String,
    reward_list: Vec<GenshinReward>,
}

#[derive(Deserialize)]
struct GenshinReward {
    name: String,
    rarity: String,
    num: u64,
    homepage_show: bool,
}

#[derive(Deserialize)]
struct GenshinBannerPool {
    pool_id: u64,
    pool_name: String,
    version_name: String,
    avatars: Vec<GenshinAvatar>,
    weapon: Vec<GenshinWeapon>,
    start_timestamp: String,
    end_timestamp: String,
}

#[derive(Deserialize)]
struct GenshinAvatar {
    name: String,
    element: String,
    rarity: u8,
}

#[derive(Deserialize)]
struct GenshinWeapon {
    name: String,
    rarity: u8,
}

const STARRAIL_CALENDAR_API: &str =
    "https://sg-public-api.hoyolab.com/event/game_record/hkrpg/api/get_act_calender";

#[derive(Deserialize)]
struct StarRailResponse {
    retcode: i32,
    message: String,
    data: Option<StarRailCalendarData>,
}

#[derive(Deserialize)]
struct StarRailCalendarData {
    avatar_card_pool_list: Vec<SRAvatarPool>,
    equip_card_pool_list: Vec<SREquipPool>,
    act_list: Vec<SRActivity>,
    challenge_list: Vec<SRChallenge>,
}

#[derive(Deserialize)]
struct SRTimeInfo {
    start_ts: String,
    end_ts: String,
    start_time: String,
    end_time: String,
}

#[derive(Deserialize)]
struct SRAvatarPool {
    id: String,
    name: String,
    version: String,
    avatar_list: Vec<SRCharacter>,
    time_info: SRTimeInfo,
}

#[derive(Deserialize)]
struct SREquipPool {
    id: String,
    name: String,
    version: String,
    equip_list: Vec<SRLightCone>,
    time_info: SRTimeInfo,
}

#[derive(Deserialize)]
struct SRCharacter {
    item_name: String,
    rarity: String,
    damage_type: String,
    avatar_base_type: String,
}

#[derive(Deserialize)]
struct SRLightCone {
    item_name: String,
    rarity: String,
    avatar_base_type: String,
}

#[derive(Deserialize)]
struct SRActivity {
    id: u64,
    name: String,
    act_type: String,
    reward_list: Vec<SRReward>,
    special_reward: Option<SRReward>,
    time_info: SRTimeInfo,
}

#[derive(Deserialize)]
struct SRChallenge {
    group_id: u64,
    name_mi18n: String,
    challenge_type: String,
    special_reward: Option<SRReward>,
    time_info: SRTimeInfo,
}

#[derive(Deserialize)]
struct SRReward {
    item_id: u64,
    name: String,
    rarity: String,
    num: u64,
}

const ZENLESS_ACTIVITY_CALENDAR_API: &str =
    "https://sg-act-public-api.hoyolab.com/event/game_record_zzz/api/zzz/activity_calendar";
const ZENLESS_GACHA_CALENDAR_API: &str =
    "https://sg-act-public-api.hoyolab.com/event/game_record_zzz/api/zzz/gacha_calendar";

#[derive(Deserialize)]
struct ZenlessActivityResponse {
    retcode: i32,
    message: String,
    data: Option<ZenlessActivityData>,
}

#[derive(Deserialize)]
struct ZenlessActivityData {
    activity_list: Vec<ZenlessActivity>,
}

#[derive(Deserialize)]
struct ZenlessActivity {
    activity_id: u64,
    state: String,
    name: String,
    monochrome_cnt: u64,
    start_ts: i64,
    end_ts: i64,
}

#[derive(Deserialize)]
struct ZenlessGachaResponse {
    retcode: i32,
    message: String,
    data: Option<ZenlessGachaData>,
}

#[derive(Deserialize)]
struct ZenlessGachaData {
    avatar_gacha_schedule_list: Vec<ZenlessAvatarGacha>,
    weapon_gacha_schedule_list: Vec<ZenlessWeaponGacha>,
}

#[derive(Deserialize)]
struct ZenlessAvatarGacha {
    gacha_type: String,
    gacha_state: String,
    start_ts: i64,
    end_ts: i64,
    version: String,
    avatar_list: Vec<ZenlessAvatar>,
}

#[derive(Deserialize)]
struct ZenlessWeaponGacha {
    gacha_type: String,
    gacha_state: String,
    start_ts: i64,
    end_ts: i64,
    version: String,
    weapon_list: Vec<ZenlessWeapon>,
}

#[derive(Deserialize)]
struct ZenlessAvatar {
    avatar_id: u64,
    avatar_name: String,
    full_name: String,
    rarity: String,
    avatar_profession: u8,
    avatar_element_type: u16,
}

#[derive(Deserialize)]
struct ZenlessWeapon {
    weapon_id: u64,
    rarity: String,
    talent_title: String,
    profession: u8,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let lang = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_LANG.to_string());

    let config_str = std::fs::read_to_string("config.toml")
        .map_err(|_| anyhow::anyhow!("config.toml not found — run from project root"))?;
    let config: Config = toml::from_str(&config_str)?;

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;

    print_genshin_calendar(&client, &config.validator.genshin, &lang).await?;
    println!();
    print_starrail_calendar(&client, &config.validator.starrail, &lang).await?;
    println!();
    print_zenless_calendar(&client, &config.validator.zenless, &lang).await?;

    Ok(())
}

async fn print_genshin_calendar(
    client: &reqwest::Client,
    creds: &GenshinConfig,
    lang: &str,
) -> anyhow::Result<()> {
    if creds.cookie.is_empty() || creds.uid.is_empty() {
        anyhow::bail!("cookie or uid is empty for [validator.genshin] in config.toml");
    }

    println!("━━ Genshin Impact ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("UID:    {}", creds.uid);
    println!("Region: {}", creds.region);
    println!("Lang:   {}", lang);
    println!();

    let body = serde_json::json!({ "role_id": creds.uid, "server": creds.region });
    let body_str = body.to_string();
    let ds = generate_ds_genshin(&body_str);
    let cookie = cookie_with_lang(&creds.cookie, lang);

    let resp = client
        .post(GENSHIN_CALENDAR_API)
        .header("Cookie", cookie)
        .header("DS", ds)
        .header("x-rpc-app_version", "1.5.0")
        .header("x-rpc-client_type", "5")
        .header("x-rpc-language", lang)
        .json(&body)
        .send()
        .await?
        .json::<GenshinResponse>()
        .await?;

    if resp.retcode != 0 {
        anyhow::bail!("API error {}: {}", resp.retcode, resp.message);
    }

    let data = resp
        .data
        .ok_or_else(|| anyhow::anyhow!("API returned no data"))?;

    println!(
        "── Events ({}) ──────────────────────────────",
        data.act_list.len()
    );
    for e in &data.act_list {
        let special = e.reward_list.iter().find(|r| r.homepage_show);
        println!("  [{}] {} ({})", e.id, e.name, e.type_name);
        println!("       {} → {}", e.start_timestamp, e.end_timestamp);
        if let Some(sr) = special {
            println!(
                "       special: {} x{} (rarity {})",
                sr.name, sr.num, sr.rarity
            );
        }
        let rewards: Vec<_> = e.reward_list.iter().filter(|r| !r.homepage_show).collect();
        if !rewards.is_empty() {
            println!(
                "       rewards: {}",
                rewards
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    println!();

    let all_pools: Vec<_> = data
        .avatar_card_pool_list
        .iter()
        .chain(&data.weapon_card_pool_list)
        .chain(&data.mixed_card_pool_list)
        .collect();
    println!(
        "── Banners ({}) ──────────────────────────────",
        all_pools.len()
    );
    for b in &all_pools {
        println!("  [{}] {} v{}", b.pool_id, b.pool_name, b.version_name);
        println!("       {} → {}", b.start_timestamp, b.end_timestamp);
        if !b.avatars.is_empty() {
            let chars: Vec<_> = b
                .avatars
                .iter()
                .map(|a| format!("{} ({} ★{})", a.name, a.element, a.rarity))
                .collect();
            println!("       characters: {}", chars.join(", "));
        }
        if !b.weapon.is_empty() {
            let weapons: Vec<_> = b
                .weapon
                .iter()
                .map(|w| format!("{} ★{}", w.name, w.rarity))
                .collect();
            println!("       weapons: {}", weapons.join(", "));
        }
    }

    println!();

    println!(
        "── Challenges ({}) ──────────────────────────────",
        data.fixed_act_list.len()
    );
    for c in &data.fixed_act_list {
        let special = c.reward_list.iter().find(|r| r.homepage_show);
        println!("  [{}] {} ({})", c.id, c.name, c.type_name);
        println!("       {} → {}", c.start_timestamp, c.end_timestamp);
        if let Some(sr) = special {
            println!(
                "       special: {} x{} (rarity {})",
                sr.name, sr.num, sr.rarity
            );
        }
    }

    Ok(())
}

async fn print_starrail_calendar(
    client: &reqwest::Client,
    creds: &StarRailConfig,
    lang: &str,
) -> anyhow::Result<()> {
    if creds.cookie.is_empty() || creds.uid.is_empty() {
        anyhow::bail!("cookie or uid is empty for [validator.starrail] in config.toml");
    }

    println!("━━ Honkai: Star Rail ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("UID:    {}", creds.uid);
    println!("Region: {}", creds.region);
    println!("Lang:   {}", lang);
    println!();

    let ds = generate_ds_starrail();
    let cookie = cookie_with_lang(&creds.cookie, lang);

    let resp = client
        .get(STARRAIL_CALENDAR_API)
        .query(&[("server", &creds.region), ("role_id", &creds.uid)])
        .header("Cookie", cookie)
        .header("DS", ds)
        .header("x-rpc-app_version", "1.5.0")
        .header("x-rpc-client_type", "5")
        .header("x-rpc-language", lang)
        .send()
        .await?
        .json::<StarRailResponse>()
        .await?;

    if resp.retcode != 0 {
        anyhow::bail!("API error {}: {}", resp.retcode, resp.message);
    }

    let data = resp
        .data
        .ok_or_else(|| anyhow::anyhow!("API returned no data"))?;

    let events: Vec<_> = data
        .act_list
        .iter()
        .filter(|e| e.time_info.start_ts != "0" && e.time_info.end_ts != "0")
        .collect();
    println!(
        "── Events ({}) ──────────────────────────────",
        events.len()
    );
    for e in &events {
        let special = e.special_reward.as_ref().filter(|r| r.item_id != 0);
        println!("  [{}] {} ({})", e.id, e.name, e.act_type);
        println!(
            "       {} → {}",
            e.time_info.start_time, e.time_info.end_time
        );
        if let Some(sr) = special {
            println!(
                "       special: {} x{} (rarity {})",
                sr.name, sr.num, sr.rarity
            );
        }
        let rewards: Vec<_> = e.reward_list.iter().filter(|r| r.num > 0).collect();
        if !rewards.is_empty() {
            println!(
                "       rewards: {}",
                rewards
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    println!();

    let banner_count = data.avatar_card_pool_list.len() + data.equip_card_pool_list.len();
    println!(
        "── Banners ({}) ──────────────────────────────",
        banner_count
    );
    for p in &data.avatar_card_pool_list {
        println!("  [{}] {} v{}", p.id, p.name, p.version);
        println!(
            "       {} → {}",
            p.time_info.start_time, p.time_info.end_time
        );
        if !p.avatar_list.is_empty() {
            let list: Vec<_> = p
                .avatar_list
                .iter()
                .map(|c| {
                    format!(
                        "{} ({} path:{} ★{})",
                        c.item_name, c.damage_type, c.avatar_base_type, c.rarity
                    )
                })
                .collect();
            println!("       characters: {}", list.join(", "));
        }
    }
    for p in &data.equip_card_pool_list {
        println!("  [{}] {} v{}", p.id, p.name, p.version);
        println!(
            "       {} → {}",
            p.time_info.start_time, p.time_info.end_time
        );
        if !p.equip_list.is_empty() {
            let list: Vec<_> = p
                .equip_list
                .iter()
                .map(|c| {
                    format!(
                        "{} (path:{} ★{})",
                        c.item_name, c.avatar_base_type, c.rarity
                    )
                })
                .collect();
            println!("       light cones: {}", list.join(", "));
        }
    }

    println!();

    println!(
        "── Challenges ({}) ──────────────────────────────",
        data.challenge_list.len()
    );
    for c in &data.challenge_list {
        let special = c.special_reward.as_ref().filter(|r| r.item_id != 0);
        println!("  [{}] {} ({})", c.group_id, c.name_mi18n, c.challenge_type);
        println!(
            "       {} → {}",
            c.time_info.start_time, c.time_info.end_time
        );
        if let Some(sr) = special {
            println!(
                "       special: {} x{} (rarity {})",
                sr.name, sr.num, sr.rarity
            );
        }
    }

    Ok(())
}

async fn print_zenless_calendar(
    client: &reqwest::Client,
    creds: &ZenlessConfig,
    lang: &str,
) -> anyhow::Result<()> {
    if creds.cookie.is_empty() || creds.uid.is_empty() {
        anyhow::bail!("cookie or uid is empty for [validator.zenless] in config.toml");
    }

    println!("━━ Zenless Zone Zero ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("UID:    {}", creds.uid);
    println!("Region: {}", creds.region);
    println!("Lang:   {}", lang);
    println!();

    let cookie = cookie_with_lang(&creds.cookie, lang);

    let activity_resp = client
        .get(ZENLESS_ACTIVITY_CALENDAR_API)
        .query(&[("uid", &creds.uid), ("region", &creds.region)])
        .header("Cookie", cookie.clone())
        .header("x-rpc-language", lang)
        .send()
        .await?
        .json::<ZenlessActivityResponse>()
        .await?;

    if activity_resp.retcode != 0 {
        anyhow::bail!(
            "activity API error {}: {}",
            activity_resp.retcode,
            activity_resp.message
        );
    }

    let activity_data = activity_resp
        .data
        .ok_or_else(|| anyhow::anyhow!("activity API returned no data"))?;

    let gacha_resp = client
        .get(ZENLESS_GACHA_CALENDAR_API)
        .query(&[("uid", &creds.uid), ("region", &creds.region)])
        .header("Cookie", cookie)
        .header("x-rpc-language", lang)
        .send()
        .await?
        .json::<ZenlessGachaResponse>()
        .await?;

    if gacha_resp.retcode != 0 {
        anyhow::bail!(
            "gacha API error {}: {}",
            gacha_resp.retcode,
            gacha_resp.message
        );
    }

    let gacha_data = gacha_resp
        .data
        .ok_or_else(|| anyhow::anyhow!("gacha API returned no data"))?;

    println!(
        "── Events ({}) ──────────────────────────────",
        activity_data.activity_list.len()
    );
    for e in &activity_data.activity_list {
        println!("  [{}] {} ({})", e.activity_id, e.name, e.state);
        println!("       {} → {}", e.start_ts, e.end_ts);
        println!("       polychrome: {}", e.monochrome_cnt);
    }

    println!();

    let banner_count =
        gacha_data.avatar_gacha_schedule_list.len() + gacha_data.weapon_gacha_schedule_list.len();
    println!(
        "── Banners ({}) ──────────────────────────────",
        banner_count
    );
    for b in &gacha_data.avatar_gacha_schedule_list {
        println!("  {} v{} ({})", b.gacha_type, b.version, b.gacha_state);
        println!("       {} → {}", b.start_ts, b.end_ts);
        if !b.avatar_list.is_empty() {
            let agents: Vec<_> = b
                .avatar_list
                .iter()
                .map(|a| {
                    let name = if a.full_name.is_empty() {
                        a.avatar_name.as_str()
                    } else {
                        a.full_name.as_str()
                    };
                    format!(
                        "{} #{} (★{} {} {})",
                        name,
                        a.avatar_id,
                        a.rarity,
                        map_profession(a.avatar_profession),
                        map_element(a.avatar_element_type)
                    )
                })
                .collect();
            println!("       agents: {}", agents.join(", "));
        }
    }
    for b in &gacha_data.weapon_gacha_schedule_list {
        println!("  {} v{} ({})", b.gacha_type, b.version, b.gacha_state);
        println!("       {} → {}", b.start_ts, b.end_ts);
        if !b.weapon_list.is_empty() {
            let weapons: Vec<_> = b
                .weapon_list
                .iter()
                .map(|w| {
                    format!(
                        "{} #{} (★{} {})",
                        w.talent_title,
                        w.weapon_id,
                        w.rarity,
                        map_profession(w.profession)
                    )
                })
                .collect();
            println!("       w-engines: {}", weapons.join(", "));
        }
    }

    Ok(())
}
