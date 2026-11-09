//! Face 1 — REST (axum) with code-first OpenAPI (utoipa). The `#[utoipa::path]`
//! annotations on the typed handlers ARE the source of the served OpenAPI
//! document (`/api-docs/openapi.json` + the Swagger UI mounted in `main`), so
//! the Rust types are the source and the spec is emitted from them (URDUME
//! public-service mandate — the resolution of the OpenAPI-as-source inversion).

use crate::dto::{PotatoDto, PrRefDto};
use crate::service::GameService;
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "catch",
        version = "0.1.0",
        description = "catch — the team PR-review hot-potato game. The REST face; the same core is served at /graphql (GraphQL) and over the gRPC BoardService."
    ),
    components(schemas(PotatoDto, PrRefDto, ReadyBody, CatchBody)),
    tags((name = "board", description = "The hot-potato board"))
)]
pub struct ApiDoc;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReadyBody {
    pub pr: PrRefDto,
    pub holder: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CatchBody {
    pub pr: PrRefDto,
    pub by: String,
}

#[utoipa::path(
    get, path = "/api/board", tag = "board",
    responses((status = 200, description = "The current board", body = [PotatoDto]))
)]
async fn get_board(State(svc): State<GameService>) -> Json<Vec<PotatoDto>> {
    Json(svc.board().await.into_iter().map(Into::into).collect())
}

#[utoipa::path(
    post, path = "/api/board/ready", tag = "board",
    request_body = ReadyBody,
    responses((status = 202, description = "PR is now a fresh potato on the board"))
)]
async fn ready_for_review(State(svc): State<GameService>, Json(b): Json<ReadyBody>) -> StatusCode {
    svc.ready_for_review(b.pr.into(), b.holder).await;
    StatusCode::ACCEPTED
}

#[utoipa::path(
    post, path = "/api/board/catch", tag = "board",
    request_body = CatchBody,
    responses(
        (status = 202, description = "Potato caught, off the board"),
        (status = 404, description = "No such potato on the board")
    )
)]
async fn catch_potato(State(svc): State<GameService>, Json(b): Json<CatchBody>) -> StatusCode {
    match svc.catch(&b.pr.into(), b.by).await {
        Some(_) => StatusCode::ACCEPTED,
        None => StatusCode::NOT_FOUND,
    }
}

/// The documented REST face as a utoipa router; `split_for_parts` yields the
/// axum `Router<GameService>` + the `OpenApi` doc the Swagger UI serves.
#[must_use]
pub fn router() -> (axum::Router<GameService>, utoipa::openapi::OpenApi) {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(get_board))
        .routes(routes!(ready_for_review))
        .routes(routes!(catch_potato))
        .split_for_parts()
}
