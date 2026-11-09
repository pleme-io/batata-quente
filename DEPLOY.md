# catch — production hosting on rio

> Companion to [`DESIGN.md`](./DESIGN.md) (the product + the gamification) and
> [`SPEC.md`](./SPEC.md) (the four typed pillars + data model). This file is the
> **production deployment spec**: how **catch** (the public product + the
> `catch.<domain>` hostname; **batata-quente** is the internal codename)
> gets built and hosted on the **rio** homelab cluster with full pleme-io
> substrate coverage.
>
> Status: scope / codification (2026-06-17). Grounded in a cross-subsystem read
> of the live rio tree; adversarially reviewed (compliance + completeness +
> tier-honesty). **Nothing is built yet** — `batata-quente/` holds only DESIGN.md
> + SPEC.md. Every artifact below is a name-swapped copy of a proven rio file;
> low-risk per-artifact, full greenfield in aggregate.

This spec is **GITOPS-NATIVE** + **PLATFORM-MEDIATED** + **HELM-NATIVE**
throughout: every change is a commit to `~/code/github/pleme-io/k8s/clusters/rio/`
(or a helmworks chart / pangea-architectures gem) that FluxCD + pangea-operator
reconcile. The operator's only verbs are **declare** (commit) and **observe**
(`grafana-rio` MCP). No `kubectl apply` / `helm install` / `tofu apply` by hand.

---

## §0 The destination (named first)

catch runs at **`catch.<domain>`** on rio as one Urdume `tecido` (the
game-engine service) + one Tela face (the board), fronted by saguão/Authentik,
durable on CNPG Postgres, observed by the pleme-lib observabilityBundle, held in
band by breathe, and delivered by AUTOBUMP — the **same substrate every rio
microservice uses**, mirroring the `grafana.<domain>` worked precedent.

**Tier-honest one-liner (corrected — do not round up):** most of the substrate
nouns are live, proven rio artifacts today (the CNPG Cluster, the cloudflare
tunnel + one-ingress-line, the AUTOBUMP image-automation loop, Vector→VLogs, the
observabilityBundle, breathe bands, the saguão/Authentik gate, the cofre-SOPS
secret path). **Two are NOT yet live on rio and gate M0/M1:**
- **shinka** — the operator is *undeployed*; **zero `DatabaseMigration` CRs have
  ever reconciled on rio** (`apps/drive/gaveta.yaml` carries `shinkaWait.enabled:
  false` with the comment "Off until a shinka DatabaseMigration `gaveta` CR is
  committed"). catch would be the **first** shinka consumer on rio — the
  un-migrated-schema unrepresentability guarantee is *unproven on rio*, not
  inherited.
- **NATS** — the `pleme-nats` broker is deprovisioned (`infrastructure/nats/
  release.yaml` `enabled: false`, paused 2026-05-03) and `catch` is not in
  `networkPolicy.allowedClientNamespaces`. The doctrine's "denshin-typed NATS
  subjects" surface does **not** exist; the real fleet idiom is raw `async-nats`.

The deploy is therefore destination-first but honestly phased: **M0 needs neither
NATS nor the web board's GraphQL seam** (the game flows via the Slack/GitHub
webhook path straight into the engine, with Postgres as the single source of
truth).

---

## §1 Runtime topology

