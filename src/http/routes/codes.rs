use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::Response;
use axum::routing::get;
use tower_governor::GovernorLayer;
use tower_governor::errors::GovernorError;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::KeyExtractor;

use crate::config::{ApiConfig, ProxyCidr};
use crate::database::redemption_code::{RedemptionCode, RedemptionCodeResponse};
use crate::games::Game;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};
use crate::http::routes::json_response;

#[derive(Clone)]
struct ClientIp {
    trusted_proxies: Arc<[ProxyCidr]>,
}

impl ClientIp {
    fn trusts(&self, peer: Option<IpAddr>) -> bool {
        self.trusted_proxies.is_empty()
            || peer.is_some_and(|ip| self.trusted_proxies.iter().any(|cidr| cidr.contains(ip)))
    }
}

impl KeyExtractor for ClientIp {
    type Key = String;

    fn extract<T>(&self, req: &axum::http::Request<T>) -> Result<Self::Key, GovernorError> {
        let headers = req.headers();
        let peer = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip());

        let forwarded = self
            .trusts(peer)
            .then(|| {
                headers
                    .get("CF-Connecting-IP")
                    .or_else(|| headers.get("X-Real-IP"))
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        headers
                            .get("X-Forwarded-For")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.split(',').next())
                            .map(|s| s.trim().to_string())
                    })
            })
            .flatten();

        forwarded
            .or_else(|| peer.map(|ip| ip.to_string()))
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

pub fn routes(api: &ApiConfig) -> Router<Arc<Global>> {
    if api.trusted_proxies.is_empty() {
        tracing::warn!("api.trusted_proxies is empty, client IP headers are trusted from any peer");
    }

    let governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(api.rate_limit.per_second)
            .burst_size(api.rate_limit.burst_size)
            .key_extractor(ClientIp {
                trusted_proxies: api.trusted_proxies.clone().into(),
            })
            .finish()
            .unwrap(),
    );

    Router::new()
        .route("/:game/codes", get(get_codes))
        .layer(GovernorLayer { config: governor })
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
) -> Result<Response<Body>, ApiError> {
    let game = Game::from_slug(&game_slug)
        .ok_or_else(|| ApiError::not_found(ApiErrorCode::UNKNOWN_GAME, "unknown game"))?;

    let cache_key = game.codes_path();

    let bytes = global
        .response_cache
        .get_or_try_insert(cache_key, async {
            let all_codes = RedemptionCode::find_all(&global.db, game)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "failed to query codes");
                    ApiError::internal_server_error(
                        ApiErrorCode::DATABASE_ERROR,
                        "failed to query codes",
                    )
                })?;

            let (active, inactive): (Vec<_>, Vec<_>) =
                all_codes.into_iter().partition(|c| c.active);
            let response = CodesResponse {
                active: active.into_iter().map(Into::into).collect(),
                inactive: inactive.into_iter().map(Into::into).collect(),
            };

            Ok(Bytes::from(
                serde_json::to_vec(&response).expect("CodesResponse is always serializable"),
            ))
        })
        .await?;

    Ok(json_response(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(peer: &str, headers: &[(&str, &str)]) -> axum::http::Request<()> {
        let mut builder = axum::http::Request::builder();
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let mut req = builder.body(()).unwrap();
        req.extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
        req
    }

    fn extractor(trusted: &[&str]) -> ClientIp {
        ClientIp {
            trusted_proxies: trusted.iter().map(|s| s.parse().unwrap()).collect(),
        }
    }

    #[test]
    fn untrusted_peer_cannot_spoof_client_ip() {
        let key = extractor(&["173.245.48.0/20"])
            .extract(&request(
                "203.0.113.9:5000",
                &[("CF-Connecting-IP", "1.2.3.4")],
            ))
            .unwrap();

        assert_eq!(key, "203.0.113.9");
    }

    #[test]
    fn trusted_peer_forwards_client_ip() {
        let key = extractor(&["173.245.48.0/20"])
            .extract(&request(
                "173.245.48.7:443",
                &[("CF-Connecting-IP", "1.2.3.4")],
            ))
            .unwrap();

        assert_eq!(key, "1.2.3.4");
    }

    #[test]
    fn empty_trust_list_keeps_header_behavior() {
        let key = extractor(&[])
            .extract(&request(
                "203.0.113.9:5000",
                &[("X-Forwarded-For", "5.6.7.8, 10.0.0.1")],
            ))
            .unwrap();

        assert_eq!(key, "5.6.7.8");
    }
}
