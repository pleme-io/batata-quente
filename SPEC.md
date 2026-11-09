# Batata-Quente — the typed spec

> Companion to [`DESIGN.md`](./DESIGN.md). DESIGN.md states *what* and *why*;
> this file states the *typed contract* — the four pillars as Rust+Lisp typed
> domains, the data model, the generation map, and the milestones.
>
> Status: spec / codification (2026-06-17). Recon-validated; not yet built.
> Naming: `batata-quente` is the codename (BR-PT *hot potato*, Tier-2 convention).

This spec obeys the pleme-io org directives by construction. The load-bearing
ones for batata-quente, named up front so every section below can cite them:

- **★★ TYPED-SPEC + INTERPRETER TRIPLET** — every algorithmic primitive (the
  fuse FSM, the ladder derivation, each integration adapter) ships as *typed
  Rust border + authored Lisp spec + interpreter behind an `Environment` trait*.
  The traits in Pillar 3 **are** that Environment trait — Slack/GitHub side
  effects behind a mockable boundary, so the engine tests without live network.
- **★★ UNREPRESENTABILITY** — each pillar runs Gate 0 ("name the illegal state
  this domain admits") and grades its tier honestly (`truly-unrepresentable` /
  `parse-time-rejected` / `only-mitigated`). No rounding up.
- **★★ TYPED EMISSION** — every byte every adapter emits (Block Kit JSON,
  Adaptive Card, Discord embed, the rendered ladder YAML) goes through a typed
  AST + serializer. `format!()` of any target syntax is forbidden.
- **★★ CATALOG REFLECTION** — the typed domains self-describe via a meta-spec;
  adding a domain requires its catalog entry in the same commit.
- **★ Rust + Lisp** — every typed domain is `#[derive(TataraDomain)]` +
  `(def… )` authoring form.
- **Urdume / Tela** — the SaaS shell is an Urdume `tecido` + a Tela face
  (DESIGN.md §8); this spec details L0/L1/L3/L9 (the game-specific tramas).

---

## 0. The shape in one diagram

```
                         ┌─────────────────────────────────────────┐
   one typed game spec   │  Pillar 1  game-state typescape          │
   ───────────────────►  │    (defgame) Potato · FuseState FSM      │
                         │    OutcomeChain (BLAKE3 append-only)     │ ◄── breathe-store
                         └───────────────┬─────────────────────────┘     chain pattern
                                         │ events
        ┌────────────────────────────────┼────────────────────────────────┐
        ▼                                ▼                                  ▼
┌──────────────────┐         ┌────────────────────────┐        ┌────────────────────┐
│ Pillar 2  config │         │ Pillar 3  integration  │        │ Pillar 4 analytics │
│  derive_ladder   │         │   ChatPlatform ×       │        │  + (defaward)      │
│  (sla, attrs)    │         │   VcsPlatform traits   │        │  predicate engine  │
│  EscalationLadder│         │  (the Environment      │        │  views over the    │
│  shikumi tiers   │         │   trait — mockable)    │        │  OutcomeChain      │
└────────┬─────────┘         └───────────┬────────────┘        └─────────┬──────────┘
         │                               │                               │
         └───────────────► the reconcile loop (PromessaController-shaped) ◄┘
                                         │
                            forge-gen  ──┴──►  REST · gRPC · GraphQL · MCP · SDKs · docs
                            Tela       ──────►  board · leaderboard · heat-map · award-shelf
```

The reconcile loop is the **Seven-Beat Convergence Tick** of a Viggy
`PromessaController` (Observe → Diff → Classify → Decide → Act → Attest → Tick);
Pillars 1–4 are the typed data it folds over. Nothing here is a new control
primitive — it is the `Reconciler` trait + `shigoto::Dag` + the BLAKE3 chain
(breathe-store) + the iac-forge `Backend` trait, recomposed for a new domain.

---

## Pillar 1 — the game-state typescape

The single source of truth. Every other pillar is a function of it.

### 1.1 The authoring form

```lisp
(defgame batata-quente
  :board     (potatoes …)            ; the live set of PRs-in-review
  :fuse-fsm  fresh→warming→hot→scalding→critical→{caught|exploded}
  :events    outcome-chain)          ; the append-only attested log
```

### 1.2 The typed Rust border

