//! Route assembly.

pub mod api_keys;
pub mod app_settings;
pub mod auth;
pub mod bookmarks;
pub mod cache;
pub mod config;
pub mod content;
pub mod crunchyroll;
pub mod downloads;
pub mod search;
pub mod tracking;
pub mod ws;

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::docs::ApiDoc;
use crate::state::AppState;

/// Build the complete API router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(auth::router())
        .merge(api_keys::router())
        .merge(bookmarks::router())
        .merge(tracking::router())
        .merge(app_settings::router())
        .merge(crunchyroll::router())
        .merge(search::router())
        .merge(content::router())
        .merge(downloads::router())
        .merge(cache::router())
        .merge(config::router())
        .merge(ws::router())
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}