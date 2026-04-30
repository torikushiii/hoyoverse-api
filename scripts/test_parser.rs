//! Standalone test for any scraper parser.
//!
//! Fetches the live source page and runs the parser, printing every code it
//! finds along with rewards. Useful for verifying parsers work without needing
//! MongoDB or the full service running.
//!
//! Run with: cargo run --bin test-parser -- <game> <source>
//! Examples:
//!   cargo run --bin test-parser -- genshin fandom
//!   cargo run --bin test-parser -- genshin game8
//!   cargo run --bin test-parser -- starrail fandom

use std::collections::HashMap;

use hoyoverse_api::scraper::sources::genshin::{fandom as genshin_fandom, game8 as genshin_game8};
use hoyoverse_api::scraper::sources::honkai::fandom as honkai_fandom;
use hoyoverse_api::scraper::sources::starrail::{
    fandom as starrail_fandom, game8 as starrail_game8, sportskeeda as starrail_sportskeeda,
};
use hoyoverse_api::scraper::sources::themis::tot_wiki as themis_tot_wiki;
use hoyoverse_api::scraper::sources::zenless::{fandom as zenless_fandom, game8 as zenless_game8};
use serde_json::Value;

struct Code {
    code: String,
    rewards: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game> <source>", args[0]);
        print_known_combos();
        std::process::exit(1);
    }

    let game = args[1].to_lowercase();
    let source = args[2].to_lowercase();

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;

    println!("Fetching {game}/{source}...\n");

    let codes: Vec<Code> = match (game.as_str(), source.as_str()) {
        ("genshin", "fandom") => {
            let wikitext = fetch_fandom_wikitext(
                &client,
                "https://genshin-impact.fandom.com/api.php",
                "Promotional_Code",
            )
            .await?;
            genshin_fandom::parse_wikitext(&wikitext)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        ("genshin", "game8") => {
            let html = client
                .get("https://game8.co/games/Genshin-Impact/archives/304759")
                .send()
                .await?
                .text()
                .await?;
            println!("HTML length: {} chars\n", html.len());
            genshin_game8::parse_html(&html)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        ("starrail", "fandom") => {
            let wikitext = fetch_fandom_wikitext(
                &client,
                "https://honkai-star-rail.fandom.com/api.php",
                "Redemption_Code",
            )
            .await?;
            starrail_fandom::parse_wikitext(&wikitext)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        ("starrail", "game8") => {
            let html = client
                .get("https://game8.co/games/Honkai-Star-Rail/archives/410296")
                .send()
                .await?
                .text()
                .await?;
            println!("HTML length: {} chars\n", html.len());

            starrail_game8::parse_html(&html)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        ("starrail", "sportskeeda") => {
            let html = client
                .get("https://www.sportskeeda.com/esports/honkai-star-rail-hsr-4-0-redeem-codes")
                .send()
                .await?
                .text()
                .await?;
            println!("HTML length: {} chars\n", html.len());

            starrail_sportskeeda::parse_html(&html)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        ("starrail", "hoyolab") => {
            let resp: Value = client
                .get("https://bbs-api-os.hoyolab.com/community/painter/wapi/circle/channel/guide/material?game_id=6")
                .header("x-rpc-app_version", "4.8.0")
                .header("x-rpc-client_type", "4")
                .header("x-rpc-language", "en-us")
                .header("Referer", "https://www.hoyolab.com/")
                .send()
                .await?
                .json()
                .await?;

            let item_name = |url: &str| -> Option<&'static str> {
                let filename = url.rsplit('/').next().unwrap_or(url);
                let hash = filename.split('.').next().unwrap_or(filename);
                match hash {
                    "77cb5426637574ba524ac458fa963da0_6409817950389238658" => Some("Stellar Jade"),
                    "7cb0e487e051f177d3f41de8d4bbc521_2556290033227986328" => {
                        Some("Refined Aether")
                    }
                    "508229a94e4fa459651f64c1cd02687a_6307505132287490837" => {
                        Some("Traveler's Guide")
                    }
                    "0b12bdf76fa4abc6b4d1fdfc0fb4d6f5_4521150989210768295" => Some("Credit"),
                    _ => None,
                }
            };

            resp["data"]["modules"]
                .as_array()
                .map(|modules| {
                    modules
                        .iter()
                        .filter_map(|m| m["exchange_group"]["bonuses"].as_array())
                        .flatten()
                        .filter(|b| {
                            b["code_status"].as_str() == Some("ON")
                                && !b["exchange_code"].as_str().unwrap_or("").is_empty()
                        })
                        .map(|b| {
                            let rewards = b["icon_bonuses"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|ib| {
                                            let url = ib["icon_url"].as_str()?;
                                            let num = ib["bonus_num"].as_u64()?;
                                            let name = item_name(url)?;
                                            Some(format!("{} ×{}", name, num))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            Code {
                                code: b["exchange_code"].as_str().unwrap_or("").to_string(),
                                rewards,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        ("zenless", "fandom") => {
            let wikitext = fetch_fandom_wikitext(
                &client,
                "https://zenless-zone-zero.fandom.com/api.php",
                "Redemption_Code",
            )
            .await?;
            zenless_fandom::parse_wikitext(&wikitext)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        ("zenless", "game8") => {
            let html = client
                .get("https://game8.co/games/Zenless-Zone-Zero/archives/435683")
                .send()
                .await?
                .text()
                .await?;
            println!("HTML length: {} chars\n", html.len());
            zenless_game8::parse_html(&html)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        ("zenless", "hoyolab") => {
            let resp: Value = client
                .get("https://bbs-api-os.hoyolab.com/community/painter/wapi/circle/channel/guide/material?game_id=8")
                .header("x-rpc-app_version", "4.8.0")
                .header("x-rpc-client_type", "4")
                .header("x-rpc-language", "en-us")
                .header("Referer", "https://www.hoyolab.com/")
                .send()
                .await?
                .json()
                .await?;

            let item_name = |url: &str| -> Option<&'static str> {
                let filename = url.rsplit('/').next().unwrap_or(url);
                let hash = filename.split('.').next().unwrap_or(filename);
                match hash {
                    "cd6682dd2d871dc93dfa28c3f281d527_6175554878133394960" => Some("Dennies"),
                    "8609070fe148c0e0e367cda25fdae632_208324374592932270" => Some("Polychrome"),
                    "6ef3e419022c871257a936b1857ac9d1_411767156105350865" => {
                        Some("W-Engine Energy Module")
                    }
                    "86e1f7a5ff283d527bbc019475847174_5751095862610622324" => {
                        Some("Senior Investigator Logs")
                    }
                    _ => None,
                }
            };

            resp["data"]["modules"]
                .as_array()
                .map(|modules| {
                    modules
                        .iter()
                        .filter_map(|m| m["exchange_group"]["bonuses"].as_array())
                        .flatten()
                        .filter(|b| {
                            b["code_status"].as_str() == Some("ON")
                                && !b["exchange_code"].as_str().unwrap_or("").is_empty()
                        })
                        .map(|b| {
                            let rewards = b["icon_bonuses"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|ib| {
                                            let url = ib["icon_url"].as_str()?;
                                            let num = ib["bonus_num"].as_u64()?;
                                            let name = item_name(url)?;
                                            Some(format!("{} ×{}", name, num))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            Code {
                                code: b["exchange_code"].as_str().unwrap_or("").to_string(),
                                rewards,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        ("honkai", "fandom") => {
            let wikitext = fetch_fandom_wikitext(
                &client,
                "https://honkaiimpact3.fandom.com/api.php",
                "Exchange_Rewards",
            )
            .await?;
            honkai_fandom::parse_wikitext(&wikitext)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        ("themis", "totwiki") => {
            let html = client
                .get("https://tot.wiki/wiki/Redeem_Code")
                .send()
                .await?
                .text()
                .await?;
            println!("HTML length: {} chars\n", html.len());
            themis_tot_wiki::parse_html(&html)
                .into_iter()
                .map(|c| Code {
                    code: c.code,
                    rewards: c.rewards,
                })
                .collect()
        }

        _ => {
            eprintln!("Unknown combination: {game} {source}");
            print_known_combos();
            std::process::exit(1);
        }
    };

    print_results(&codes);
    Ok(())
}

async fn fetch_fandom_wikitext(
    client: &reqwest::Client,
    api_url: &str,
    page: &str,
) -> anyhow::Result<String> {
    let resp = client
        .get(api_url)
        .query(&[
            ("action", "parse"),
            ("page", page),
            ("format", "json"),
            ("prop", "wikitext"),
        ])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let wikitext = resp["parse"]["wikitext"]["*"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("failed to extract wikitext from {api_url}"))?
        .to_string();

    println!("Wikitext length: {} chars\n", wikitext.len());
    Ok(wikitext)
}

fn print_results(codes: &[Code]) {
    println!("Parsed {} code entries:\n", codes.len());
    println!("{:<30} REWARDS", "CODE");
    println!("{}", "-".repeat(80));

    for code in codes {
        let normalized = code.code.to_uppercase();
        let dup_marker = if normalized != code.code {
            " (mixed case)"
        } else {
            ""
        };
        let rewards = if code.rewards.is_empty() {
            "(none)".to_string()
        } else {
            code.rewards.join(", ")
        };
        println!("{:<30} {}{}", &code.code, rewards, dup_marker);
    }

    let mut seen: HashMap<String, Vec<String>> = HashMap::new();
    for code in codes {
        seen.entry(code.code.to_uppercase())
            .or_default()
            .push(code.code.clone());
    }

    let duplicates: Vec<_> = seen.iter().filter(|(_, v)| v.len() > 1).collect();
    if duplicates.is_empty() {
        println!("\nNo case-insensitive duplicates found.");
    } else {
        println!("\nCase-insensitive duplicates found:");
        for (upper, variants) in &duplicates {
            println!("  {} -> {:?}", upper, variants);
        }
    }
}

fn print_known_combos() {
    eprintln!("Known combinations:");
    eprintln!("  genshin   fandom");
    eprintln!("  genshin   game8");
    eprintln!("  starrail  fandom");
    eprintln!("  starrail  game8");
    eprintln!("  starrail  sportskeeda");
    eprintln!("  starrail  hoyolab");
    eprintln!("  zenless   fandom");
    eprintln!("  zenless   game8");
    eprintln!("  zenless   hoyolab");
    eprintln!("  honkai    fandom");
    eprintln!("  themis    totwiki");
}
