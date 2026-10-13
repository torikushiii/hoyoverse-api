use std::collections::HashSet;
use std::sync::Arc;

use futures::TryStreamExt as _;
use mongodb::bson::doc;

use crate::database::redemption_code::RedemptionCode;
use crate::games::Game;
use crate::global::Global;
use crate::notifier::discord;
use crate::scraper::sources::ParsedCode;
use crate::validator::hoyoverse_api;

const MAX_COOLDOWN_RETRIES: u32 = 5;

pub type SourceResult = (&'static str, anyhow::Result<Vec<ParsedCode>>);

#[derive(Debug, Clone, Copy)]
pub struct StoreOptions {
    pub uppercase: bool,
    pub validate: bool,
    pub sync_active: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct MergedCode {
    code: String,
    rewards: Vec<String>,
    source: &'static str,
}

#[derive(Debug)]
enum Verdict {
    Active,
    Inactive,
    Deferred,
}

fn merge(results: Vec<SourceResult>, uppercase: bool) -> (Vec<MergedCode>, bool) {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    let mut all_ok = true;

    for (source, result) in results {
        match result {
            Ok(scraped) => {
                for parsed in scraped {
                    let code = if uppercase {
                        parsed.code.to_uppercase()
                    } else {
                        parsed.code
                    };
                    if seen.insert(code.clone()) {
                        merged.push(MergedCode {
                            code,
                            rewards: parsed.rewards,
                            source,
                        });
                    }
                }
            }
            Err(e) => {
                all_ok = false;
                tracing::error!(source, error = %e, "source scraper failed");
            }
        }
    }

    (merged, all_ok)
}

async fn verify(global: &Arc<Global>, game: Game, code: &str) -> Verdict {
    for _ in 0..=MAX_COOLDOWN_RETRIES {
        match hoyoverse_api::validate_code(global, game, code).await {
            Ok(resp) if resp.is_cooldown() => {
                tracing::warn!(code, "hit cooldown, retrying");
            }
            Ok(resp) if resp.is_code_valid() => return Verdict::Active,
            Ok(resp) if resp.is_expired() || resp.is_invalid() => return Verdict::Inactive,
            Ok(resp) if resp.is_credentials_error() => {
                tracing::warn!(
                    code,
                    retcode = resp.retcode,
                    "invalid credentials, storing unverified code as active"
                );
                return Verdict::Active;
            }
            Ok(resp) => {
                tracing::warn!(
                    code,
                    retcode = resp.retcode,
                    message = %resp.message,
                    "unexpected redeem response, deferring code"
                );
                return Verdict::Deferred;
            }
            Err(e) => {
                tracing::warn!(code, error = %e, "validation request failed, deferring code");
                return Verdict::Deferred;
            }
        }
    }

    tracing::warn!(code, "cooldown persisted, deferring code");
    Verdict::Deferred
}

pub async fn store_codes(
    global: &Arc<Global>,
    game: Game,
    results: Vec<SourceResult>,
    options: StoreOptions,
) -> anyhow::Result<()> {
    let (merged, all_ok) = merge(results, options.uppercase);

    if merged.is_empty() {
        return Ok(());
    }

    let collection = RedemptionCode::collection(&global.db, game);
    let total = merged.len();

    let candidates: Vec<String> = merged.iter().map(|c| c.code.clone()).collect();
    let existing: HashSet<String> = collection
        .find(doc! { "code": { "$in": &candidates } })
        .await?
        .try_collect::<Vec<RedemptionCode>>()
        .await?
        .into_iter()
        .map(|c| c.code)
        .collect();

    let validate = options.validate
        && global
            .config
            .validator
            .game_config(game)
            .is_some_and(|c| c.enabled)
        && game.redeem_endpoint().is_some();

    let mut discovered: Vec<(String, Vec<String>, String)> = Vec::new();
    let mut inserted = 0;

    for MergedCode {
        code,
        rewards,
        source,
    } in merged.into_iter().filter(|c| !existing.contains(&c.code))
    {
        let active = if validate {
            match verify(global, game, &code).await {
                Verdict::Active => true,
                Verdict::Inactive => false,
                Verdict::Deferred => continue,
            }
        } else {
            true
        };

        let doc = RedemptionCode {
            code: code.clone(),
            active,
            date: bson::DateTime::now(),
            rewards: rewards.clone(),
            source: source.to_string(),
        };

        if let Err(e) = collection.insert_one(doc).await {
            tracing::error!(code, error = %e, "failed to insert code");
            continue;
        }
        inserted += 1;

        if active {
            tracing::info!(code, source, "new code discovered");
            metrics::counter!("scraper_codes_discovered_total", "game" => game.slug()).increment(1);
            discovered.push((code, rewards, source.to_string()));
        } else {
            tracing::warn!(code, "code is invalid, storing as inactive");
            metrics::counter!("scraper_codes_invalid_total", "game" => game.slug()).increment(1);
        }
    }

    let (mut deactivated, mut reactivated) = (0, 0);
    if options.sync_active {
        if all_ok {
            deactivated = collection
                .update_many(
                    doc! { "active": true, "code": { "$nin": &candidates } },
                    doc! { "$set": { "active": false } },
                )
                .await?
                .modified_count;
            reactivated = collection
                .update_many(
                    doc! { "active": false, "code": { "$in": &candidates } },
                    doc! { "$set": { "active": true } },
                )
                .await?
                .modified_count;
        } else {
            tracing::warn!("skipping active sync because a source failed");
        }
    }

    tracing::info!(
        game = game.slug(),
        new = discovered.len(),
        inserted,
        deactivated,
        reactivated,
        total,
        "scrape complete"
    );

    if !discovered.is_empty() {
        discord::notify_new_codes(global, game, &discovered).await;
    }

    if inserted > 0 || deactivated > 0 || reactivated > 0 {
        global.response_cache.remove(&game.codes_path()).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(code: &str, reward: &str) -> ParsedCode {
        ParsedCode {
            code: code.to_string(),
            rewards: vec![reward.to_string()],
        }
    }

    #[test]
    fn first_source_wins_on_duplicate_codes() {
        let (merged, all_ok) = merge(
            vec![
                ("fandom", Ok(vec![parsed("abc", "fandom reward")])),
                (
                    "game8",
                    Ok(vec![parsed("ABC", "game8 reward"), parsed("XYZ", "x")]),
                ),
            ],
            true,
        );

        assert!(all_ok);
        assert_eq!(
            merged,
            vec![
                MergedCode {
                    code: "ABC".to_string(),
                    rewards: vec!["fandom reward".to_string()],
                    source: "fandom",
                },
                MergedCode {
                    code: "XYZ".to_string(),
                    rewards: vec!["x".to_string()],
                    source: "game8",
                },
            ]
        );
    }

    #[test]
    fn keeps_case_when_uppercase_is_disabled() {
        let (merged, _) = merge(
            vec![("tot_wiki", Ok(vec![parsed("MixedCase", "r")]))],
            false,
        );

        assert_eq!(merged[0].code, "MixedCase");
    }

    #[test]
    fn failed_source_clears_all_ok_but_keeps_other_codes() {
        let (merged, all_ok) = merge(
            vec![
                ("fandom", Err(anyhow::anyhow!("boom"))),
                ("game8", Ok(vec![parsed("ABC", "r")])),
            ],
            true,
        );

        assert!(!all_ok);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "game8");
    }
}