```rust
#[derive(TataraDomain, Serialize, Deserialize, Debug, Clone)]
#[tatara(keyword = "defgame")]
pub struct GameSpec { /* board ref + fuse-fsm ref + chain ref */ }

/// A PR on the board. PrRef is the tenant-scoped, VCS-neutral identity.
pub struct Potato {
    pub pr: PrRef,                 // (workspace, repo, number) — the tenant axis runs through here
    pub state: FuseState,          // the FSM position (see 1.3)
    pub holder: Reviewer,          // current responsible reviewer
    pub fuse: Fuse,                // remaining SLA budget (Pillar 2 owns derivation)
    pub passes: PassChain,         // append-only record of reassignments
}

/// FuseState is a SUM (sum-over-product, UNREPRESENTABILITY catalog) — a potato
/// is in EXACTLY one phase; "hot AND caught" is unrepresentable.
pub enum FuseState {
    Fresh, Warming, Hot, Scalding, Critical,   // burning rungs (Pillar 2 fractions)
    Caught { receipt: CatchReceipt },          // terminal-good
    Exploded { at: TickIndex },                // terminal-bad (the SLA miss)
}
```

### 1.3 Gate 0 — the illegal states, and the tier we hit

| Illegal state | Technique | Tier (honest) |
|---|---|---|
| A potato in two phases at once | `FuseState` is a sum, not a struct of bools | **truly-unrepresentable** (compile) |
| A fuse advancing while the *author* holds the ball | the tick reads `PauseReason` from the VCS event; a paused fuse has no `advance` morphism | **only-mitigated** (runtime classify) — the §12 pause semantics are a `Result`, not an absence |
| An event appended out of sequence / a forged history | BLAKE3 chain: `seq` LE ‖ NUL-delimited fields ‖ `prev_hash`; genesis `[0u8;32]` — **the exact breathe-store `decision_content_hash` pattern** | **parse-time-rejected** (verify re-hashes RAW stored fields; cross-checks registry head for tail-truncation) |
| A score mutated after the fact | scores are *folds* over the chain (`CumulativeCounters::fold` shape from breathe-store), never stored mutable | **truly-unrepresentable** (no mutable score cell exists) |
| A potato for a workspace the caller can't see | `PrRef` carries the workspace; queries are `ProductScoped` (pleme-database); in-engine identity is `x-user-*` only (`from_jwt` internal-only) | **truly-unrepresentable in-engine** (no forge path) / data-scope **only-mitigated** |

The OutcomeChain is **the** load-bearing reuse: it is breathe-store's
`DecisionLog` trait + `PgDecisionLog` + the BLAKE3 `decision_content_hash_fields`
verify, with the leaf type swapped from breathe's `DecisionEntry` to
batata's `PotatoEvent`. Same `append`-in-one-tx, same `verify_chain`, same
add-only migrations. **Do not re-implement the chain — generalize breathe-store's
leaf type** (the three-site extraction trigger fires here: breathe + batata + any
third = lift `Chain<Leaf>` into a shared crate; until then batata consumes the
shape by copying the proven module and naming the extraction in `CLAUDE.md`).

### 1.4 The event ontology (the leaf type)

```rust
pub enum PotatoEvent {
    ReadyForReview { pr, at },
    RungEscalated  { pr, from: FuseState, to: FuseState, at },
    Passed         { pr, from: Reviewer, to: Reviewer, reason: PassReason, at },
    Caught         { pr, by: Reviewer, depth: ReviewDepth, at },
    Exploded       { pr, at },
    Defused        { pr, at },             // author re-took the ball (pause)
    BoardClean     { workspace, streak: u64, at },
}
```

Every analytic (Pillar 4) and every award (Pillar 4) is a pure query over
`Vec<PotatoEvent>` — the analytics *are* the game (DESIGN.md §6), not a second
store.

---

## Pillar 2 — the SLA master knob

One setpoint derives the whole game. This is **literal reuse of breathe's
band-law**: one setpoint → derived convergent behavior, `shikumi::TieredConfig`
chain, named presets = breathe's spot-fleet recipes.

### 2.1 The derivation (the only new algorithm)

```rust
/// The whole game falls out of one Duration + team attributes.
pub fn derive_ladder(sla: Duration, attrs: TeamAttrs) -> EscalationLadder;
```

- **Rungs are fractions of the SLA, scaled to its length** — `warming≈0.5`,
  `hot≈0.75`, `scalding≈0.9` (+ the pass), `critical=1.0` (explode). A 2-week SLA
  → gentle, spread-out, business-hours-aware rungs; a 2-day SLA → sharp, compressed
  rungs. *Same shape, auto-scaled.*