```
                  INTERNET ──► Cloudflare edge (Universal SSL: <domain> + *.<domain>)
                                   │  CNAME catch.<domain> → <tunnel-id>.cfargotunnel.com
                                   │  (pangea-operator emits it; dns_proxied:true)
                                   ▼
                     ┌──────────────────────────────────────────┐
                     │  rio cloudflared tunnel "rio"             │ ◄── TWO ingress lines in
                     │  (rio-drive-cloudflare-tunnel CR)         │     drive-cloudflare-tunnel.yaml
                     └───┬───────────────────┬──────────────────┘     (pangea-operator reconciles)
       inbound webhooks  │                   │  board (browser)
       (Slack/GitHub POST)▼                  ▼
                   ┌──────────────┐     ┌──────────────┐
                   │ catch     │     │ catch-web │  (Tela/Leptos CSR, served by hanabi;
                   │ (game engine │◄────│ hanabi BFF   │   enable_bff:false static at M0/M1,
                   │  + /metrics  │ /gql│ (M2 only)    │   flips true at M2 when the GraphQL
                   │  + webhooks) │     └──────┬───────┘   subgraph lands)
                   └──────┬───────┘            │
 ══ CONTROL PLANE ════════┼═══════════════════ ┼══════════════════════════ DATA / RUNTIME ══
   FluxCD (prune,SOPS):   │ from_headers        │ x-user-* + x-product
     apps-catch + arch │ (pleme-rbac) —      │ (BFF injects; M2 derivation gap)
   pangea-operator (magma │  NEVER from_jwt     ▼ ProductScoped<T> (product_id = x-product)
     in-process) ◄────────┤            ┌─────────────────────────────────────────────┐
     reconciles tunnel CR │            │ ns catch: game-engine reconcile loop      │
   AUTOBUMP (image-       │            │ (PromessaController tick:                     │
     reflector+automation)│            │  Observe→Diff→Classify→Decide→Act→Attest→Tick)│
     writes #imagepolicy  │     append │ (replicas:1 — single-writer OutcomeChain)    │
     marker, commits      └────────────┤                                              │
                                       └──────┬──────────────────────┬────────────────┘
                              (ONE Postgres tx)│           (M1+) publish│ PotatoEvent
                                               ▼                        ▼
                                  ┌─────────────────────┐   ┌──────────────────────────┐
                                  │ CNPG Postgres        │   │ NATS JetStream (M1+)      │
                                  │ pangea-database      │   │ CATCH_GAME_EVENTS      │
                                  │ (.pangea-system)     │   │  → hanabi BFF invalidate  │
                                  │  schema catch:    │   │  (broker OFF on rio today)│
                                  │   outcome_chain      │   └──────────────────────────┘
                                  │   chain_head         │
                                  │  BLAKE3 verify_chain │
                                  │  UNIQUE(stream,seq)  │
                                  │  shinka-wait gates   │
                                  │  pod start (M0 first)│
                                  └─────────────────────┘
 ══ SIDEWISE — attaches to every pod by construction (the chart is a pleme-lib shim) ════════════
   Vector DaemonSet ─tail JSON stdout─► VictoriaLogs        breathe MemoryBand/CpuBand carve
   VMServiceScrape ─/metrics (health port)─► VictoriaMetrics  limits (dryRun-first; shadow→live)
   VMRule severity ─► Alertmanager ─► ntfy                  OBSERVE: grafana-rio MCP +
   NetworkPolicy: deny-all + allow-dns + allow-prometheus     status.lastCycle receipts
                    + EXPLICIT egress allowlist (§4)
```

**The actual M0 data-plane entry point is the inbound webhook**, not the board: a
PR-review game is driven by Slack events + GitHub App webhooks POSTing *in*. The
board (`catch-web`) at M0/M1 is a static/CSR shell that cannot read or write a
`PotatoEvent` (no rio deploy runs `enable_bff:true` with a real backend today —
`pangea-cloud/hanabi.yaml` and `pangea-coming-soon` both run
`federation.enabled:false`, "no backend yet"). The frontend→backend GraphQL seam
is **M2 work**, not a live pattern.

---

## §2 Build + ship — the gen flake + dockerTools image + AUTOBUMP

catch's backend is a Rust service built with **the ONE pleme-io Rust pattern**
(the same one breathe just adopted, 2026-06-17): substrate's gen-driven
`lockfile-builder.mkProject` over a committed `Cargo.gen.lock` delta + a
`dockerTools` image (Pillar 8, no Dockerfile). No `buildRustPackage`, no `fenix`.

**Precedents, disentangled (do not cross-inherit):**
- **breathe** proves the *flake shape* (gen/lockfile-builder + dockerTools) — but
  builds on `ubuntu-latest`, fixed-tag, and is **NOT autobump** (`skip-auto-release:
  internal-only`). Copy breathe's `flake.nix`, **not** its `image.yml`.
- **hanabi / gaveta** prove the *AUTOBUMP-on-`rio-build-01`* native-amd64 shape.
  Copy their `release.yml` / `auto-release.yml`.
- **pangea-operator + breathe** prove the *observabilityBundle* path (§6). gaveta
  has `monitoring.enabled: false`, so gaveta does **not** prove observability.

**Artifacts:**
- `catch/flake.nix` — `packages.{default, catch-api-server, catch-mcp,
  image}`; `image` = `dockerTools.buildLayeredImage` over
  `project.workspaceMembers.catch-api-server.build` (copy breathe's flake; swap
  crate names). The reconcile loop runs in-process in the api-server; likely only
  two binaries (api-server + mcp), not breathe's four.
- `catch/Cargo.gen.lock` — committed slim delta (`gen build .` emits it +
  `Cargo.build-spec.json`). `.gitignore`: `Cargo.build-spec.json` + `/target`
  `/result` `/result-*`.
- `catch/.github/workflows/auto-release.yml` — ~10-line shim over
  `pleme-io/substrate/.github/workflows/image-push.yml@main`, `runner:
  rio-build-01` (**CRITICAL native-arch caveat:** build on `rio-build-01` amd64
  or the image `exec-format-error`s at runtime — the recurring hanabi
  aarch64-as-amd64 bug). `tag: amd64-r${{github.run_number}}-${{github.sha}}`,
  `additionalTags: amd64-latest`. Private ghcr package (BYOC/multi-tenant) →
  `skip-auto-release` waiver if not public-publishing.
- `catch/spec/openapi.yaml` — the single forge-gen source (sekkei→takumi IR)
  for the L3 REST+gRPC+GraphQL+MCP+SDK+docs surface. **CI gate:** `forge-gen
  generate` + fail-on-diff against committed SDKs (GEN TYPED-SPEC CONTRACT).
