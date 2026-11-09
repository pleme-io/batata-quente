//! catch API server — the `Arc<GameService>` core projected by the THREE
//! faces (URDUME public-service mandate): REST (axum) + code-first OpenAPI/Swagger
//! (utoipa) on `/swagger` + `/api-docs/openapi.json`; GraphQL (async-graphql) on
//! `/graphql`; gRPC (tonic) `BoardService` on the gRPC port. Plus the standard
//! `/health/{startup,live,ready}` + `/metrics`, and the inbound webhook intake.
//!
//! M0 (DEPLOY.md): in-memory board, no DB; the SeaORM/`PgOutcomeChain` durable
//! store and the fuse-advancing reconcile loop are the M-phase.

mod dto;
mod graphql;
mod grpc;
mod rest;
mod service;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use catch_core::PrRef;
use service::GameService;
use std::fmt::Write as _;
use utoipa_swagger_ui::SwaggerUi;

/// Build the full HTTP app (the REST face + Swagger UI + GraphQL + health +
/// metrics + webhook). Shared by `main` and the tests so the wiring is exercised.
fn build_app(svc: GameService) -> Router {
    let (rest_router, api) = rest::router();
    let schema = graphql::schema(svc.clone());

    let extra = Router::<GameService>::new()
        .route("/health/startup", get(|| async { StatusCode::OK }))
        .route("/health/live", get(|| async { StatusCode::OK }))
        .route("/health/ready", get(|| async { StatusCode::OK }))
        .route("/metrics", get(metrics))
        .route("/api/webhooks/github", post(github_webhook))
        .route_service("/graphql", async_graphql_axum::GraphQL::new(schema));

    rest_router
        .merge(extra)
        .with_state(svc)
        .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", api))
}

/// Game-native prometheus gauges (DEPLOY.md §6) — workspace/repo-labeled, NO
/// holder label (the anti-toxicity invariant). Hand-rolled text exposition keeps
/// the M0 starter dependency-light; the metric *names* are the contract the
/// pangea-dashboards `Monitorable` reads.
async fn metrics(State(svc): State<GameService>) -> impl IntoResponse {
    let active = svc.active_count().await;
    let mut out = String::new();
    let _ = writeln!(out, "# HELP catch_build_info Build metadata.");
    let _ = writeln!(out, "# TYPE catch_build_info gauge");
    let _ = writeln!(out, "catch_build_info{{version=\"{}\"}} 1", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(out, "# HELP catch_potatoes_active PRs currently on the board.");
    let _ = writeln!(out, "# TYPE catch_potatoes_active gauge");
    let _ = writeln!(out, "catch_potatoes_active {active}");
    ([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], out)
}

/// The inbound webhook intake (DEPLOY.md §5 — the actual M0 data-plane entry).
/// M0 stub: a GitHub `pull_request` ready/opened event records a fresh potato.
/// The full typed `VcsPlatform::ingest_webhook` (SPEC.md Pillar 3) is the M-phase.
#[derive(serde::Deserialize)]
struct GithubPrWebhook {
    action: String,
    pull_request: GithubPr,
    repository: GithubRepo,
    #[serde(default)]
    organization: Option<GithubOrg>,
}
#[derive(serde::Deserialize)]
struct GithubPr {
    number: u64,
    user: GithubUser,
}
#[derive(serde::Deserialize)]
struct GithubRepo {
    name: String,
}
#[derive(serde::Deserialize)]
struct GithubOrg {
    login: String,
}
#[derive(serde::Deserialize)]
struct GithubUser {
    login: String,
}

async fn github_webhook(State(svc): State<GameService>, Json(hook): Json<GithubPrWebhook>) -> StatusCode {
    if hook.action == "ready_for_review" || hook.action == "opened" {
        let workspace = hook.organization.map_or_else(|| "default".to_string(), |o| o.login);
        let pr = PrRef { workspace, repo: hook.repository.name, number: hook.pull_request.number };
        svc.ready_for_review(pr, hook.pull_request.user.login).await;
    }
    StatusCode::ACCEPTED
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,catch_api_server=info".into()),
        )
        .init();

    let svc = GameService::new();

    // Face 3 — gRPC on its own port (HTTP/2).
    let grpc_bind = std::env::var("CATCH_GRPC_BIND").unwrap_or_else(|_| "0.0.0.0:50051".into());
    let grpc_addr = grpc_bind.parse().expect("CATCH_GRPC_BIND");
    let grpc_svc = svc.clone();
    tokio::spawn(async move {
        tracing::info!(%grpc_bind, "catch gRPC BoardService listening");
        if let Err(e) = tonic::transport::Server::builder()
            .add_service(grpc::server(grpc_svc))
            .serve(grpc_addr)
            .await
        {
            tracing::error!(error = %e, "gRPC server exited");
        }
    });

    // Faces 1 + 2 + health/metrics on the HTTP port.
    let bind = std::env::var("CATCH_API_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind).await.expect("bind");
    tracing::info!(%bind, "catch-api-server listening (REST + GraphQL + Swagger)");
    axum::serve(listener, build_app(svc)).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    #[tokio::test]
    async fn health_ready_is_200() {
        let resp = build_app(GameService::new())
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn openapi_doc_is_served_and_documents_the_board() {
        let resp = build_app(GameService::new())
            .oneshot(Request::builder().uri("/api-docs/openapi.json").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(doc["info"]["title"], "catch");
        assert!(doc["paths"]["/api/board"].is_object(), "OpenAPI must document GET /api/board");
        assert!(doc["components"]["schemas"]["PotatoDto"].is_object());
    }

    #[tokio::test]
    async fn metrics_exposes_the_game_native_gauges() {
        let resp = build_app(GameService::new())
            .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("catch_potatoes_active 0"));
    }

    #[tokio::test]
    async fn webhook_then_board_reflects_the_potato_across_the_rest_face() {
        let svc = GameService::new();
        let app = build_app(svc.clone());
        let payload = serde_json::json!({
            "action": "ready_for_review",
            "pull_request": { "number": 42, "user": { "login": "alice" } },
            "repository": { "name": "api" },
            "organization": { "login": "acme" }
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/webhooks/github")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        // the REST board face now reflects it
        let resp = build_app(svc)
            .oneshot(Request::builder().uri("/api/board").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let board: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(board[0]["pr"]["number"], 42);
        assert_eq!(board[0]["pr"]["workspace"], "acme");
        assert_eq!(board[0]["state"], "fresh");
    }
}
