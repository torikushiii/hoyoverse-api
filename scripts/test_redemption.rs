//! Standalone test for the HoYoverse redemption API.
//!
//! Reads credentials from config.toml and redeems a code against the live API,
//! printing the raw response and its interpretation.
//!
//! Run with: cargo run --bin test-redemption -- <game> <code>
//! Example:  cargo run --bin test-redemption -- genshin GENSHINGIFT

use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
	validator: ValidatorConfig,
}

#[derive(Deserialize)]
struct ValidatorConfig {
	genshin: GameValidatorConfig,
	starrail: GameValidatorConfig,
	zenless: GameValidatorConfig,
	themis: GameValidatorConfig,
}

#[derive(Deserialize)]
struct GameValidatorConfig {
	cookie: String,
	uid: String,
	#[serde(default = "default_region")]
	region: String,
}

fn default_region() -> String {
	"os_usa".to_string()
}

#[derive(Debug, Deserialize)]
struct RedeemResponse {
	retcode: i32,
	message: String,
}

impl RedeemResponse {
	fn interpret(&self) -> &'static str {
		match self.retcode {
			0 => "success — code redeemed",
			-2017 | -2018 => "already redeemed (code is active)",
			-2021 | -2011 => "game level too low (code is active)",
			-2001 => "expired",
			-1065 | -2003 | -2004 | -2014 => "invalid / does not exist",
			-2006 => "max usage limit reached (invalid)",
			-2016 => "rate limited (cooldown)",
			-1071 => "invalid cookies",
			-1073 => "no game account bound to this HoYoLab account",
			-1075 => "no character on this server",
			_ => "unknown",
		}
	}

	fn is_credentials_issue(&self) -> bool {
		matches!(self.retcode, -1071 | -1073 | -1075)
	}
}

struct GameInfo {
	endpoint: &'static str,
	game_biz: &'static str,
	display_name: &'static str,
}

fn game_info(slug: &str) -> Option<GameInfo> {
	match slug {
		"genshin" => Some(GameInfo {
			endpoint: "https://sg-hk4e-api.hoyoverse.com/common/apicdkey/api/webExchangeCdkey",
			game_biz: "hk4e_global",
			display_name: "Genshin Impact",
		}),
		"starrail" => Some(GameInfo {
			endpoint: "https://sg-hkrpg-api.hoyoverse.com/common/apicdkey/api/webExchangeCdkey",
			game_biz: "hkrpg_global",
			display_name: "Honkai: Star Rail",
		}),
		"zenless" => Some(GameInfo {
			endpoint: "https://public-operation-nap.hoyoverse.com/common/apicdkey/api/webExchangeCdkey",
			game_biz: "nap_global",
			display_name: "Zenless Zone Zero",
		}),
		"themis" => Some(GameInfo {
			endpoint: "https://public-operation-common.hoyoverse.com/common/apicdkey/api/webExchangeCdkey",
			game_biz: "nxx_global",
			display_name: "Tears of Themis",
		}),
		_ => None,
	}
}

fn game_credentials<'a>(config: &'a ValidatorConfig, slug: &str) -> Option<&'a GameValidatorConfig> {
	match slug {
		"genshin" => Some(&config.genshin),
		"starrail" => Some(&config.starrail),
		"zenless" => Some(&config.zenless),
		"themis" => Some(&config.themis),
		_ => None,
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let args: Vec<String> = std::env::args().collect();
	if args.len() != 3 {
		eprintln!("Usage: {} <game> <code>", args[0]);
		eprintln!("Games: genshin, starrail, zenless, honkai, themis");
		std::process::exit(1);
	}

	let slug = args[1].to_lowercase();
	let code = args[2].to_uppercase();

	let config_str = std::fs::read_to_string("config.toml")
		.map_err(|_| anyhow::anyhow!("config.toml not found — run from project root"))?;
	let config: Config = toml::from_str(&config_str)?;

	let info = game_info(&slug)
		.ok_or_else(|| anyhow::anyhow!("no redeem endpoint configured for '{slug}'"))?;

	let creds = game_credentials(&config.validator, &slug)
		.ok_or_else(|| anyhow::anyhow!("unknown game '{slug}'"))?;

	if creds.cookie.is_empty() || creds.uid.is_empty() {
		anyhow::bail!(
			"cookie or uid is empty for [{slug}] in config.toml — fill them in first"
		);
	}

	println!("Game:     {}", info.display_name);
	println!("Code:     {code}");
	println!("UID:      {}", creds.uid);
	println!("Region:   {}", creds.region);
	println!("Endpoint: {}", info.endpoint);
	println!();

	let client = reqwest::Client::builder()
		.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
		.build()?;

	let timestamp = chrono::Utc::now().timestamp_millis().to_string();

	let mut params = vec![
		("cdkey", code.as_str()),
		("uid", creds.uid.as_str()),
		("region", creds.region.as_str()),
		("lang", "en"),
		("game_biz", info.game_biz),
		("t", timestamp.as_str()),
	];
	if slug == "genshin" {
		params.push(("sLangKey", "en-us"));
	}

	let mut req = client
		.get(info.endpoint)
		.query(&params)
		.header("Cookie", &creds.cookie);
	if slug == "themis" {
		req = req.header("Referer", "https://tot.hoyoverse.com/");
	}
	let resp = req
		.send()
		.await?
		.json::<RedeemResponse>()
		.await?;

	println!("retcode: {}", resp.retcode);
	println!("message: {}", resp.message);
	println!("result:  {}", resp.interpret());

	if resp.is_credentials_issue() {
		println!();
		println!("Hint: check that cookie, uid, and region in config.toml are correct for this game.");
	}

	Ok(())
}