- `ghcr.io/pleme-io/catch:amd64-r<run>-<sha>` — the AUTOBUMP exact-pin stream;
  `:amd64-latest` is a human pointer only.

**AUTOBUMP wiring (copy gaveta):**
- `k8s/clusters/rio/infrastructure/flux-image-automation/catch.yaml` —
  `ImageRepository` (`ghcr.io/pleme-io/catch`, `secretRef: ghcr-charts-secret`)
  + `ImagePolicy` (`filterTags ^amd64-r(?P<n>[0-9]+)-[0-9a-f]+$`, extract `$n`,
  numerical asc).
- Add `- catch.yaml` to that dir's `kustomization.yaml`.
- The shared `rio-autobump` `ImageUpdateAutomation` writes the selected exact tag
  into the HelmRelease's `# {"$imagepolicy": "flux-system:catch:tag"}` marker
  and commits to `main` — GitOps-native delivery, never `:latest`.

---

## §3 Data layer — CNPG Postgres + shinka migrations + the OutcomeChain

**Durable home:** the shared `pangea-database` CNPG Cluster in `pangea-system`
(`cloudnative-pg/postgresql:16.6`, `instances: 2`, limits ceded to breathe). This
is the operator's *own* state DB — so the per-app DB provisioning is the one place
PLATFORM-MEDIATED matters most (see the bootstrap decision below).

**catch is the FIRST shinka consumer on rio** (corrected — there is no live
precedent). The pleme-lib templates exist (`pleme-lib.shinkaDatabaseMigration` +
`pleme-lib.shinkaWaitInitContainer`, v0.28.0) but have **never reconciled** on
rio. So:

**M0 step-0 (HARD prereq, before the catch image):** deploy the **shinka
operator** into `shinka-system` as a **FluxCD HelmRelease/Kustomization** with its
image **AUTOBUMP- or `v*`-semver-pinned** (never `kubectl apply -f deploy/`, never
a floating tag) under its own `infrastructure/shinka/` dir — it is a *shared* rio
component (benefits gaveta + every future Urdume service). Then commit + **prove
ONE `DatabaseMigration` CR reconciles to `phase==Ready`** and the `shinkaWait`
init container exits 0. Acceptance: `kubectl get databasemigration catch -o
jsonpath={.status.phase}` → `Ready`.

**Per-app DB provisioning — PLATFORM-MEDIATED decision (resolve before M0):**
- **(a) compliant destination:** declare the `catch` role + database via the
  `pleme-cnpg` chart's `bootstrap.initdb` or a CNPG `Database` CR that Flux/the
  operator reconciles.
