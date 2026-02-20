use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::games::Game;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedemptionCode {
	pub code: String,
	pub active: bool,
	pub date: bson::DateTime,
	pub rewards: Vec<String>,
	pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedemptionCodeResponse {
	pub code: String,
	pub rewards: Vec<String>,
}

impl RedemptionCode {
	pub fn collection(db: &mongodb::Database, game: Game) -> mongodb::Collection<Self> {
		db.collection(game.collection_name())
	}

	/// Fetch all active codes for a game.
	#[tracing::instrument(skip(db))]
	pub async fn find_active(db: &mongodb::Database, game: Game) -> anyhow::Result<Vec<Self>> {
		use futures::TryStreamExt;

		let collection = Self::collection(db, game);
		let codes = collection
			.find(doc! { "active": true })
			.await?
			.try_collect()
			.await?;

		Ok(codes)
	}

	#[tracing::instrument(skip(db))]
	pub async fn find_all(db: &mongodb::Database, game: Game) -> anyhow::Result<Vec<Self>> {
		use futures::TryStreamExt;

		let collection = Self::collection(db, game);
		let codes = collection
			.find(doc! {})
			.await?
			.try_collect()
			.await?;

		Ok(codes)
	}

	#[tracing::instrument(skip(db))]
	pub async fn exists(db: &mongodb::Database, game: Game, code: &str) -> anyhow::Result<bool> {
		let collection = Self::collection(db, game);
		let count = collection
			.count_documents(doc! { "code": code })
			.await?;

		Ok(count > 0)
	}

	#[tracing::instrument(skip(db))]
	pub async fn set_active(
		db: &mongodb::Database,
		game: Game,
		code: &str,
		active: bool,
	) -> anyhow::Result<()> {
		let collection = Self::collection(db, game);
		collection
			.update_one(
				doc! { "code": code },
				doc! { "$set": { "active": active } },
			)
			.await?;

		Ok(())
	}
}

impl From<RedemptionCode> for RedemptionCodeResponse {
	fn from(code: RedemptionCode) -> Self {
		Self {
			code: code.code,
			rewards: code.rewards,
		}
	}
}
