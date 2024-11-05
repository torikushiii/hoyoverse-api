pub mod sources;

#[cfg(test)]
mod tests;

use crate::types::GameCode;
use sources::{eurogamer, game8};

pub async fn fetch_all_codes() -> anyhow::Result<Vec<GameCode>> {
    let mut codes = Vec::new();

    // Fetch from multiple sources concurrently
    let (eurogamer_codes, game8_codes) = tokio::join!(
        eurogamer::fetch_codes(),
        game8::fetch_codes(),
    );

    // Combine results, ignoring errors from individual sources
    if let Ok(mut source_codes) = eurogamer_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = game8_codes {
        codes.append(&mut source_codes);
    }

    // Deduplicate codes based on the code string
    codes.sort_by(|a, b| a.code.cmp(&b.code));
    codes.dedup_by(|a, b| a.code == b.code);

    Ok(codes)
} 