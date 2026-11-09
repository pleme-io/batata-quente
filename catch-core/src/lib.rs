//! catch game core — the pure, dependency-light heart of the hot-potato game.
//!
//! This is SPEC.md Pillar 1 (the game-state typescape) + Pillar 2 (the SLA
//! master knob) as plain Rust: the [`FuseState`] FSM, [`derive_ladder`], and the
//! positive-only [`PotatoEvent`] ontology. No I/O, no async — every type here is
//! unit-testable in isolation, which is where catch's "light unit testing"
//! lives. The Postgres OutcomeChain (the durable, BLAKE3-chained log of
//! `PotatoEvent`s) is `catch-store` (the breathe-store leaf-swap, M-phase);
//! this crate defines the leaf type + the in-memory board the M0 service serves.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Where a potato (a PR in review) is on its fuse — a SUM, so a potato is in
/// EXACTLY one phase. "hot AND caught" is unrepresentable (SPEC.md §1.3, Gate 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuseState {
    /// PR ready-for-review, on the board, silent.
    Fresh,
    /// ~50% of the SLA spent — a quiet DM to the holder.
    Warming,
    /// ~75% spent — a channel ping.
    Hot,
    /// ~90% spent — @holder + @author; the potato auto-passes to a backup.
    Scalding,
    /// Past the deadline — the whole team rallies to catch it.
    Critical,
    /// A real review landed — fuse defused, off the board (terminal-good).
    Caught,
    /// The SLA was missed (terminal-bad). Carries NO holder — the queue is the
    /// antagonist, never a colleague (the anti-toxicity invariant, SPEC.md §4.3).
    Exploded,
}

impl FuseState {
    /// The burning rungs, in order. `Caught`/`Exploded` are terminals.
    pub const RUNGS: [FuseState; 5] = [
        FuseState::Fresh,
        FuseState::Warming,
        FuseState::Hot,
        FuseState::Scalding,
        FuseState::Critical,
    ];

    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, FuseState::Caught | FuseState::Exploded)
    }
}

/// A single escalation rung: at `fraction` of the SLA spent, enter `state` and
/// fire `action`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rung {
    pub state: FuseState,
    /// Fraction of the SLA window consumed when this rung fires (0.0..=1.0).
    pub fraction: f64,
    pub action: RungAction,
}

/// What fires at a rung. Notification breadth widens as the fuse burns; the pass
/// is the team mechanic; the explosion is a team event, never individual blame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RungAction {
    /// Nothing visible yet (Fresh).
    Silent,
    /// Quiet DM to the holder (Warming).
    DmHolder,
    /// Channel ping, @holder (Hot).
    PingChannel,
    /// @holder + @author AND auto-pass to a backup reviewer (Scalding).
    PassToBackup,
    /// @here — the team rallies (Critical / past deadline).
    RallyTeam,
}

/// Team attributes that modulate the derived ladder (SPEC.md §2.1). Defaults are
/// the neutral preset; a real team overrides through the shikumi chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamAttrs {
    /// More critical repos compress the ladder (escalate earlier/steeper).
    pub repo_criticality: Criticality,
    /// More catchers ⇒ pass sooner.
    pub reviewer_pool_size: u32,
}

