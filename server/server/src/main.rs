mod abuse_repository;
mod activity_repository;
mod auth;
mod avatar_repository;
mod backend;
mod bookmark_repository;
mod comment_repository;
mod duckdb;
mod enforcement_repository;
mod feed;
mod group_repository;
mod handlers;
mod hasher;
mod mail;
mod mail_token_repository;
mod mysql;
mod notification_repository;
mod poll_repository;
mod postgres;
mod reaction_repository;
mod repository;
mod search_repository;
mod section_repository;
mod session_repository;
mod topic_repository;

use axum::Router;
use axum::http::{Method, header};
use axum::routing::{delete, get, patch, post};
use backend::{Backend, Vendor};
use handlers::{
    AppState, block_address_handler, commit_topic_handler, create_group_handler,
    create_topic_handler, delete_comment_handler, delete_topic_handler, get_profile_handler,
    get_topic_handler, lift_address_block_handler, list_address_blocks_handler,
    list_comments_handler, list_groups_handler, list_sections_handler, list_topics_by_tag_handler,
    list_topics_handler, move_topic_handler, post_comment_handler, register_handler,
    set_postscore_handler, sign_in_handler, sign_out_handler, uncommit_topic_handler,
    update_bio_handler,
};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

async fn connect(url: &str) -> Arc<dyn Backend> {
    let vendor = Vendor::from_url(url).expect("DATABASE_URL must name a supported vendor");
    println!("storage: {}", vendor.as_str());
    match Vendor::from_url(url) {
        Ok(Vendor::Postgres) => Arc::new(postgres::PostgresBackend::connect(url).await),
        Ok(Vendor::MySql) => Arc::new(mysql::MySqlBackend::connect(url).await),
        Ok(Vendor::DuckDb) => Arc::new(duckdb::DuckDbBackend::connect(url).await),
        Err(_) => panic!("DATABASE_URL must name a supported vendor"),
    }
}

fn limits_from_env() -> app::Limits {
    let mut limits = app::Limits::default();
    if let Ok(raw) = std::env::var("RATE_LIMIT_MAX") {
        if let Ok(value) = raw.parse() {
            limits.rate_limit_max = value;
        }
    }
    if let Ok(raw) = std::env::var("RATE_LIMIT_WINDOW_SECONDS") {
        if let Ok(value) = raw.parse() {
            limits.rate_limit_window = time::Duration::seconds(value);
        }
    }
    if let Ok(raw) = std::env::var("SLOW_MODE_SCORE_FLOOR") {
        if let Ok(value) = raw.parse() {
            limits.slow_mode_score_floor = value;
        }
    }
    if let Ok(raw) = std::env::var("SLOW_MODE_INTERVAL_SECONDS") {
        if let Ok(value) = raw.parse() {
            limits.slow_mode_interval = time::Duration::seconds(value);
        }
    }
    limits
}

fn maintenance_from_env() -> app::MaintenanceSettings {
    let mut settings = app::MaintenanceSettings::default();
    if let Ok(raw) = std::env::var("MAINTENANCE_SCORE_FLOOR") {
        if let Ok(value) = raw.parse() {
            settings.floor = value;
        }
    }
    if let Ok(raw) = std::env::var("CONFIRMATION_WINDOW_SECONDS") {
        if let Ok(value) = raw.parse() {
            settings.confirmation_window = time::Duration::seconds(value);
        }
    }
    settings
}