- **(b) interim escape hatch:** a one-shot idempotent bootstrap Job (clone
  `infrastructure/cracha/db-bootstrap-job.yaml`, `\gexec` for `CREATE DATABASE`).
  A Job running `CREATE DATABASE` is still an imperative command inside the
  operator's state DB — so it requires a `skip-platform-mediated:
  db-bootstrap-job-pre-cnpg-database-cr` waiver in `catch/CLAUDE.md` naming the
  CNPG-Database-CR destination. **Use the CNPG admin/superuser secret shape** (not
  cracha's stale vanilla-PG `pangea-database-app` username/password keys — that
  header predates the CNPG migration).

**The OutcomeChain (the system of record):** catch-store is breathe-store with
the leaf type swapped — *consume the proven module, name the `Chain<Leaf>`
extraction in CLAUDE.md* (the third-consumer extraction trigger).
- `migrations/0001_chain_head.sql` (clone breathe `band_registry`: `stream TEXT
  PK`, counters `BIGINT`, `seq BIGINT`, `last_hash BYTEA`).
- `migrations/0002_outcome_chain.sql` (clone breathe `decision_log`: `UNIQUE(stream,
  seq)`, `content_hash`/`prev_hash BYTEA`, the `PotatoEvent` columns).
- `0003+` add-only: `workspace`, `repo`, `potato`, `reviewer`, `award_grant`
  (rebuildable cache), `config` — all FK `workspace`, **add-only forever** (never
  drop a column).
- `src/{lib.rs,pg.rs}` — `PgOutcomeChain`: runtime sqlx 0.8 (no `query!` macros),
  `sqlx::migrate!`, append in ONE tx (`INSERT … ON CONFLICT DO NOTHING` → `SELECT …
  FOR UPDATE` → fold → `INSERT` → `UPDATE` → commit), `verify_chain` re-hashing the
  RAW stored fields + chain_head cross-check. Byte-identical to breathe-store.

**Tier-honesty on the chain:** the chain *logic* (content-hash, tamper-detection)
is unit-tested and runs in default CI; the **PG tx / FOR-UPDATE / concurrency tests
are `BREATHE_TEST_PG_URL`-gated and SKIP in vanilla CI** (breathe-store proved them
only against a live Postgres). catch-store inherits this — add a rio-Postgres or
kind+CNPG ephemeral CI job if the tx path must be CI-proven. The M0–M2 chain is
breathe's **unsigned** M2 tier: `verify_chain` catches single-table tampering, but
a writer with access to *both* `outcome_chain` and `chain_head` can rewrite
consistently. The BYOC `kensa verify` proof (DESIGN.md §10) **requires the
Ed25519-signed tameshi/sekiban chain** — an M3 `PgOutcomeChain` backend swap.

**add-only ⇒ reversible (stated as discipline, not a present guarantee):** the
add-only schema is what *makes* rollback-by-image-redeploy possible — but deploying
the prior image against a forward-migrated schema is only safe if every intervening
migration was strictly additive AND the prior binary tolerates the new columns. It
is the discipline, proven per-migration, not an automatic property.

**Backup/PITR (do NOT defer to M3):** the OutcomeChain is the system of record, so
catch needs a per-database restore story from M0 — either an catch-scoped CNPG
`ScheduledBackup` within the shared cluster, or a documented "rides the cluster WAL→
Garage S3 archive" with an explicit RPO/RTO + restore procedure.

---

## §4 The Helm chart — full pleme-lib coverage

**Decision (recommended): consume the existing public `pleme-microservice` chart**
(the gaveta path) rather than author a bespoke `helmworks/charts/catch`. It
already ships `deployment.yaml`, `service.yaml`, `pdb.yaml`, `hpa.yaml`,
`networkpolicy.yaml`, `servicemonitor.yaml`, `shinka-migration.yaml`,
`prometheusrule.yaml` — every template the hand-rolled chart would re-list, with
the PDB+HPA the draft would have dropped. Author a bespoke chart only if catch
needs a template the microservice chart lacks (then it is a 1-line `{{- include
"pleme-lib.<x>" . }}` per template + a `Chart.yaml` dep on `pleme-lib >=0.28.0`).

**The HelmRelease** (`k8s/clusters/rio/apps/catch/catch.yaml`, mirror
`apps/pangea-cloud/hanabi.yaml` for the public-chart path or `apps/drive/
gaveta.yaml` for a private `GitRepository` + `./deploy/chart`) wires:

- **Dual-port:** container `http` (game API + webhook receiver) + `health`/`metrics`
  (probes + `/metrics`, kept off the public API surface). Service: `http` on `:80`
  (uniform with grafana/pangea ingress URLs), `metrics` on its own port.
- **Probes — standardize on `/health/{startup,live,ready}`** (the live
  pangea-coming-soon/hanabi convention; delete any `/healthz`//`readyz` variant).
  `/health/ready` checks **DB reachable + migration complete + (M1+) NATS
  connected**; `/health/live` checks **only the process** (must NOT touch the DB or
  a transient blip restart-loops the pod). `startupProbe` enabled.
- **Resources:** `requests` + **mandatory `limits`** (so breathe can carve), e.g.
  `requests {50m, 64Mi}` / `limits {500m, 256Mi}` — refined once measured.
- **PodDisruptionBudget:** `pdb: {enabled: true, minAvailable: 1}` — the always-on
  reconcile engine must survive a node drain mid-tick.
- **HPA DISABLED + single-writer:** `hpa: {enabled: false}` — **breathe owns
  vertical scaling; HPA would fight the limit-carve.** The engine runs **`replicas:
  1` pinned** (or leader-election) because the OutcomeChain appender is a
  single-writer — a second replica double-appends. Tie to open decision #scaling.
- **NetworkPolicy — explicit egress allowlist** (`pleme-lib.networkpolicy` +
  `_compliance_egress.tpl`): `deny-all` + `allow-dns` + `allow-prometheus` (ingress
  on the metrics port) **plus egress to**: `pangea-database-rw.pangea-system:5432`,
  `shinka.shinka-system:8080` (the `shinkaWait` init-container poll — silently
  blocked under default-deny otherwise), Slack API + GitHub API (the app-plane
  calls), and `pleme-nats.nats:4222` (M1+). **M0 check:** the init container can
  reach `shinka-system` under the applied policy + compliance overlay.
- **`compliance.overlays`** — the multi-tenant SaaS posture (one declaration; the
  BYOC-resident tier adds the regulated overlay set).
- **`shinkaMigration` + `shinkaWait`** — `cnpgClusterRef {pangea-database,
  catch}`, `migrator {type: sqlx, command: [/bin/catch, --migrate]}`,
  `shinkaWait.enabled: true`. (Live only after §3 M0 step-0.)
- **Graceful drain (opt-in):** `lifecycle.preStop` (HTTP `/drain` so the reconcile
  tick + OutcomeChain flush finish) + `terminationGracePeriodSeconds: 30+`.

**FluxCD wiring:** `apps/catch/{namespace.yaml (pod-security baseline),
catch.yaml, kustomization.yaml}` + `flux-kustomizations/apps-catch.yaml`
(`dependsOn: [infrastructure, infrastructure-pleme-charts]`) + register
`- apps-catch.yaml` in `flux-kustomizations/kustomization.yaml`.

**CI drift gates (GEN TYPED-SPEC CONTRACT / CATALOG REFLECTION):** regenerate-and-
diff for the forge-gen SDKs, the generated `dashboards/catch.json`
(`render_all.rb`), and any `breathe-bands-generated.yaml`.

---

## §5 Ingress — pangea-operator cloudflare-tunnel (two hosts)

rio's per-cluster tunnel is `clusters/rio/architectures/drive-cloudflare-tunnel.yaml`
(an inline-Ruby `InfrastructureTemplate` calling
`Pangea::Architectures::CloudflareTunnel.build`). Adding a host = appending an
entry to its `ingress:` array (additive; the resource is `destroyProtection: true`)
**before** the `http_status:404` catch-all, then bumping the `# rev:` comment to
force an operator re-compile. `dns_proxied: true` makes the architecture emit the
`catch.<domain> → <id>.cfargotunnel.com` CNAME automatically; Universal SSL
already covers `*.<domain>`. Mirror the `grafana.<domain>` entry verbatim.

