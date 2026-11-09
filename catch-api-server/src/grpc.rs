//! Face 3 — gRPC (tonic) over the same `GameService` core. The proto messages
//! are the gRPC projection of the shared DTOs (proto/catch.proto).

use crate::dto::fuse_state_str;
use crate::service::GameService;
use catch_core::PrRef;
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("catch.v1");
}

use pb::board_service_server::{BoardService, BoardServiceServer};

#[derive(Clone)]
pub struct GrpcBoard {
    pub svc: GameService,
}

fn to_core(pr: Option<pb::PrRef>) -> Result<PrRef, Status> {
    let pr = pr.ok_or_else(|| Status::invalid_argument("pr is required"))?;
    Ok(PrRef { workspace: pr.workspace, repo: pr.repo, number: pr.number })
}

#[tonic::async_trait]
impl BoardService for GrpcBoard {
    async fn get_board(
        &self,
        req: Request<pb::GetBoardRequest>,
    ) -> Result<Response<pb::GetBoardResponse>, Status> {
        let ws = req.into_inner().workspace;
        let potatoes = self
            .svc
            .board()
            .await
            .into_iter()
            .filter(|p| ws.is_empty() || p.pr.workspace == ws)
            .map(|p| pb::Potato {
                pr: Some(pb::PrRef { workspace: p.pr.workspace, repo: p.pr.repo, number: p.pr.number }),
                state: fuse_state_str(p.state).to_string(),
                holder: p.holder,
                fuse_pct: p.fuse_pct,
            })
            .collect();
        Ok(Response::new(pb::GetBoardResponse { potatoes }))
    }

    async fn ready_for_review(
        &self,
        req: Request<pb::ReadyForReviewRequest>,
    ) -> Result<Response<pb::MutationAck>, Status> {
        let r = req.into_inner();
        self.svc.ready_for_review(to_core(r.pr)?, r.holder).await;
        Ok(Response::new(pb::MutationAck { accepted: true }))
    }

    async fn catch(
        &self,
        req: Request<pb::CatchRequest>,
    ) -> Result<Response<pb::MutationAck>, Status> {
        let r = req.into_inner();
        let accepted = self.svc.catch(&to_core(r.pr)?, r.by).await.is_some();
        Ok(Response::new(pb::MutationAck { accepted }))
    }
}

#[must_use]
pub fn server(svc: GameService) -> BoardServiceServer<GrpcBoard> {
    BoardServiceServer::new(GrpcBoard { svc })
}
