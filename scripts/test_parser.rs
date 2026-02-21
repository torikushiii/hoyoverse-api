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
    fandom as starrail_fandom, game8 as starrail_game8,
};
use hoyoverse_api::scraper::sources::themis::tot_wiki as themis_tot_wiki;
use hoyoverse_api::scraper::sources::zenless::{fandom as zenless_fandom, game8 as zenless_game8};

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
    eprintln!("  zenless   fandom");
    eprintln!("  zenless   game8");
    eprintln!("  honkai    fandom");
    eprintln!("  themis    totwiki");
}
