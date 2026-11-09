# batata-quente — catch, code review as a team hot-potato game

> **Product name:** catch (public name + `catch.<domain>` hostname).
> **Codename:** batata-quente (BR-PT *hot potato*, the pleme-io Tier-2 convention).
> Internal references say batata-quente; everything external says catch.

The team PR-review **hot-potato game**: a per-PR review SLA clock that escalates
*louder + broader* as it burns down until someone catches the potato — a fun,
collective, team-positive game whose win condition is a perpetually-empty review
queue and whose proof is an attested SLA chain ("no potato exploded this sprint"
is a *theorem*, not a dashboard claim). A multi-tenant SaaS: an **Urdume Method**
backend (the game engine) + a **Tela Method** frontend (the board), hosted on rio.

## The three canonical docs (read in order)

1. **[`DESIGN.md`](./DESIGN.md)** — the product: the problem, the gamification
   insight (anti-Goodhart, White-Hat, team-collective), the competitive gap, the
   game mechanics, the SLA master-knob, the analytics, the award engine, the
   BYOC data-residency installation, the GTM.
2. **[`SPEC.md`](./SPEC.md)** — the typed codification: the four pillars (game-state
   typescape + OutcomeChain; the `derive_ladder(sla,attrs)` SLA knob + the
   `(defpromessa)`; the `ChatPlatform`×`VcsPlatform` integration IR; analytics +
   the `(defaward)` predicate engine), the add-only data model, the generation map,
   catalog reflection, and the M0–M3 milestones. Each pillar runs Gate 0 with
   honest unrepresentability tiers — incl. the spine claim that the **toxic version
   of this product is unauthorable** (the event ontology has no negative-individual
   event to gamify).
3. **[`DEPLOY.md`](./DEPLOY.md)** — the production hosting spec on rio:
   build+AUTOBUMP, CNPG+shinka, the pleme-lib chart, the pangea-operator
   cloudflare-tunnel ingress, observabilityBundle, Vector+NATS, breathe bands,
   saguão+cofre, the Tela frontend, the M0→M3 rollout, and the §14 compliance
   posture + required waivers. Grounded in the live rio tree, adversarially
   reviewed; **nothing is built yet** beyond these docs.

## Status — M0 starter built to the URDUME public-service mandate

The basic starter is built **fully by the standards** (URDUME L3 public-service
mandate: 3 faces from one typed core + SeaORM core + served OpenAPI/Swagger +
latest versions). Crate map:

| Crate | Role |
|---|---|
| `catch-core` | pure game logic — the `FuseState` FSM, `derive_ladder` (the SLA knob), the positive-only `PotatoEvent` ontology + the in-memory `Board`. Light unit tests incl. the anti-toxicity invariant (`Exploded` carries no holder). SPEC pillars 1 & 2. |
| `catch-entities` | the **SeaORM L0 core** — `DeriveEntityModel` entities (workspace / potato / outcome_chain), schema `catch`, add-only forever. The durable destination the `PgOutcomeChain` writes (M-phase). |
| `catch-api-server` | the `Arc<GameService>` core projected by the **THREE faces** — REST (axum) + **code-first OpenAPI/Swagger** (utoipa, `/swagger` + `/api-docs/openapi.json`), GraphQL (async-graphql, `/graphql`), gRPC (tonic `BoardService`) — plus `/health/{startup,live,ready}` + `/metrics` + the inbound webhook intake. |

- **Build:** the ONE gen pattern — `flake.nix` `lockfileBuilder.mkProject` over a
  committed `Cargo.gen.lock` + a dockerTools image (Pillar 8). `nix run .#release`
  is the esteira rio-lane push.
- **CI/CD:** `.github/workflows/release.yml` — the **esteira private rio lane**
  ([`theory/ESTEIRA.md`](../theory/ESTEIRA.md)): `[self-hosted, rio]`, AUTOBUMP
  exact-pin `amd64-r<run>-<sha>`, no container actions. Go-live gate: the rio
  runner must be registered + catch in its group + Actions enabled via the
  pangea-github catalog (ESTEIRA-PLAN blocker #3).
- **Data plane:** the in-flight `pleme-lib` data-lifecycle bundle (the abstracted
  shinka/CNPG/backup/breathe/cofre primitive — `helmworks/.../docs/DATA-LIFECYCLE-BUNDLE.md`).
- **M0 caveat (tier-honest):** the service serves from the in-memory board (no DB);
  the SeaORM/`PgOutcomeChain` durable store + the fuse-advancing reconcile loop are
  the M-phase. The full helm chart (caixa-rendered) + the rio GitOps manifests +
  the Tela frontend (`catch-web`, a separate leptos repo) follow.

## Substrate alignment (directives apply — catch is a full Urdume service)

The pleme-io org directives apply by default; catch is the **first consumer**
of several substrate primitives and a forcing-function for them:
- **The ONE Rust build pattern** — gen/lockfile-builder over a committed
  `Cargo.gen.lock` + dockerTools (Pillar 8); no `buildRustPackage`, no `fenix`.
- **AUTOBUMP** — exact-pin `amd64-r<run>-<sha>` built on `rio-build-01`.
- **HELM-NATIVE** — the chart is the infrastructure (pleme-lib bundles).
- **GITOPS-NATIVE + PLATFORM-MEDIATED** — declare to FluxCD + pangea-operator;
  declare+observe only.
- **The abstracted data-lifecycle bundle** — catch is the first consumer of the
  pleme-lib shinka/CNPG/backup/breathe/cofre data-lifecycle abstraction (in design;
  see DEPLOY.md §3/§4).
- **UNREPRESENTABILITY** — in-engine tenant-forgery is truly-unrepresentable
  (`from_jwt` internal-only); the OutcomeChain is the breathe-store BLAKE3 chain
  (parse-time-rejected single-table; Ed25519 signing is the M3 BYOC tier).

Waivers are added to this header **only when a deviation is actually taken**
(e.g. `skip-auto-release: internal-only` while private-first; `skip-platform-mediated:
db-bootstrap-job-pre-cnpg-database-cr` if the interim bootstrap Job is used instead
of a declared CNPG Database CR). DEPLOY.md §14 enumerates the candidate waivers.