- **`TeamAttrs` modulate the derived preset** (the "good for 2 weeks based on
  attributes" requirement): repo criticality (compress), review-pool size (pass
  sooner), team size (DM-vs-channel noise floor), timezone spread (follow-the-sun
  pass target), typical PR size (longer fuse + more reviewers).

`derive_ladder` is a **TYPED-SPEC triplet** in its own right: typed
`EscalationLadder` border + `(defladder …)` Lisp spec of the canonical fractions
+ a pure interpreter `apply(spec, sla, attrs) -> EscalationLadder`. It has **no
side effects**, so its `Environment` is trivial — pure function, property-tested
(monotone: a longer SLA never produces an earlier rung; rungs strictly increasing;
`critical` always last).

### 2.2 Gate 0

| Illegal state | Technique | Tier |
|---|---|---|
| Rungs out of order (`hot` before `warming`) | `EscalationLadder` constructed only via `derive_ladder` → sorted-by-construction; the field is `pub(crate)` | **parse-time-rejected** (sealed ctor) — *fixable to truly-unrep with phantom typestate; named debt* |
| A negative / absurd fuse | `Fuse` is `Refined<Duration, FuseBounds>` (the ishou-tokens `Refined<T,B>` pattern — saturation by type) | **parse-time-rejected** (clamp at parse; `Refined::default` skips clamp — the known `only-mitigated` hole) |
| A rung that pings at 3am | business-hours mask is a typed `WorkingHours` value the tick consults; off-hours has no fire morphism | **only-mitigated** (runtime mask) |

### 2.3 The config surface (shikumi)

```rust
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatataConfig {                 // impls shikumi::TieredConfig
    pub sla: Duration,                    // THE knob
    pub attrs: TeamAttrs,
    pub overrides: Option<LadderOverrides>,// every rung overridable: env > yaml > derived-preset > bare
    pub presets: PresetSelector,          // "2-week-relaxed" | "2-day-sprint" | "critical-path" | "follow-the-sun" | custom
}
```

- `prescribed_default()` = the curated opinionated presets (the breathe pattern).
- The **shareable preset library** is the shikumi "custom tier" as a community
  marketplace — teams publish + fork `(sla, attrs)` bundles.
- Env prefix `BATATA_SVC_` (the breathe-config lesson: a too-generic prefix
  collides with chart legacy env on `deny_unknown_fields` — namespace it).

### 2.4 The game IS a promessa

The engine's correctness is a Viggy `(defpromessa)` (DESIGN.md §8 L9):

```lisp
(defpromessa batata-quente-prod-sla
  :scope       (services [game-engine])
  :target      (sla :availability "99.9%" :custom "no-potato-exploded")  ; PromessaTarget::Sla + ::CustomerKpi
  :window      "30d"
  :observation (shinryu :query "explosions in window" :materialize SlaSnapshot)
  :remediation { :on-functional (auto-correct :action (flux-commit …))   ; e.g. widen reviewer pool
                 :on-critical   escalate }
  :escalation  (ladder :step1 (controller :timeout "5m")
                       :step2 (on-call :timeout "15m" :who batata-pager)))
```

