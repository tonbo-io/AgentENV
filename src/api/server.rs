use axum::{http::StatusCode, middleware, routing::get, Router};

use super::{impls::auth, proxy, ApiImpl};
use crate::observability::prometheus;
use crate::sandbox::UblkDeviceManager;
use agentenv_http_server::apis;
use agentenv_observability::metrics_handler;

async fn readiness() -> StatusCode {
    readiness_status(UblkDeviceManager::try_global().is_some_and(UblkDeviceManager::is_available))
}

fn readiness_status(ublk_available: bool) -> StatusCode {
    if ublk_available {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_fails_closed_when_ublk_is_unavailable() {
        assert_eq!(readiness_status(true), StatusCode::NO_CONTENT);
        assert_eq!(readiness_status(false), StatusCode::SERVICE_UNAVAILABLE);
    }
}

pub fn new<I, A, E, C>(api_impl: I) -> Router
where
    I: AsRef<A> + AsRef<ApiImpl> + Clone + Send + Sync + 'static,
    A: apis::admin::Admin<E, Claims = C>
        + apis::default::Default<E>
        + apis::sandboxes::Sandboxes<E, Claims = C>
        + apis::snapshots::Snapshots<E, Claims = C>
        + apis::templates::Templates<E, Claims = C>
        + apis::ApiKeyAuthHeader<Claims = C>
        + apis::ApiAuthBasic<Claims = C>
        + Send
        + Sync
        + 'static,
    E: std::fmt::Debug + Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    // Keep the generated control-plane API as the primary router, then merge in
    // the hand-written `/proxy/*` entrypoints needed for the temporary reverse
    // proxy contract.
    agentenv_http_server::server::new::<I, A, E, C>(api_impl.clone())
        .merge(proxy::router(api_impl.clone()))
        .route("/metrics", get(metrics_handler))
        .route("/ready", get(readiness))
        .layer(middleware::from_fn_with_state(
            api_impl.clone(),
            proxy::sandbox_proxy_classifier::<I>,
        ))
        .layer(middleware::from_fn_with_state(
            api_impl,
            auth::require_auth::<I>,
        ))
        .layer(middleware::from_fn(prometheus::http_metrics_middleware))
}