**Two ingress entries (the board host alone is insufficient):**
1. **The board** — `{ hostname: 'catch.<domain>', service:
   'http://catch-web.catch.svc.cluster.local:80' }`.
2. **The webhook receiver (the actual M0 data-plane entry)** — route the Slack
   events + GitHub App webhook path to the **backend** Service. Either a dedicated
   host `{ hostname: 'hooks.catch.<domain>', service:
   'http://catch.catch.svc.cluster.local:80' }`, or path-routing
   `catch.<domain>/api/webhooks/*` → the backend. **Without this the engine
   receives zero events and the OutcomeChain never advances** — pick the scheme and
   state the path-prefixes the engine exposes.

**HARD M0 BLOCKER — the tunnel state-drift defect.** `apps/drive/RUNBOOK.md`
(2026-06-16) records the `rio-drive-cloudflare-tunnel` operator going to a
**persistent magma no-op** (state recorded ingress hostnames as applied while the
live Cloudflare config never received them); `drive.<domain>` is currently a
**surviving hand-PUT**, pending an operator re-converge that has not happened.
Adding `catch.<domain>` as a new entry to a tunnel whose operator is wedged
means the M0 serving gate is **blocked on an unresolved operator defect, not a
budgeted step.** Before appending catch:
- **Clear the drift the compliant way** — force the operator to re-converge (an
  operator reconcile annotation / `importPolicy.autoOnConflict` re-adoption / a
  magma state-row drop+recompute so the live config becomes operator-owned again),
  and **verify it re-converges the drive+pangea+grafana entries** (plan ≠
  all-NoOp-on-stale-state) **first**.
- **Anti-goal (hard):** a hand-PUT to the Cloudflare API is **forbidden** as the
  serving mechanism — even as break-glass — unless logged in
  `apps/catch/RUNBOOK.md` AND followed by the operator state refresh in the same
  change.

---

## §6 Observability — the bundle + game-native metrics + the dashboard

One `.Values.observability` border (`flavor: vm`) → scrape + rules + route +
dashboard, via a single `{{- include "pleme-lib.observabilityBundle" . }}` (proven
by pangea-operator + breathe; *not* by gaveta, which has `monitoring.enabled:
false`).

- **`.Values.observability`:** `scrape {port: metrics, path: /metrics, interval:
  30s}`, `rules {enabled, groups}`, `dashboard {enabled, deploy: sidecar, file:
  dashboards/catch.json}`. Metrics on the dedicated health/metrics port.
- **Rust side** (`src/observability.rs`): the metric statics + `init_observability
  ("catch")` + a `/metrics` axum handler (`text/plain; version=0.0.4`) — mirror
  hanabi's `prometheus`-crate pattern. *Three-site trigger:* hanabi + breathe +
  catch = the point to lift a shared `init_observability(product)` +
  `/metrics`+`/health`+label-injection primitive into tsunagu/ServiceBuilder
  (none exists today) — extract on this third consumer.
- **Game-native series** (workspace/repo-labeled, **NO holder label** —
  anti-toxicity is structural): `catch_potatoes_active`, `catch_catch_rate`,
  `catch_explosions_total`, `catch_pass_chain_depth`,
  `catch_fuse_escalations_total{rung}`, `catch_time_to_defuse_seconds`,
  `catch_clean_board_streak`, `catch_reconcile_ticks_total`,
  `catch_reconcile_errors_total`, `catch_build_info{version}`.
- **Pangea-declared dashboard:** `pangea-architectures/lib/pangea/architectures/
  catch.rb` (`Pangea::Architectures::Catch < Monitorable`, `monitor do…end`
  StatusOverview→DataPresence→game-panels→LogWindows + `monitor_alert` VMRule
  groups with severity labels) + an `catch_drift_spec.rb` that fails CI if any
  panel/alert expr references a metric absent from the Rust allowlist + a
  `render(Pangea::Architectures::Catch, 'catch')` entry in `render_all.rb`.
- **Cardinality:** the `workspace` label gives per-tenant panels but unbounded
  cardinality at SaaS scale — gate per-workspace series behind a recording-rule
  rollup (the BYOC-resident single-tenant install sidesteps this).
