//! The `GameService` core — the ONE `Arc<Service>` the three faces (REST,
//! GraphQL, gRPC) project (URDUME: three transports = projections of one core).
//! M0 holds the board in memory; the M-phase swaps in the SeaORM/`PgOutcomeChain`
//! durable store behind the same method surface.

use catch_core::{Board, Potato, PotatoEvent, PrRef};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct GameService {
    board: Arc<RwLock<Board>>,
}

impl GameService {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn board(&self) -> Vec<Potato> {
        self.board.read().await.potatoes.clone()
    }

    pub async fn ready_for_review(&self, pr: PrRef, holder: String) -> PotatoEvent {
        self.board.write().await.ready_for_review(pr, holder)
    }

    pub async fn catch(&self, pr: &PrRef, by: String) -> Option<PotatoEvent> {
        self.board.write().await.catch(pr, by)
    }

    pub async fn active_count(&self) -> usize {
        self.board.read().await.potatoes.len()
    }
}
