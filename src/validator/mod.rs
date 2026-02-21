use std::sync::Arc;

use crate::database::redemption_code::RedemptionCode;
use crate::games::Game;
use crate::global::Global;
use crate::util::sleep_until_aligned;

pub mod hoyoverse_api;

#[tracing::instrument(name = "Validator", skip_all)]
pub async fn run(global: Arc<Global>) -> anyhow::Result<()> {
    if !global.config.validator.enabled {
        tracing::info!("validator is disabled");
        std::future::pending::<()>().await;
        return Ok(());
    }

    let interval_secs = global.config.validator.interval_secs;
    tracing::info!(interval_secs, "starting validator");

    loop {
        sleep_until_aligned(interval_secs).await;

        if let Err(e) = validate_all_codes(&global).await {
            tracing::error!("validation cycle failed: {:#}", e);
        }
    }
}

#[tracing::instrument(skip_all)]
async fn validate_all_codes(global: &Arc<Global>) -> anyhow::Result<()> {
    let all_games = [
        Game::Genshin,
        Game::Starrail,
        Game::Zenless,
        Game::Honkai,
        Game::Themis,
    ];

    for game in all_games {
        let Some(game_config) = global.config.validator.game_config(game) else {
            continue; // game has no redemption API (e.g. Honkai)
        };

        if !game_config.enabled {
            continue;
        }

        if game.redeem_endpoint().is_none() {
            tracing::warn!(
                game = game.display_name(),
                "redeem endpoint not configured, skipping"
            );
            continue;
        }

        let codes = RedemptionCode::find_active(&global.db, game).await?;

        tracing::info!(
            game = game.display_name(),
            count = codes.len(),
            "validating active codes"
        );

        for code in &codes {
            match hoyoverse_api::validate_code(global, game, &code.code).await {
                Ok(resp) => {
                    if resp.is_expired() || resp.is_invalid() {
                        tracing::warn!(
                            code = code.code,
                            retcode = resp.retcode,
                            message = %resp.message,
                            "marking code as inactive"
                        );
                        RedemptionCode::set_active(&global.db, game, &code.code, false).await?;
                    } else if resp.is_cooldown() {
                        tracing::warn!(
                            game = game.display_name(),
                            "hit redemption cooldown, skipping remaining codes"
                        );
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(code = code.code, error = %e, "failed to validate code");
                }
            }

            // Rate limit: HoYoverse enforces ~5s between redemptions
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }

        global
            .response_cache
            .remove(&format!("/mihoyo/{}/codes", game.slug()))
            .await;
    }

    Ok(())
}
