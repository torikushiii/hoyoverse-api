use std::borrow::Cow;
use axum::{response::IntoResponse, Json};
use hyper::{HeaderMap, StatusCode};
use tracing::error;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiError {
    #[serde(skip)]
    pub status_code: StatusCode,
    pub status: Cow<'static, str>,
    pub error_code: ApiErrorCode,
    pub error: Cow<'static, str>,
    #[serde(skip)]
    pub extra_headers: Option<Box<HeaderMap>>,
}

#[derive(Debug, Clone, Copy, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u16)]
pub enum ApiErrorCode {
    Unknown = 0,

    // Database Errors
    DatabaseError = 1000,
    CacheError = 1001,

    // Rate Limiting
    RateLimitExceeded = 2000,

    // Resource Errors
    ResourceNotFound = 3000,
    InvalidResource = 3001,

    // Request Errors
    BadRequest = 4000,
    InvalidLanguage = 4001,
    InvalidCategory = 4002,

    // Service Errors
    ServiceUnavailable = 5000,
    ExternalServiceError = 5001,
}

impl ApiErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::DatabaseError => "DATABASE_ERROR",
            Self::CacheError => "CACHE_ERROR",
            Self::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            Self::ResourceNotFound => "RESOURCE_NOT_FOUND",
            Self::InvalidResource => "INVALID_RESOURCE",
            Self::BadRequest => "BAD_REQUEST",
            Self::InvalidLanguage => "INVALID_LANGUAGE",
            Self::InvalidCategory => "INVALID_CATEGORY",
            Self::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            Self::ExternalServiceError => "EXTERNAL_SERVICE_ERROR",
        }
    }
}

impl ApiError {
    pub fn new(
        status_code: StatusCode,
        error_code: ApiErrorCode,
        error: impl Into<Cow<'static, str>>
    ) -> Self {
        let error_message = error.into();
        error!("API Error: {} - {}", error_code.as_str(), error_message);

        Self {
            status_code,
            error_code,
            error: error_message,
            status: status_code.canonical_reason()
                .unwrap_or("unknown status code")
                .into(),
            extra_headers: None,
        }
    }

    pub fn internal_server_error(error: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Unknown,
            error
        )
    }

    pub fn database_error(error: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::DatabaseError,
            error
        )
    }

    pub fn cache_error(error: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::CacheError,
            error
        )
    }

    pub fn rate_limit_exceeded(error: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            ApiErrorCode::RateLimitExceeded,
            error
        )
    }

    pub fn not_found(error: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            ApiErrorCode::ResourceNotFound,
            error
        )
    }

    pub fn bad_request(error: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::BadRequest,
            error
        )
    }

    pub fn with_extra_headers(mut self, headers: HeaderMap) -> Self {
        self.extra_headers = Some(Box::new(headers));
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(mut self) -> axum::response::Response {
        let extra_headers = self.extra_headers.take();
        let mut resp = (self.status_code, Json(self)).into_response();

        if let Some(headers) = extra_headers {
            resp.headers_mut().extend(*headers);
        }

        resp
    }
}