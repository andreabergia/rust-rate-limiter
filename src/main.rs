use std::net::SocketAddr;

use axum::{routing::get, Router};

mod clock;
mod rate_limiter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/", get(say_hello));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn say_hello() -> &'static str {
    "Hello, World!"
}
