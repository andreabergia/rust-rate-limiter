use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{http::StatusCode, response::IntoResponse, routing::get, Extension, Router};
use clock::UnixEpochMillisecondsClock;
use error::Result;
use rate_limiter::{RateLimiter, RequestProcessingResponse, SourceAddress};

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
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn say_hello_rate_limited(
    Extension(rate_limiter): Extension<Arc<Mutex<RateLimiterOfUnixEpochMsClock>>>,
) -> Result<impl IntoResponse> {
    // TODO: extract source address
    let address = SourceAddress::new("todo");
    let result = rate_limiter.lock()?.try_add_request(address)?;
    match result {
        RequestProcessingResponse::Allow => Ok((StatusCode::OK, "Hello!").into_response()),
        RequestProcessingResponse::Deny => Ok(StatusCode::TOO_MANY_REQUESTS.into_response()),
    }
}
