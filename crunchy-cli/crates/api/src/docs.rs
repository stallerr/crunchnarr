//! OpenAPI spec generation and Swagger UI serving.

use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi;
use utoipa::{Modify, OpenApi};

use crate::error::ErrorBody;
use crate::routes::{
    api_keys::{ApiKeyItem, CreateApiKeyRequest, CreateApiKeyResponse},
    auth::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest, UserResponse},
    bookmarks::{Bookmark, BookmarkItem, CreateBookmarkRequest, UpdateBookmarkRequest},
    cache::CleanParams,
    crunchyroll::{CrLoginRequest, CrLoginResponse},
    downloads::{
        DownloadResponse, DownloadedEpisodeIds, MarkManualBulkRequest, MarkManualRequest,
        MarkManualResponse, StartDownloadRequest,
    },
    search::SearchParams,
    tracking::{AddTrackingRequest, TrackedSeriesItem, UpdateTrackingRequest},
};
use crate::services::tracking::CheckSummary;

/// OpenAPI spec for crunchy-api.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Crunchy API",
        version = "0.1.0",
        description = "REST + WebSocket API for managing Crunchyroll downloads"
    ),
    paths(
        // Auth
        crate::routes::auth::register,
        crate::routes::auth::login,
        crate::routes::auth::refresh,
        crate::routes::auth::me,
        // API Keys
        crate::routes::api_keys::list_keys,
        crate::routes::api_keys::create_key,
        crate::routes::api_keys::revoke_key,
        // Bookmarks
        crate::routes::bookmarks::list_bookmarks,
        crate::routes::bookmarks::create_bookmark,
        crate::routes::bookmarks::remove_bookmark,
        crate::routes::bookmarks::update_bookmark_note,
        // Watchlist
        crate::routes::tracking::list_tracked,
        crate::routes::tracking::add_tracked,
        crate::routes::tracking::update_tracked,
        crate::routes::tracking::delete_tracked,
        crate::routes::tracking::check_tracked,
        // Crunchyroll
        crate::routes::crunchyroll::cr_login,
        crate::routes::crunchyroll::cr_logout,
        crate::routes::crunchyroll::cr_whoami,
        // Search
        crate::routes::search::search,
        // Content
        crate::routes::content::get_series,
        crate::routes::content::get_seasons,
        crate::routes::content::get_episodes,
        crate::routes::content::get_episode,
        // Downloads
        crate::routes::downloads::start_download,
        crate::routes::downloads::list_downloads,
        crate::routes::downloads::get_download,
        crate::routes::downloads::cancel_download,
        crate::routes::downloads::pause_download,
        crate::routes::downloads::resume_download,
        crate::routes::downloads::downloaded_episode_ids,
        crate::routes::downloads::mark_manual,
        crate::routes::downloads::mark_manual_bulk,
        crate::routes::downloads::unmark_manual,
        // Cache
        crate::routes::cache::list_caches,
        crate::routes::cache::clean_caches,
        crate::routes::cache::cache_stats,
        // Config
        crate::routes::config::get_config,
        crate::routes::config::update_config,
        crate::routes::config::reset_config,
        // WebSocket
        crate::routes::ws::ws_handler,
    ),
    components(schemas(
        ErrorBody,
        RegisterRequest,
        LoginRequest,
        RefreshRequest,
        AuthResponse,
        UserResponse,
        ApiKeyItem,
        CreateApiKeyRequest,
        CreateApiKeyResponse,
        Bookmark,
        BookmarkItem,
        CreateBookmarkRequest,
        UpdateBookmarkRequest,
        TrackedSeriesItem,
        AddTrackingRequest,
        UpdateTrackingRequest,
        CheckSummary,
        CrLoginRequest,
        CrLoginResponse,
        SearchParams,
        StartDownloadRequest,
        DownloadResponse,
        DownloadedEpisodeIds,
        MarkManualRequest,
        MarkManualBulkRequest,
        MarkManualResponse,
        CleanParams,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "User authentication (register, login, refresh, profile)"),
        (name = "API Keys", description = "Per-user API keys for non-interactive auth"),
        (name = "Bookmarks", description = "Per-user saved series"),
        (name = "Watchlist", description = "Series tracking and auto-download"),
        (name = "Crunchyroll", description = "Crunchyroll account linking"),
        (name = "Search", description = "Search Crunchyroll catalog"),
        (name = "Content", description = "Browse series, seasons, and episodes"),
        (name = "Downloads", description = "Manage active downloads"),
        (name = "Cache", description = "Download cache management"),
        (name = "Config", description = "User configuration"),
        (name = "WebSocket", description = "Real-time event stream"),
    )
)]
pub struct ApiDoc;

/// Adds Bearer auth security scheme to the OpenAPI spec.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Api-Key"))),
            );
        }
    }
}