- **SLA-as-promessa:** encode SPEC.md §2.4's `(defpromessa)` "no-potato-exploded"
  as a plain VMRule alert at M0; the full Viggy PromessaController + OutcomeChain
  attestation is the L9 destination (M3).

---

## §7 Vector + NATS

**Vector (live, zero new files):** catch inherits the running rio `pleme-vector`
DaemonSet (`infrastructure/vector`, `kubernetes_logs` → VictoriaLogs). Requirement:
catch logs JSON to stdout via `tracing`; **verify** the `catch` namespace is
not in Vector `excludePaths` and that the compliance NetworkPolicy leaves the tail
path intact.

**NATS (NOT live — deferred to M1).** The broker is `enabled: false` (paused
2026-05-03) and `catch` is absent from `allowedClientNamespaces`. **M0 needs no
NATS** — Postgres is the source of truth; BFF cache-invalidation lands at M1. When
wired (M1+):
- Two prerequisite GitOps commits: flip `infrastructure/nats/release.yaml`
  `enabled: true`, and add `catch` to `networkPolicy.allowedClientNamespaces`.
- `templates/jetstream.yaml` = `{{- include "pleme-lib.jetstream" . }}`;
  `jetstream:` block declares stream `CATCH_GAME_EVENTS`
  (`subjects: catch.game.events.>`, `retention: limits`, `storage: file`,
  `replicas: 1` — rio is single-node). Subjects: one PotatoEvent leaf each
  (`…<workspace>.{ready,escalated,passed,caught,exploded,defused,board-clean}`).
- **Close the fan-out leg explicitly:** either declare a durable JetStream
  **consumer** for `catch.board.invalidated` in the `jetstream` values, OR state
  that hanabi consumes via **Core NATS subscribe (at-most-once, lossy on restart)**
  and that lossy invalidation is acceptable because the board re-reads from Postgres
  on the next request. Do not leave a stream with no consumer.
- Rust uses **raw `async-nats`** (publish `with_headers` + `Nats-Msg-Id` dedup;
  subscribe loop) — *there is no denshin-typed-NATS surface*; that's the shipped
  fleet idiom (hanabi/tend).

---

## §8 Breathability — breathe bands (shadow-first)

Every catch workload is held in a utilization band. Bands ship `dryRun: true`
(SHADOW) and are promoted to LIVE only after the `ShadowWouldApply` window holds —
in order: game-engine cpu → frontend → catch-db memory/cpu (after the CNPG
limits cede).

- `infrastructure/breathe-bands/memoryband-catch-game-engine.yaml` — `MemoryBand`,
  `targetRef {Deployment, catch, apps/v1}`, floor 192Mi / ceiling 768Mi,
  `allowRestart`, `dryRun: true`.
- `cpuband-catch-game-engine.yaml` — floor 100m / ceiling 1000m,
  `restartFreeOnly`.
- `memoryband-catch-frontend.yaml` / `cpuband-catch-frontend.yaml` — 96Mi/256Mi,
  50m/500m.
- `memoryband-catch-db.yaml` — `targetRef {Cluster, catch-db,
  postgresql.cnpg.io/v1}`, floor 2Gi / ceiling 4Gi, `predictive: true`
  (`lookahead 60s`), `cooldown 600s` — mirrors the live pangea-database band.
  `cpuband-catch-db.yaml` floor 500m / ceiling 2 (disjoint `breathe/cpu` field
  manager).
- **StorageBand** (`storageband-databases.yaml` entry) — `GrowOnly`, 5Gi/25Gi,
  `dryRun: true` — **stays parked in Conflict/`MetricUnrepresentable` on rio's
  local-path** (no per-volume accounting); promote only when a per-volume CSI
  (Longhorn/Ceph/EBS) lands.

**Prereq:** cede `limits.{cpu,memory}` in the CNPG Cluster values (helm-`null`)
before the DB bands go live, or breathe and helm-controller fight over the leaf
fields. **RBAC** for the Deployment bands (`pods/resize`) is fleet-granted already;
the **M3 CNPG-Cluster band + StorageBand** rely on `cnpg/clusters` + `pvc` patch
verbs in the controller ClusterRole — confirm present before promoting those.

---

## §9 Identity + secrets — saguão + cofre (the tenant boundary)

**The engine never authns/authzs itself.** It reads only the injected `x-user-*` /
`x-product` headers via `pleme_rbac::AuthzContext::from_headers` — `from_jwt` is
forbidden in-engine (in-engine tenant-identity forgery is truly-unrepresentable —
no code path). Every query goes through `pleme_database::ProductScoped<T>` with
`product_id = x-product`.

- **Gate (recommended: SSO):** catch is a team-internal PR-review SaaS → front
  it with saguão/Authentik. Register an `catch` OIDC provider + app +
  `catch-users`/`catch-admins` groups in
  `infrastructure/authentik/quero-cloud-blueprint.yaml` (mirror grafana-rio);
  native-OIDC-via-hanabi (matches the Leptos auth FSM) over forward-auth-proxy.
  The operator/admin surface is gated by a `cracha` grant in
  `infrastructure/cracha/policies/family.yaml`; **cracha gates only the
  operator/admin surface** — per-team customer roles live in catch's own DB via
  `pleme-rbac` roles in the headers.
