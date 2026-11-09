//! Shared DTOs — ONE typed shape projected onto every face: serde (the wire),
//! utoipa `ToSchema` (the OpenAPI/Swagger doc, code-first), and async-graphql
//! `SimpleObject` (the GraphQL type). The gRPC face maps these to its proto
//! messages. "The core data structures derive the faces" (URDUME mandate).

use async_graphql::SimpleObject;
use catch_core::{FuseState, Potato, PrRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, SimpleObject)]
pub struct PrRefDto {
    pub workspace: String,
    pub repo: String,
    pub number: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, SimpleObject)]
pub struct PotatoDto {
    pub pr: PrRefDto,
    /// FuseState, snake_case (fresh | warming | hot | scalding | critical).
    pub state: String,
    pub holder: String,
    pub fuse_pct: f64,
}

#[must_use]
pub fn fuse_state_str(s: FuseState) -> &'static str {
    match s {
        FuseState::Fresh => "fresh",
        FuseState::Warming => "warming",
        FuseState::Hot => "hot",
        FuseState::Scalding => "scalding",
        FuseState::Critical => "critical",
        FuseState::Caught => "caught",
        FuseState::Exploded => "exploded",
    }
}

impl From<PrRef> for PrRefDto {
    fn from(p: PrRef) -> Self {
        Self { workspace: p.workspace, repo: p.repo, number: p.number }
    }
}

impl From<PrRefDto> for PrRef {
    fn from(p: PrRefDto) -> Self {
        Self { workspace: p.workspace, repo: p.repo, number: p.number }
    }
}

impl From<Potato> for PotatoDto {
    fn from(p: Potato) -> Self {
        Self {
            pr: p.pr.into(),
            state: fuse_state_str(p.state).to_string(),
            holder: p.holder,
            fuse_pct: p.fuse_pct,
        }
    }
}
