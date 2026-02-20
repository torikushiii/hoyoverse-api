use std::sync::Arc;

use anyhow::Context as _;
use serde::Deserialize;

use crate::games::Game;
use crate::global::Global;

#[derive(Debug, Deserialize)]
pub struct RedeemResponse {
    pub retcode: i32,
    pub message: String,
}

impl RedeemResponse {
    /// Code was successfully redeemed or already redeemed (still active).
    pub fn is_code_valid(&self) -> bool {
        // 0     = success
        // -2017 = already redeemed
        // -2018 = already redeemed (alt)
        // -2021 = game level too low (code is still valid)
        // -2011 = game level too low (alt)
        matches!(self.retcode, 0 | -2017 | -2018 | -2021 | -2011)
    }

    /// Code has expired.
    pub fn is_expired(&self) -> bool {
        // -2001 = code has expired
        self.retcode == -2001
    }

    /// Code is invalid / does not exist.
    pub fn is_invalid(&self) -> bool {
        // -1065 = invalid code
        // -2003 = incorrectly formatted
        // -2004 = invalid code
        // -2006 = max usage limit reached
        // -2014 = code not activated
        matches!(self.retcode, -1065 | -2003 | -2004 | -2006 | -2014)
    }

    /// Redemption is on cooldown (rate limited).
    pub fn is_cooldown(&self) -> bool {
        self.retcode == -2016
    }
}

/// Validate a redemption code against the HoYoverse API.
#[tracing::instrument(skip(global))]
pub async fn validate_code(
    global: &Arc<Global>,
    game: Game,
    code: &str,
) -> anyhow::Result<RedeemResponse> {
    let endpoint = game
        .redeem_endpoint()
        .with_context(|| format!("validation not yet supported for {}", game.display_name()))?;

    let game_biz = game
        .game_biz()
        .with_context(|| format!("game_biz not configured for {}", game.display_name()))?;

    let game_config = global
        .config
        .validator
        .game_config(game)
        .with_context(|| format!("{} has no redemption API", game.display_name()))?;
    let timestamp = chrono::Utc::now().timestamp_millis().to_string();

    let mut params = vec![
        ("cdkey", code),
        ("uid", game_config.uid.as_str()),
        ("region", game_config.region.as_str()),
        ("lang", "en"),
        ("game_biz", game_biz),
        ("t", timestamp.as_str()),
    ];
    if game == Game::Genshin {
        params.push(("sLangKey", "en-us"));
    }

    let mut req = global
        .http_client
        .get(endpoint)
        .query(&params)
        .header("Cookie", &game_config.cookie);
    if game == Game::Themis {
        req = req.header("Referer", crate::games::themis::REFERER);
    }
    let resp = req.send().await?.json::<RedeemResponse>().await?;

    tracing::info!(
        code,
        retcode = resp.retcode,
        message = %resp.message,
        "validated code"
    );

    Ok(resp)
}