- **Customer auth is Slack/GitHub, not Authentik:** team members authenticate via
  the Slack/GitHub install OAuth flow. The BFF session that stamps
  `x-user-*`/`x-product` is minted from that social identity (verify hanabi's
  `auth/social_oauth.rs` covers Slack + GitHub app installs).
- **Secrets via cofre `SecretRef`, SOPS-interim (ESO is suspended fleet-wide on
  rio):** SOPS-encrypted Secrets in `apps/catch/` for the DB URL, the Slack
  bot/app token + signing secret (`generation: None` — operator captures from the
  install, cofre verifies), the GitHub App private key + app id + webhook secret
  (`generation: None`), and (M1+) NATS creds. An `catch/cofre/catch.cofre.yaml`
  `SecretMaterializationPlan` declares the typed plan (`Sops` backend interim →
  a vault destination under `/pleme/rio/catch/<key>`); the ESO `ExternalSecret`
  arm is the M3 destination (blocked on ESO unsuspend).

**GENUINE MULTI-TENANT GAP (M2 build work, not just a decision):** hanabi's
`config.bff.product` is a **single static value** today; no shipping deploy derives
`x-product` per session (lilitu/novaskyn are one-product-per-deploy). A public
multi-tenant SaaS **requires the BFF to derive `x-product` from the session's
workspace** — this is unbuilt and is the genuine tenant-isolation gate, owned by
**M2 (a hanabi work-item)**. The in-engine `from_headers`/`ProductScoped` half is
solid; the BFF-derivation half is missing.

---

## §10 Frontend — the Tela/Leptos board

`catch-web` is a Leptos CSR app served by hanabi (SSR is blocked on the
leptos_axum / axum 0.7-0.8 skew — `pending-tela: 4`; the localStorage auth
regression is `pending-tela: 9`).

- `catch-web/flake.nix` (~12 lines) = `leptos-build-flake.nix` →
  `{default(=combined), ssrBinary, csrBundle, dockerImage}`; the `dockerImage` uses
  `mkLeptosDockerImageWithHanabi` (CSR served by the hanabi binary). Mirror
  `lilitu-web/flake.nix` + an `ishou` input. 3-line `auto-release.yml` on
  `rio-build-01` → `ghcr.io/pleme-io/catch-web:amd64-r<run>-<sha>`.
- Deployed in `apps/catch/` (the board host from §5), `pleme-web` chart,
  AUTOBUMP marker `# {"$imagepolicy": "flux-system:catch-web:tag"}`, probes
  `/health/{startup,ready}`, NetworkPolicy on.
- hanabi `config.yaml`: **`enable_bff: false` static-from-ConfigMap at M0/M1** (the
  pangea-coming-soon shape — NOT the armázem-S3 path); **flip `enable_bff: true` +
  `bff.mode: proxy` + `federation.supergraph_url`** only at **M2** when the backend
  GraphQL subgraph lands. Widgets from `pleme-widget-gen` + ishou `TokenRef`s
  (SPEC.md Pillar 4 / Tela F2).

---

## §11 Rate-limiting + cross-cutting standard concerns

- **Rate-limiting (★★ samba pattern):** a Cloudflare-edge or engine-side
  `LeakyBucket` on the inbound webhook + the public board endpoint (multi-tenant
  abuse surfaces), AND **samba-paced outbound** to the Slack/GitHub APIs (the engine
  is a rate-limited *consumer* posting board updates back — `quotaPct` knob, rate
  from `X-RateLimit-Limit`). Currently specified nowhere; it's a standing org
  pattern + a real abuse surface.
- **Backup/PITR:** §3 — per-database restore story from M0, not deferred.
- **PDB + single-writer + HPA-off:** §4 — the always-on engine survives drains,
  runs `replicas: 1`, breathe owns vertical.
- **Hermetic supply chain:** catch's chart dep is `pleme-lib` (in-tree
  `file://`) — no upstream subchart, so no `forge helm mirror` obligation today;
  if it ever pulls one (CNPG/NATS), the mirror step applies.

---

## §12 Rollout — destination-first, then phased

The destination is the §0 fully-hosted catch. The path:

