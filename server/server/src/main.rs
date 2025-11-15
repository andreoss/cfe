mod auth;
mod comment_repository;
mod handlers;
mod feed;
mod hasher;
mod bookmark_repository;
mod notification_repository;
mod reaction_repository;
mod poll_repository;
mod repository;
mod search_repository;
mod section_repository;
mod session_repository;
mod topic_repository;

use axum::Router;
use axum::http::{Method, header};
use axum::routing::{get, patch, post};
use handlers::{
    AppState, create_topic_handler, delete_comment_handler, delete_topic_handler,
    get_profile_handler, get_topic_handler, list_comments_handler, list_sections_handler,
    list_topics_by_tag_handler, list_topics_handler, post_comment_handler, register_handler,
    sign_in_handler, sign_out_handler, update_bio_handler,
};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, CorsLayer};

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect to database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let state = AppState { pool };
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
            "/api/topics/{id}",
            get(get_topic_handler).patch(handlers::edit_topic_handler),
        )
        .route("/api/topics/{id}/delete", post(delete_topic_handler))
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
        .route(
            "/api/sections/{slug}/feed",
            get(handlers::section_feed_handler),
        )
        .route("/api/tags/{tag}/feed", get(handlers::tag_feed_handler))
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
    axum::serve(listener, app).await.expect("serve");
}
