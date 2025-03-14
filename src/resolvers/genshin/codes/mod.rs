pub mod sources;

#[cfg(test)]
mod tests;

use crate::{types::GameCode, config::Settings};
use anyhow::Result;
use sources::{fandom, game8, hoyolab};

pub async fn fetch_codes(config: &Settings) -> Result<Vec<GameCode>> {
    let (fandom_codes, game8_codes, hoyolab_codes) = tokio::join!(
        fandom::fetch_codes(config),
        game8::fetch_codes(config),
        hoyolab::fetch_codes(config),
    );

    let mut codes = Vec::new();

    // Combine results, ignoring errors from individual sources
    if let Ok(mut source_codes) = fandom_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = game8_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = hoyolab_codes {
        codes.append(&mut source_codes);
    }

    codes.sort_by(|a, b| a.code.to_lowercase().cmp(&b.code.to_lowercase()));
    codes.dedup_by(|a, b| a.code.to_lowercase() == b.code.to_lowercase());

    Ok(codes)
}