fn maintenance_interval() -> Option<std::time::Duration> {
    let seconds: u64 = std::env::var("MAINTENANCE_INTERVAL_SECONDS")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(3600);
    if seconds == 0 {
        None
    } else {
        Some(std::time::Duration::from_secs(seconds))
    }
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let state = AppState {
        backend: connect(&database_url).await,
        mailer: mail::build(),
        limits: limits_from_env(),
        maintenance: maintenance_from_env(),
    };
    if let Some(interval) = maintenance_interval() {
        let scheduled = state.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                handlers::run_maintenance_now(&scheduled).await;
            }
        });
    }
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|_origin, _parts| true))
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);
    let app = Router::new()
        .route("/api/register", post(register_handler))
        .route("/api/sign-in", post(sign_in_handler))
        .route("/api/sign-out", post(sign_out_handler))
        .route("/api/me", get(handlers::me_handler))
        .route("/api/me/bio", patch(update_bio_handler))
        .route("/api/users/{username}", get(get_profile_handler))
        .route("/api/sections", get(list_sections_handler))
        .route(
            "/api/sections/{slug}/topics",
            get(list_topics_handler).post(create_topic_handler),
        )
        .route(
            "/api/sections/{slug}/groups",
            get(list_groups_handler).post(create_group_handler),
        )
        .route(
            "/api/topics/{id}",
            get(get_topic_handler).patch(handlers::edit_topic_handler),
        )
        .route("/api/topics/{id}/delete", post(delete_topic_handler))
        .route("/api/topics/{id}/postscore", post(set_postscore_handler))
        .route("/api/topics/{id}/commit", post(commit_topic_handler))
        .route("/api/topics/{id}/uncommit", post(uncommit_topic_handler))
        .route("/api/topics/{id}/move", post(move_topic_handler))
        .route(
            "/api/topics/{id}/comments",
            get(list_comments_handler).post(post_comment_handler),
        )
        .route(
            "/api/topics/{topic_id}/comments/{id}",
            patch(handlers::edit_comment_handler),
        )
        .route(
            "/api/topics/{topic_id}/comments/{id}/delete",
            post(delete_comment_handler),
        )
        .route("/api/tags/{tag}/topics", get(list_topics_by_tag_handler))
        .route("/api/search", get(handlers::search_handler))
        .route("/api/activity", get(handlers::activity_handler))
        .route(
            "/api/sections/{slug}/feed",
            get(handlers::section_feed_handler),
        )
        .route("/api/tags/{tag}/feed", get(handlers::tag_feed_handler))
        .route("/api/me/password", post(handlers::change_password_handler))
        .route("/api/password-reset", post(handlers::request_reset_handler))
        .route(
            "/api/password-reset/confirm",
            post(handlers::reset_password_handler),
        )
        .route(
            "/api/me/email",
            post(handlers::request_email_change_handler),
        )
        .route(
            "/api/me/email/confirm",
            post(handlers::confirm_email_handler),
        )
        .route("/api/activate", post(handlers::confirm_activation_handler))
        .route("/api/me/warnings", get(handlers::my_warnings_handler))
        .route(
            "/api/me/warnings/acknowledge",
            post(handlers::acknowledge_warnings_handler),
        )
        .route(
            "/api/users/{username}/ban",
            post(handlers::ban_user_handler).delete(handlers::lift_ban_handler),
        )
        .route(
            "/api/users/{username}/warn",
            post(handlers::warn_user_handler),
        )
        .route(
            "/api/users/{username}/promote",
            post(handlers::promote_handler),
        )
        .route(
            "/api/users/{username}/ignore",
            get(handlers::ignore_state_handler)
                .post(handlers::ignore_user_handler)
                .delete(handlers::stop_ignoring_handler),
        )
        .route(
            "/api/address-blocks",
            get(list_address_blocks_handler).post(block_address_handler),
        )
        .route(
            "/api/address-blocks/{addr}",
            delete(lift_address_block_handler),
        )
        .route(
            "/api/maintenance/run",
            post(handlers::run_maintenance_handler),
        )
        .route("/api/me/deregister", post(handlers::deregister_handler))
        .route(
            "/api/me/avatar",
            post(handlers::upload_avatar_handler).delete(handlers::delete_avatar_handler),
        )
        .route(
            "/api/users/{username}/avatar",
            get(handlers::get_avatar_handler),
        )
        .route("/api/bookmarks", get(handlers::list_bookmarks_handler))
        .route(
            "/api/topics/{id}/poll",
            get(handlers::get_poll_handler).post(handlers::create_poll_handler),
        )
        .route("/api/topics/{id}/poll/vote", post(handlers::vote_handler))
        .route(
            "/api/topics/{id}/reactions",
            get(handlers::topic_reactions_handler)
                .post(handlers::react_to_topic_handler)
                .delete(handlers::clear_topic_reaction_handler),
        )
        .route(
            "/api/topics/{topic_id}/comments/{id}/reactions",
            get(handlers::comment_reactions_handler)
                .post(handlers::react_to_comment_handler)
                .delete(handlers::clear_comment_reaction_handler),
        )
        .route(
            "/api/topics/{id}/bookmark",
            get(handlers::bookmark_state_handler)
                .post(handlers::add_bookmark_handler)
                .delete(handlers::remove_bookmark_handler),
        )
        .route(
            "/api/notifications",
            get(handlers::list_notifications_handler),
        )
        .route(
            "/api/notifications/unread-count",
            get(handlers::unread_count_handler),
        )
        .route(
            "/api/notifications/{id}/read",
            post(handlers::mark_notification_read_handler),
        )
        .with_state(state)
        .layer(cors);
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("bind listener");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("serve");
}
