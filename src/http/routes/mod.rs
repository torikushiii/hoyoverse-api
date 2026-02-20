use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::games::Game;
use crate::global::Global;

pub mod codes;

pub fn routes() -> Router<Arc<Global>> {
	Router::new()
		.route("/", get(root))
		.merge(codes::routes())
}

#[derive(serde::Serialize)]
struct RootResponse {
	message: &'static str,
	version: &'static str,
	uptime: u64,
	endpoints: Vec<String>,
}

#[tracing::instrument(skip(global))]
async fn root(State(global): State<Arc<Global>>) -> Json<RootResponse> {
	let games = [Game::Genshin, Game::Starrail, Game::Zenless, Game::Honkai, Game::Themis];

	let endpoints: Vec<String> = games
		.iter()
		.map(|g| format!("/mihoyo/{}/codes", g.slug()))
		.collect();

	Json(RootResponse {
		message: "HoYoverse Redemption Code API",
		version: env!("CARGO_PKG_VERSION"),
		uptime: global.started_at.elapsed().as_secs(),
		endpoints,
	})
}
