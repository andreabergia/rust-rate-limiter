use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    extract::ConnectInfo, http::StatusCode, response::IntoResponse, routing::get, Extension, Router,
};
use clock::UnixEpochMillisecondsClock;
use error::Result;
use rate_limiter::{RateLimiter, RequestKey, RequestProcessingResponse};
use tracing::info;

mod clock;
mod error;
mod rate_limiter;

type RateLimiterOfUnixEpochMsClock = RateLimiter<UnixEpochMillisecondsClock>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let clock = Arc::new(Mutex::new(UnixEpochMillisecondsClock {}));
    let rate_limiter = RateLimiter::new(clock, 1, 2_000);
    let rate_limiter = Arc::new(Mutex::new(rate_limiter));

    let app = Router::new()
        .route("/", get(say_hello_rate_limited))
        .layer(Extension(rate_limiter));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn say_hello_rate_limited(
    Extension(rate_limiter): Extension<Arc<Mutex<RateLimiterOfUnixEpochMsClock>>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse> {
    let address = RequestKey::new(&format!("{}", addr.ip()));
    let result = rate_limiter.lock()?.add_request(address)?;
    info!("request from client {}: {:?}", addr, result);
    match result {
        RequestProcessingResponse::Allow => Ok((StatusCode::OK, "Hello!").into_response()),
        RequestProcessingResponse::Deny => Ok(StatusCode::TOO_MANY_REQUESTS.into_response()),
    }
}
