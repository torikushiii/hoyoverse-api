use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::database::redemption_code::{RedemptionCode, RedemptionCodeResponse};
use crate::games::Game;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};

pub fn routes() -> Router<Arc<Global>> {
    Router::new().route("/:game/codes", get(get_codes))
}

#[derive(serde::Serialize)]
struct CodesResponse {
    active: Vec<RedemptionCodeResponse>,
    inactive: Vec<RedemptionCodeResponse>,
}

/// GET /mihoyo/:game/codes
///
/// Returns all redemption codes for the given game, split by active/inactive.
#[tracing::instrument(skip(global))]
async fn get_codes(
    State(global): State<Arc<Global>>,
    Path(game_slug): Path<String>,
) -> Result<Json<CodesResponse>, ApiError> {
    let game = Game::from_slug(&game_slug)
        .ok_or_else(|| ApiError::not_found(ApiErrorCode::UNKNOWN_GAME, "unknown game"))?;

    let all_codes = RedemptionCode::find_all(&global.db, game)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to query codes");
            ApiError::internal_server_error(ApiErrorCode::DATABASE_ERROR, "failed to query codes")
        })?;

    let (active, inactive): (Vec<_>, Vec<_>) = all_codes.into_iter().partition(|c| c.active);

    Ok(Json(CodesResponse {
        active: active.into_iter().map(Into::into).collect(),
        inactive: inactive.into_iter().map(Into::into).collect(),
    }))
}
