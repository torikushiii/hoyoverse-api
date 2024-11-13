pub mod sources;

#[cfg(test)]
mod tests;

use crate::{types::GameCode, config::Settings};
use sources::totwiki;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let mut codes = Vec::new();

    // For now we only have tot-wiki as a source
    if let Ok(mut source_codes) = totwiki::fetch_codes(config).await {
        codes.append(&mut source_codes);
    }

    // Deduplicate codes based on the code string
    codes.sort_by(|a, b| a.code.cmp(&b.code));
    codes.dedup_by(|a, b| a.code == b.code);

    Ok(codes)
}