impl Default for TeamAttrs {
    fn default() -> Self {
        Self { repo_criticality: Criticality::Normal, reviewer_pool_size: 3 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Criticality {
    Low,
    Normal,
    CriticalPath,
}

impl Criticality {
    /// Compression factor applied to every rung fraction (criticality escalates
    /// earlier). 1.0 = no compression; < 1.0 pulls every rung in.
    fn compression(self) -> f64 {
        match self {
            Criticality::Low => 1.0,
            Criticality::Normal => 0.9,
            Criticality::CriticalPath => 0.75,
        }
    }
}

/// The escalation ladder — the whole game derived from one SLA + the team attrs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EscalationLadder {
    pub sla: Duration,
    pub rungs: Vec<Rung>,
}

/// THE knob: one SLA `Duration` + team attributes derive the whole ladder
/// (SPEC.md §2.1). Rungs are fractions of the SLA, scaled to its length and
/// compressed by criticality. The shape is the same for a 2-week SLA and a 2-day
/// sprint — only the wall-clock spacing changes.
///
/// Invariants (asserted by the tests, the §2.2 Gate-0 tier): rungs are strictly
/// increasing in fraction, every fraction is in `(0.0, 1.0]`, and `Critical`
/// (the deadline, fraction 1.0) is always last.
#[must_use]
pub fn derive_ladder(sla: Duration, attrs: &TeamAttrs) -> EscalationLadder {
    let c = attrs.repo_criticality.compression();
    // Base fractions for the four pre-deadline rungs; criticality pulls them in.
    // Critical is pinned at the deadline (1.0) regardless of compression.
    let base = [
        (FuseState::Fresh, 0.0, RungAction::Silent),
        (FuseState::Warming, 0.5 * c, RungAction::DmHolder),
        (FuseState::Hot, 0.75 * c, RungAction::PingChannel),
        (FuseState::Scalding, 0.9 * c, RungAction::PassToBackup),
    ];
    let mut rungs: Vec<Rung> = base
        .iter()
        .map(|&(state, fraction, action)| Rung { state, fraction, action })
        .collect();
    rungs.push(Rung { state: FuseState::Critical, fraction: 1.0, action: RungAction::RallyTeam });
    EscalationLadder { sla, rungs }
}

impl EscalationLadder {
    /// The burning state for a fuse that has consumed `elapsed` of its SLA. Picks
    /// the last rung whose fraction has been reached. Never returns a terminal —
    /// `Caught`/`Exploded` come from real review events, not the clock alone.
    #[must_use]
    pub fn state_at(&self, elapsed: Duration) -> FuseState {
        let frac = if self.sla.is_zero() {
            1.0
        } else {
            elapsed.as_secs_f64() / self.sla.as_secs_f64()
        };
        let mut current = FuseState::Fresh;
        for rung in &self.rungs {
            if frac >= rung.fraction {
                current = rung.state;
            }
        }
        current
    }
}

/// A PR on the board (SPEC.md §1.2). `PrRef` is the tenant-scoped, VCS-neutral
/// identity — the multi-tenant axis runs through `workspace`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrRef {
    pub workspace: String,
    pub repo: String,
    pub number: u64,
}

/// A potato on the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Potato {
    pub pr: PrRef,
    pub state: FuseState,
    pub holder: String,
    /// Fraction of the fuse consumed at last observation (0.0..=1.0).
    pub fuse_pct: f64,
}

/// The OutcomeChain leaf (SPEC.md §1.4). The analytics + the awards are pure
/// folds over `Vec<PotatoEvent>`. **Positive-only by construction**: there is no
/// negative-individual event to gamify — `Exploded` carries no holder-blame, so a
/// "slowest reviewer" award is *unauthorable* (SPEC.md §4.3, the anti-toxicity
/// spine). The durable, BLAKE3-chained log of these is `catch-store`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PotatoEvent {
    ReadyForReview { pr: PrRef },
    RungEscalated { pr: PrRef, from: FuseState, to: FuseState },
    Passed { pr: PrRef, from: String, to: String },
    Caught { pr: PrRef, by: String },
    Exploded { pr: PrRef },
    Defused { pr: PrRef },
    BoardClean { workspace: String, streak: u64 },
}

/// The in-memory board the M0 service serves. The durable projection is the
/// Postgres OutcomeChain (`catch-store`, M-phase); this is the M0 source of
/// truth so the basic starter serves a request without a database.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Board {
    pub potatoes: Vec<Potato>,
}

impl Board {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A PR becomes a fresh potato. Idempotent on `PrRef`.
    pub fn ready_for_review(&mut self, pr: PrRef, holder: String) -> PotatoEvent {
        if !self.potatoes.iter().any(|p| p.pr == pr) {
            self.potatoes.push(Potato { pr: pr.clone(), state: FuseState::Fresh, holder, fuse_pct: 0.0 });
        }
        PotatoEvent::ReadyForReview { pr }
    }

    /// A real review lands — the potato is caught, off the board.
    pub fn catch(&mut self, pr: &PrRef, by: String) -> Option<PotatoEvent> {
        let idx = self.potatoes.iter().position(|p| &p.pr == pr)?;
        self.potatoes.remove(idx);
        Some(PotatoEvent::Caught { pr: pr.clone(), by })
    }

    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.potatoes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weeks(n: u64) -> Duration {
        Duration::from_secs(n * 7 * 24 * 60 * 60)
    }

