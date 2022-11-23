use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{http::StatusCode, response::IntoResponse, routing::get, Extension, Router};
use clock::MillisecondsUnixClock;
use rate_limiter::{RateLimiter, RequestKey, RequestProcessingResponse, Result};

mod clock;
mod rate_limiter;

type RateLimiterOfMsUnixClock = RateLimiter<MillisecondsUnixClock>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let clock = Arc::new(Mutex::new(MillisecondsUnixClock {}));
    let rate_limiter = RateLimiter::new(clock, 1, 2_000);
    let rate_limiter = Arc::new(Mutex::new(rate_limiter));

    let app = Router::new()
        .route("/", get(say_hello))
        .layer(Extension(rate_limiter));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn say_hello(
    Extension(rate_limiter): Extension<Arc<Mutex<RateLimiterOfMsUnixClock>>>,
) -> Result<impl IntoResponse> {
    let address = RequestKey::new("todo");
    let result = rate_limiter.lock()?.try_add_request(address)?;
    match result {
        RequestProcessingResponse::Allow => Ok(StatusCode::NO_CONTENT),
        RequestProcessingResponse::Deny => Ok(StatusCode::TOO_MANY_REQUESTS),
    }
}
