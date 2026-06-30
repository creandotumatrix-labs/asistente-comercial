pub mod transcript;
pub mod webhook;

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/webhook", get(webhook::verify).post(webhook::receive))
        .route("/conversations/:id/transcript", get(transcript::show))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn health() -> &'static str {
    "ok"
}