    #[test]
    fn ladder_rungs_are_strictly_increasing_and_critical_is_last() {
        let l = derive_ladder(weeks(2), &TeamAttrs::default());
        // strictly increasing fractions
        for w in l.rungs.windows(2) {
            assert!(w[1].fraction > w[0].fraction, "rung fractions must strictly increase: {w:?}");
        }
        // every fraction in (0,1]; Fresh is the 0.0 base, the rest in (0,1]
        assert_eq!(l.rungs.first().unwrap().fraction, 0.0);
        for r in &l.rungs[1..] {
            assert!(r.fraction > 0.0 && r.fraction <= 1.0, "fraction out of range: {r:?}");
        }
        // Critical (the deadline) is always last, at 1.0
        let last = l.rungs.last().unwrap();
        assert_eq!(last.state, FuseState::Critical);
        assert!((last.fraction - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ladder_shape_is_invariant_to_sla_length() {
        // A 2-day sprint and a 2-week SLA produce the SAME rung shape — only the
        // wall-clock spacing differs (the SLA-master-knob property, SPEC.md §2.1).
        let sprint = derive_ladder(Duration::from_secs(2 * 24 * 3600), &TeamAttrs::default());
        let relaxed = derive_ladder(weeks(2), &TeamAttrs::default());
        let states_of = |l: &EscalationLadder| l.rungs.iter().map(|r| r.state).collect::<Vec<_>>();
        let fracs_of = |l: &EscalationLadder| l.rungs.iter().map(|r| r.fraction).collect::<Vec<_>>();
        assert_eq!(states_of(&sprint), states_of(&relaxed));
        assert_eq!(fracs_of(&sprint), fracs_of(&relaxed));
    }

    #[test]
    fn criticality_compresses_the_ladder() {
        let normal = derive_ladder(weeks(2), &TeamAttrs::default());
        let critical = derive_ladder(
            weeks(2),
            &TeamAttrs { repo_criticality: Criticality::CriticalPath, ..TeamAttrs::default() },
        );
        // the Warming rung fires earlier on a critical-path repo
        let warming = |l: &EscalationLadder| l.rungs.iter().find(|r| r.state == FuseState::Warming).unwrap().fraction;
        assert!(warming(&critical) < warming(&normal));
    }

    #[test]
    fn state_at_walks_the_fuse() {
        let l = derive_ladder(weeks(2), &TeamAttrs::default()); // normal: warming 0.45, hot 0.675, scalding 0.81, critical 1.0
        assert_eq!(l.state_at(Duration::ZERO), FuseState::Fresh);
        assert_eq!(l.state_at(weeks(1)), FuseState::Warming); // 0.5 → past 0.45
        assert_eq!(l.state_at(weeks(2)), FuseState::Critical); // 1.0
    }

    #[test]
    fn exploded_carries_no_holder_blame() {
        // The anti-toxicity spine (SPEC.md §4.3): the only failure event,
        // Exploded, has NO field naming a person — so no predicate over the
        // event stream can build a "slowest reviewer" / shame award. This test
        // pins that the ontology stays positive-only.
        let pr = PrRef { workspace: "acme".into(), repo: "api".into(), number: 7 };
        let ev = PotatoEvent::Exploded { pr: pr.clone() };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(!json.contains("holder"), "Exploded must not carry a holder: {json}");
        assert!(!json.contains("\"by\""), "Exploded must not attribute the miss to a person: {json}");
    }

    #[test]
    fn board_ready_then_catch_clears() {
        let mut b = Board::new();
        let pr = PrRef { workspace: "acme".into(), repo: "api".into(), number: 7 };
        b.ready_for_review(pr.clone(), "alice".into());
        b.ready_for_review(pr.clone(), "alice".into()); // idempotent
        assert_eq!(b.potatoes.len(), 1);
        assert!(!b.is_clean());
        let ev = b.catch(&pr, "bob".into()).unwrap();
        assert!(matches!(ev, PotatoEvent::Caught { by, .. } if by == "bob"));
        assert!(b.is_clean());
    }
}