`derive_ladder`'s output and the promessa's `EscalationLadder` are the **same
type** (the promessa's `EscalationLadder { steps: Vec<EscalationStep> }`) —
the in-game per-PR escalation and the ops-level SLA escalation are one primitive.

---

## Pillar 3 — the integration IR + two traits

The iac-forge `Backend` trait pattern (one IR → N renderers), transposed to
runtime, bidirectional. Per the recon: **no multi-platform chat/VCS adapter
generator exists in the fleet** — the typed message/Block-Kit AST is the *one
genuinely new substrate primitive* batata-quente builds. slack-forge gives the
install/credential plane only, not a message AST.

### 3.1 The neutral IR (the warp — peer of `IacResource`)

```rust
// Outbound chat-render IR — the Block-Kit-AST-equivalent that does NOT exist yet.
pub enum BoardView { Board(Vec<Potato>), Leaderboard(Catchers), HeatMap(Heat), AwardShelf(Shelf) }
pub enum ChatMessage { Ping { mention: Mention, view: BoardView, fuse_bar: FuseBar }, Dm { … }, Award { … } }

// Inbound VCS-ingest IR — what every webhook resolves TO.
pub enum VcsEvent { PrReadyForReview(PrRef), ReviewSubmitted { pr, reviewer, substantive: bool },
                    ChangesRequested(PrRef), Draft(PrRef), CiRed(PrRef) }
// Outbound VCS-command IR — the neutral action a platform realizes (the pass).
pub enum VcsCommand { ReassignReviewer { pr, to: Reviewer }, ReadReviewerPool(RepoRef), BlockMergeTrain(PrRef) }
```

### 3.2 The two traits (peers of `Backend` + `NamingConvention`)

```rust
/// Outbound render. Slack / Teams / Discord each impl this. This IS the
/// Environment trait for the engine's chat side effects (TYPED-SPEC triplet).
pub trait ChatPlatform {
    fn platform(&self) -> &str;                                    // "slack" | "teams" | "discord"
    fn render_message(&self, msg: &ChatMessage) -> Result<PlatformPayload, AdapterError>;
    fn render_board(&self, view: &BoardView) -> Result<PlatformPayload, AdapterError>;
    fn naming(&self) -> &dyn ChatIdiom;                            // mention syntax, emoji, char limits
    fn validate_message(&self, _m: &ChatMessage) -> Vec<String> { vec![] }  // default-valid
}

/// Inbound ingest + outbound action. GitHub / GitLab / Bitbucket each impl this.
pub trait VcsPlatform {
    fn platform(&self) -> &str;                                    // "github" | "gitlab" | "bitbucket"
    fn ingest_webhook(&self, raw: &WebhookEnvelope) -> Result<Vec<VcsEvent>, AdapterError>;
    fn apply_command(&self, cmd: &VcsCommand) -> Result<CommandReceipt, AdapterError>;  // the pass
    fn read_reviewer_pool(&self, repo: &RepoRef) -> Result<ReviewerPool, AdapterError>; // CODEOWNERS
    fn naming(&self) -> &dyn VcsIdiom;
}
```

`PlatformPayload` is the typed-AST artifact (Slack Block Kit JSON / Teams
Adaptive Card / Discord embed) — peer of `GeneratedArtifact` — built through a
typed AST, **never `format!()` of JSON** (★★ TYPED EMISSION). The Slack payload
AST is the new primitive; a candidate name is `bloco` (BR-PT *block*) — a typed
Block Kit AST + serializer, the way `nix-synthesizer`'s `NixValue` is for Nix.

### 3.3 Gate 0

| Illegal state | Technique | Tier |
|---|---|---|
| An emitted Slack message exceeding the 50-block cap | `validate_message` override + `Refined`-bounded block vec | **only-mitigated** (validate returns the violations) |
| `format!()`-composed platform JSON | typed `bloco` AST + serde serializer; `with-format-ban` clippy on the crate | **truly-unrepresentable** (no `format!` compiles) |
| A webhook for platform A parsed by platform B's ingester | dispatch is by typed `platform()` key; an envelope carries its source | **parse-time-rejected** |
| A test needing live Slack/GitHub | the traits ARE the Environment boundary; tests use `MockChatPlatform`/`MockVcsPlatform` | **truly-unrepresentable** (no real I/O in the engine's unit tests) |

### 3.4 What's generated vs hand-written (per the recon)

| Element | Generated / inherited | Hand-written per platform |
|---|---|---|
| The neutral IR + the two traits + dispatch + composition + the blanket `ProvenMorphism` invariants (determinism, content-hash provenance, render-cache) | ✅ (the iac-forge `Backend`/`morphism` shape — copy the *shape*, sibling crate) | — |
| `render_message` / `ingest_webhook` / `apply_command` bodies | trait wiring + the `todoku::HttpClient`+`BearerToken`+`RetryPolicy` client shape (reuse from slack-forge) | ✅ the real new code — per-platform AST + webhook parse + reassign call |
| App-lifecycle / install / OAuth / bot-token capture | — | reuse **slack-forge** (Slack) + **github-app-forge** (GitHub); build the GitLab/Bitbucket/Teams/Discord lifecycle peers |
| The game/analytics/award **API + SDKs + MCP + docs** | ✅ **forge-gen** from one OpenAPI/sekkei→takumi spec | — |

Adding Mattermost = one `ChatPlatform` impl; adding Gitea = one `VcsPlatform`
impl. Net new code per platform is bounded to render/ingest/action bodies;
proofs/provenance/cache/dispatch are inherited.

---

## Pillar 4 — analytics + the `(defaward)` engine

Both are **pure queries/predicates over the OutcomeChain** — typed, so the
analytics SDK + dashboard widgets generate (forge-gen + Tela). No second system.

### 4.1 Analytics = typed views

Each metric in DESIGN.md §6 (catch rate, time-to-defuse, pass-chain depth,
explosion rate, clean-board streak, heat map, catcher board, load/fairness,
fuse-burn timing) is a `fn(&[PotatoEvent]) -> Metric` fold — the
`CumulativeCounters::fold` shape from breathe-store, generalized. They are
**provable** (the chain is attested) and **centralized** (one ontology). The
analytics endpoints are part of the forge-gen'd API surface.

### 4.2 The award engine — `(defaward)`

"Get crazy with awards" is unbounded because an award is **data**: a typed
predicate over the chain + a badge + a celebration. Solve the predicate engine
once; infinite awards fall out as data (the breathe `band_kind!` / Viggy promessa
pattern).

```rust
#[derive(TataraDomain, Serialize, Deserialize, Debug, Clone)]
#[tatara(keyword = "defaward")]
pub struct AwardSpec {
    pub id: AwardId,
    pub behavior: BehaviorPredicate,   // a typed predicate over Vec<PotatoEvent> (the interpreter evaluates it)
    pub badge: BadgeRef,               // ishou-tokened, Tela-rendered
    pub celebration: Celebration,      // a ChatMessage template (Pillar 3 render)
    pub guardrails: AwardGuardrails,   // see 4.3 — enforced by TYPE, not policy
}
```

`(defaward)` is a TYPED-SPEC triplet: typed `AwardSpec` border + `(defaward …)`
Lisp authoring + an interpreter `evaluate(spec, &chain) -> Vec<AwardGrant>` with
the chain as its (read-only, trivially-mockable) Environment. Award packs =
the preset-library distribution pattern.

Starter taxonomy (DESIGN.md §7): Clutch / Depth / Generosity / Reliability /
Team / Personality — each a `BehaviorPredicate` over the event stream.

### 4.3 Gate 0 — the anti-toxicity invariants are TYPES

The gamification recon's non-negotiables (no individual shame, score the catch
not the clock, the queue is the antagonist) are encoded so a toxic award is
**unrepresentable**, not merely discouraged:

| Illegal state | Technique | Tier |
|---|---|---|
| A "slowest reviewer" / shame award | `AwardSpec.behavior` ranges over *positive* `PotatoEvent`s only; there is **no `SlowEvent`/`MissEvent` attributable to a person** in the ontology — `Exploded` carries no holder-blame field | **truly-unrepresentable** (the negative event-with-blame doesn't exist) |
| A speed award gameable by rubber-stamp | a `Caught` event requires `ReviewDepth` (comments, files-touched); a clutch predicate that ignores depth fails a `const`-asserted guardrail test | **parse-time-rejected** (guardrail test fails the build) — *the anti-Goodhart core* |
| An individual leaderboard | the analytics `CatcherBoard` is a celebration list with **no "slowest"/rank-from-bottom projection** in its type | **truly-unrepresentable** (no such field) |

This is the project's sharpest unrepresentability claim and its ethical spine:
**the toxic version of this product literally cannot be authored** because the
data model has no negative-individual event to gamify. Cite the gamification
recon (overjustification + Goodhart + Octalysis White-Hat) as the *why*.

---

## Data model (Urdume L0 — add-only Postgres)

Schema `batata`, **add-only** (never drop a `score`/`pass`/`award` column —
deprecate + error-on-use; per Urdume L0 + reference_helm_native_infrastructure):

| Table | Holds | Notes |
|---|---|---|
| `workspace` | the tenant | `ProductScoped` axis; every other table FKs it |
| `repo` | a connected VCS repo | + reviewer-pool cache |
| `potato` | live + historical PRs | current `FuseState` is a projection; truth is `outcome_chain` |
| `outcome_chain` | the append-only attested event log | **the breathe-store `decision_log` schema** — `UNIQUE(stream, seq)`, BLAKE3 `content_hash`, genesis `[0u8;32]` |
| `chain_head` | per-stream head counters | the breathe-store `band_registry` pattern (tail-truncation cross-check) |
| `reviewer` | team members | identity from `x-user-*`; no in-engine JWT parse |
| `award_grant` | materialized award grants | a *cache* of the `evaluate(spec, chain)` fold; rebuildable from the chain |
| `config` | per-tenant `BatataConfig` | the SLA-preset (Pillar 2) |

Migrations are `shinka` `DatabaseMigration` CRs gated by a `shinka-wait` init
container — "serves an un-migrated schema" is unrepresentable at the pod level.
The store crate (`batata-store`) is **breathe-store with the leaf type swapped** —
same `DecisionLog`/`SampleCache` traits, same `PgDecisionLog`, same `verify_chain`.

---

## Generation map (define once → emit everything)

| Surface | Source | Generator |
|---|---|---|
| REST + gRPC + GraphQL + MCP + SDKs (Rust/Go/Py/JS/Java/C#) + completions + docs | one `spec/openapi.yaml` (sekkei→takumi IR) | **forge-gen** |
| 6 platform integrations (Slack/Teams/Discord × GitHub/GitLab/Bitbucket) | the Pillar 3 neutral IR + per-platform render/ingest bodies | the iac-forge `Backend`-shape sibling crate (`bloco` AST emitters); **slack-forge** + **github-app-forge** for install/credential |
| analytics endpoints + award SDK | the `(defaward)` / query forms over the chain (typed) | forge-gen (they're typed API ops) |
| board / leaderboard / heat-map / award-shelf UI | `WidgetSpec`s → Leptos | **pleme-widget-gen** (Tela F2) — `no format!()` of markup |
| Helm chart + flux + mesh | `(defcaixa :kind Servico …)` | caixa-helm/flux/mesh + `feira` (Urdume L5/L6) |
| Grafana dashboards | `Pangea::Architectures::Monitorable` | pangea-dashboards (Urdume L9) |

**The operator authors:** the game *rules* (Pillar 1/2/4 specs) + the per-platform
*render/ingest* impls (Pillar 3 bodies). **Everything else is emitted.**

---

## Catalog reflection (★★)

`batata-quente` ships a `(defbatata-catalog …)` meta-spec naming every typed
domain — `defgame`, `defladder`, `defaward`, plus each `ChatPlatform`/`VcsPlatform`
adapter — with fields per the org rule (name, authoring keyword, maturity gate,
purpose, upstream mirror, dependency edges). A substrate-invariant test fails the
build if a domain has source but no catalog entry. (Waiver-exempt: ≥3 typed
domains, so reflection earns its keep.)

---

## Milestones (destination first, then the path down)

The destination: **the SaaS in DESIGN.md §8–§10** — a multi-tenant Urdume+Tela
product with BYOC-resident provable-spawn installs. The path:

- **M0 — the dogfood core (no SaaS shell).** `batata-store` (breathe-store leaf
  swap) + `derive_ladder` + the fuse FSM + a *single* `ChatPlatform`(Slack) +
  `VcsPlatform`(GitHub) impl + the reconcile loop, wired to **one private Slack
  channel tracking the operator's own PRs**. Proves the fuse *feels* right and the
  SLA-preset derivation tunes well. The activation event (first PR through the
  game) is the value moment — build to it.
- **M1 — the analytics + award engine.** `(defaward)` interpreter + the analytics
  folds + the anti-toxicity guardrail tests (Pillar 4 Gate 0). Team dogfood
  (a team that runs PRs). Watch the streak + awards form.
- **M2 — the generated SaaS surface.** forge-gen the API/SDK/MCP; the second
  integration axis (Teams/Discord, GitLab/Bitbucket) via the trait pattern; the
  Tela board. Multi-tenant L8 (saguão + per-workspace `ProductScoped` + cofre
  install secrets).
- **M3 — BYOC provable-spawn.** The resident data-plane chart + the scoped-hook
  (namespace/role/cluster) magma spawn + `kensa verify` attestation + the Viggy
  auto-maintenance promessa (DESIGN.md §10). The enterprise/regulated premium tier.

De-risk order: the wedge is recon-confirmed; M0→M1 prove willingness-to-pay
before the M2 SaaS shell.

---

## Open decisions (carried from DESIGN.md §12 — the typed-spec angle)

- **Fuse pause semantics** → which `VcsEvent`s set `PauseReason`; the cleaner the
  pause, the closer the "fuse advancing while author holds the ball" row gets to
  truly-unrepresentable (currently only-mitigated).
- **Pass-target selection** → the fairness weight in `VcsCommand::ReassignReviewer`
  target choice (round-robin / CODEOWNERS / load-balanced + follow-the-sun).
- **`ReviewDepth` heuristic** → the typed definition of a "real" catch (the
  anti-Goodhart guardrail's teeth).
- **Chain extraction trigger** → when the third consumer appears, lift
  breathe-store's `Chain<Leaf>` into a shared crate (named now so it's not lost).
- **Product name** + `repo-forge` archetype when committing to build.
