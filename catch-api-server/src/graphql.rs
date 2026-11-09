//! Face 2 — GraphQL (async-graphql) over the same `GameService` core. The
//! `PotatoDto` (a `SimpleObject`) is the same typed shape the REST face serves.

use crate::dto::{PotatoDto, PrRefDto};
use crate::service::GameService;
use async_graphql::{Context, EmptySubscription, Object, Schema};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// The current board (the live potatoes).
    async fn board(&self, ctx: &Context<'_>) -> Vec<PotatoDto> {
        let svc = ctx.data_unchecked::<GameService>();
        svc.board().await.into_iter().map(Into::into).collect()
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// A PR becomes a fresh potato on the board.
    async fn ready_for_review(&self, ctx: &Context<'_>, pr: PrRefInput, holder: String) -> bool {
        let svc = ctx.data_unchecked::<GameService>();
        svc.ready_for_review(PrRefDto { workspace: pr.workspace, repo: pr.repo, number: pr.number }.into(), holder)
            .await;
        true
    }

    /// A real review lands — catch the potato, off the board. False if absent.
    async fn catch_potato(&self, ctx: &Context<'_>, pr: PrRefInput, by: String) -> bool {
        let svc = ctx.data_unchecked::<GameService>();
        svc.catch(&PrRefDto { workspace: pr.workspace, repo: pr.repo, number: pr.number }.into(), by)
            .await
            .is_some()
    }
}

#[derive(async_graphql::InputObject)]
pub struct PrRefInput {
    pub workspace: String,
    pub repo: String,
    pub number: u64,
}

pub type CatchSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[must_use]
pub fn schema(svc: GameService) -> CatchSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription).data(svc).finish()
}
