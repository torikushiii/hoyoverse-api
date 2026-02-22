pub mod sources;

#[cfg(test)]
mod tests;

use crate::{config::Settings, types::GameCode};
use sources::{dudcode, game8, gamerant, hoyolab, pcgamesn, zzzgg};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let mut codes = Vec::new();

    let (game8_codes, gamerant_codes, pcgamesn_codes, hoyolab_codes, dudcode_codes, zzzgg_codes) = tokio::join!(
        game8::fetch_codes(config),
        gamerant::fetch_codes(config),
        pcgamesn::fetch_codes(config),
        hoyolab::fetch_codes(config),
        dudcode::fetch_codes(config),
        zzzgg::fetch_codes(config),
    );

    if let Ok(mut source_codes) = game8_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = gamerant_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = pcgamesn_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = hoyolab_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = dudcode_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = zzzgg_codes {
        codes.append(&mut source_codes);
    }

    codes.sort_by(|a, b| a.code.to_lowercase().cmp(&b.code.to_lowercase()));
    codes.dedup_by(|a, b| a.code.to_lowercase() == b.code.to_lowercase());

    Ok(codes)
}
