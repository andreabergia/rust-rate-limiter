use std::sync::PoisonError;

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RateLimiterError {
    #[error("threading problem")]
    ThreadingProblem,
}

pub type Result<T> = std::result::Result<T, RateLimiterError>;

impl<C> From<PoisonError<C>> for RateLimiterError {
    fn from(_: PoisonError<C>) -> Self {
        Self::ThreadingProblem
    }
}

#[derive(Serialize)]
struct Message {
    message: String,
}

impl IntoResponse for RateLimiterError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            RateLimiterError::ThreadingProblem => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(Message {
            message: format!("{}", self),
        });
        (status_code, body).into_response()
    }
}