- **M0 — dogfood core (no NATS, no GraphQL board seam).**
  - *step-0 (HARD prereq):* deploy the shinka operator (FluxCD, pinned image) +
    prove one `DatabaseMigration` reconciles to `Ready`; **clear the tunnel
    state-drift** so the operator re-converges (verify, don't hand-PUT).
  - Build the catch image on `rio-build-01`; provision the `catch` DB
    (CNPG Database CR or waivered bootstrap Job); ship the chart with the
    OutcomeChain + shinka-wait + bands (shadow) + observability + egress policy;
    append the **two** tunnel ingress lines (board + webhook).
  - **GATE:** `catch.<domain>` returns HTTP 200 from a real pod (operator-
    re-converged, not hand-PUT) AND the operator's own PRs flow through the game via
    the **webhook path** in a private Slack channel (the board is a static shell at
    M0). `databasemigration catch` is `Ready`; the shinkaWait init exits 0.
- **M1 — analytics + awards + team dogfood.** The `(defaward)`
  engine + analytics folds + anti-toxicity guardrail tests (green in CI); promote
  breathe bands from shadow in order; wire NATS (broker flip + ns whitelist) for
  BFF invalidation.
  - **GATE:** the catch Grafana dashboard loads on `grafana.<domain>` with
    real catch-rate/streak/explosion series AND the guardrail tests are green.
- **M2 — generated SaaS surface + multi-tenant.** forge-gen the API/SDK/MCP; build
  the hanabi **`x-product`-per-session derivation** (the real tenant-isolation
  gate); flip `enable_bff: true` against the federated subgraph; the Tela board
  goes live; a second integration axis (Teams/Discord × GitLab/Bitbucket).
  - **GATE:** a second customer workspace is isolated end-to-end; the board reads
    live game state.
- **M3 — BYOC provable-spawn (enterprise/regulated tier).** Dedicated CNPG cluster;
  Ed25519-signed OutcomeChain (tameshi/sekiban `PgOutcomeChain` swap); the Viggy
  auto-maintenance promessa; ESO-migrated secrets; the resident-data-plane chart +
  scoped-hook magma spawn.
  - **GATE:** a customer security team runs `kensa verify` against a resident
    install and gets a cryptographic proof the vendor created exactly the declared
    scoped infra.

---

## §13 Open decisions (consolidated)

1. **DB provisioning** — CNPG `Database` CR (compliant) vs waivered bootstrap Job
   (interim) for the per-app `catch` DB in the operator's state cluster. (§3)
2. **Public chart vs private chart** — consume `pleme-microservice` (recommended)
   vs a bespoke `helmworks/charts/catch` vs a private `./deploy/chart` (gaveta).
3. **Service port** — `:80` (uniform ingress) vs `pleme-microservice`'s default
   `:8080`; keep the ingress line consistent with whichever the chart emits. (§5)
4. **Webhook host scheme** — dedicated `hooks.catch.<domain>` vs
   `catch.<domain>/api/webhooks/*` path-routing to the backend. (§5)
5. **SSO gate style** — native-OIDC-via-hanabi (recommended) vs forward-auth-proxy.
6. **Stream-key granularity** — per-PR (`workspace/repo/number`) vs per-workspace
   `chain_head` (changes `UNIQUE(stream,seq)` cardinality + `verify_chain` cost).
7. **NATS at M1** — durable JetStream consumer vs Core-NATS lossy subscribe for BFF
   invalidation; retention `limits` vs `workqueue`.
8. **breathe ceilings** — the floor/ceiling numbers are precedent-derived estimates;
   refine once the service + frontend footprints are measured.
9. **SeaORM timing** — catch could be the forcing function that puts the SeaORM
   entity set into the shared chain pattern, or ride breathe-store's SQLx floor.
10. **`Chain<Leaf>` extraction** — catch is the third chain consumer (breathe-
    store + batata + …); lift the shared crate now or copy-and-name-the-trigger.
11. **Hostname scheme** — 2-part `catch.<domain>` (matches the pangea./
    grafana. precedent) is sanctioned for public app hosts; per-tenant
    `<tenant>.catch.<domain>` would need its own cert (Universal SSL covers
    only `*.<domain>`).
12. **Product/codename** — chart dir + image repo = `catch` (product), repo
    codename stays `batata-quente`.

---

## §14 Compliance posture + required waivers

- **GITOPS-NATIVE / PLATFORM-MEDIATED:** every mutation is a commit FluxCD +
  pangea-operator reconcile; the cloudflare tunnel + CNAME are operator-owned. The
  one tension is the per-app DB bootstrap (§3) — resolved as a CNPG Database CR
  (compliant) or a **`skip-platform-mediated: db-bootstrap-job-pre-cnpg-database-cr`**
  waiver.
- **HELM-NATIVE / Pillar 8:** the chart is the infrastructure; the image is
  dockerTools, no Dockerfile.
- **AUTO-RELEASE / AUTOBUMP:** ship-on-merge exact-pin; private ghcr →
  **`skip-auto-release: internal-only`** waiver if not public-publishing (like
  breathe), dropped when catch goes public.
- **TYPED-EMISSION / GEN TYPED-SPEC CONTRACT:** forge-gen + dashboards + bands are
  regenerate-and-diff CI gated.
- **UNREPRESENTABILITY (tier-honest):** in-engine tenant-forgery is
  truly-unrepresentable (`from_jwt` internal-only); the un-migrated-schema pod gate
  is truly-unrepresentable **once shinka is live** (unproven on rio until M0
  step-0); the OutcomeChain is parse-time-rejected single-table but **only-mitigated**
  cross-table until the M3 Ed25519 signing.
- **breathe SHADOW-first:** every band `dryRun: true` until its shadow window holds.
