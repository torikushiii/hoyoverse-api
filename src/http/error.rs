use std::borrow::Cow;

use axum::response::IntoResponse;
use axum::Json;
use hyper::StatusCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct ApiErrorCode(pub u16);

impl ApiErrorCode {
	/// The requested route does not exist.
	pub const ROUTE_NOT_FOUND: Self = Self(404);
	/// The requested game slug does not exist.
	pub const UNKNOWN_GAME: Self = Self(1000);
	/// A database query failed unexpectedly.
	pub const DATABASE_ERROR: Self = Self(2000);
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiError {
	#[serde(skip)]
	pub status_code: StatusCode,
	pub status: Cow<'static, str>,
	pub error_code: ApiErrorCode,
	pub error: Cow<'static, str>,
}

impl ApiError {
	pub fn new(status_code: StatusCode, error_code: ApiErrorCode, error: impl Into<Cow<'static, str>>) -> Self {
		Self {
			status_code,
			status: status_code.canonical_reason().unwrap_or("unknown").into(),
			error_code,
			error: error.into(),
		}
	}

	pub fn bad_request(error_code: ApiErrorCode, error: impl Into<Cow<'static, str>>) -> Self {
		Self::new(StatusCode::BAD_REQUEST, error_code, error)
	}

	pub fn not_found(error_code: ApiErrorCode, error: impl Into<Cow<'static, str>>) -> Self {
		Self::new(StatusCode::NOT_FOUND, error_code, error)
	}

	pub fn internal_server_error(error_code: ApiErrorCode, error: impl Into<Cow<'static, str>>) -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR, error_code, error)
	}
}

impl IntoResponse for ApiError {
	fn into_response(self) -> axum::http::Response<axum::body::Body> {
		(self.status_code, Json(self)).into_response()
	}
}
