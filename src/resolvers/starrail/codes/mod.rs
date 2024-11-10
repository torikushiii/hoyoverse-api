pub mod sources;

#[cfg(test)]
mod tests;

use crate::{types::GameCode, config::Settings};
use sources::{eurogamer, game8, hoyolab};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let mut codes = Vec::new();

    // Fetch from multiple sources concurrently
    let (eurogamer_codes, game8_codes, hoyolab_codes) = tokio::join!(
        eurogamer::fetch_codes(config),
        game8::fetch_codes(config),
        hoyolab::fetch_codes(config),
    );

    // Combine results, ignoring errors from individual sources
    if let Ok(mut source_codes) = eurogamer_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = game8_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = hoyolab_codes {
        codes.append(&mut source_codes);
    }

    // Deduplicate codes based on the code string
    codes.sort_by(|a, b| a.code.cmp(&b.code));
    codes.dedup_by(|a, b| a.code == b.code);

    Ok(codes)
} 