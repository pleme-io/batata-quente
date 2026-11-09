# Batata-Quente — code review as a team hot-potato game

> Codename **batata-quente** (Brazilian-Portuguese *hot potato*), per the pleme-io
> Tier-2 naming convention. Product name TBD.
>
> Status: design / codification (2026-06-17). Recon-validated wedge; not yet built.
> Companion: **`SPEC.md`** (the typed spec — the four pillars + the data model +
> the generation map).

---

## 0. The destination (named first)

The team's **code-review queue is perpetually empty** — not by nagging
individuals, but by turning review into a **fun, collective hot-potato game**
with a per-PR timer that escalates *louder and broader* as it burns down, until
someone catches the potato. The win condition is a clean board. The proof is an
**attested SLA chain** — "no potato exploded this sprint" is a *theorem*, not a
dashboard claim.

The behavioral bet: a polite daily nudge is ignorable; a **shared countdown the
whole team rallies on** is not. Make the queue *everyone's* problem and *fun* to
clear, and it clears itself.

The product bet: define the game **once** as a typed spec and **generate** every
integration (Slack/Teams/Discord × GitHub/GitLab/Bitbucket), every SDK, the MCP
surface, the analytics API, and the web board — Pillar 3 (forge-gen) + Pillar 12
(generation over composition) applied to a SaaS.

---

## 1. The problem

PR review latency is the silent killer of cycle time. The queue fills because
review is nobody's *specific* job; reminders are ignorable; and the two failure
modes are both bad — either the slow reviewer gets privately shamed (toxic), or
nothing happens and the PR rots. Every incumbent treats this as a *notification*
problem ("remind the reviewer"). It's a *coordination + incentive* problem.

---

## 2. The insight — gamify it (the differentiator)

Reframe the PR as a **hot potato**: a ready-for-review, unactioned PR you don't
want to be *holding* when the timer escalates — but it's a *team* game, so when
it's about to explode the whole team rallies to catch it, never to blame the
holder. **Passing** = auto-reassign to a backup reviewer on a miss. **Catching**
= a real review → the timer defuses → you score. The escalation is **fun +
collective** ("don't let it blow up on the team"), never individual name-and-shame.

Fun is the moat. Incumbent nudges feel like nagging; a hot-potato countdown with
team streaks feels like a game you *want* to win.

---

## 3. The competitive gap (recon-validated, June 2026)

| Tool | graduated escalation | per-PR SLA clock | attested proof | auto-action | zero-queue framing |
|---|---|---|---|---|---|
| **Axolo** | ❌ | ❌ | ❌ | ❌ | partial |
| **Swarmia** | ❌ | partial (aggregate agreements) | ❌ | ❌ | partial |
| **LinearB** (WorkerB + gitStream) | ❌ | ✅ pickup deadline | ❌ | partial (gitStream) | ❌ |
| **GitHub native / merge queues / Mergify / Graphite** | ❌ | ❌ | ⚠️ rulesets ≠ SLA proof | ✅ (merge enforcement) | ❌ |
| **longtail** (ReviewNudgeBot, PullNotifier, …) | ❌ | ❌ | ❌ | ❌ | ❌ |

**Three uncontested axes** (the reason to exist): (1) **graduated escalation
toward a deadline**, (2) **attested SLA proof**, (3) **the composed timebomb** —
LinearB has the *pieces* (an SLA clock + auto-actions) but in two products that
don't compose on a deadline. Axolo's strength (ephemeral per-PR channels) is
orthogonal — batata-quente can even sit *beside* it.

---

## 4. Game mechanics (the codified rules)

```
🥔  Fresh      PR ready-for-review → on the board, SILENT.
🥔  Warming    fuse ~50% spent      → quiet DM to the holder.
🔥  Hot        fuse ~75% spent      → channel ping, @holder, fuse bar shown.
🔥  Scalding   fuse ~90% spent      → @holder + @author; potato AUTO-PASSES
                                       to a backup reviewer (auto-reassign).
💥  Critical   past deadline        → @here, the whole team rallies to catch it
                                       before it "explodes"; lead notified;
                                       (optional) block the merge-train.
✅  Caught     a REAL review lands   → fuse DEFUSED, holder scores, off the board.
🧹  Clean      board empty          → team streak ++ ("kitchen's clean").
```

- **The fuse** = a per-PR review SLA clock. Starts at `ready_for_review`; **pauses**
  when the ball is in the author's court (changes-requested / draft / CI-red) so
  the clock only burns on *reviewer* time. Business-hours-aware (no weekend burn,
  no 3am pings).
- **The pass** = auto-reassign on a miss — the *team* mechanic + the auto-action
  incumbents lack. Target = load-balanced over CODEOWNERS / a reviewer pool.
- **The catch** = a real review (quality-gated — see §7 guardrails), not a
  rubber-stamp `/lgtm`.
- **Scoring is team-positive only** (§7).

---

## 5. Configuration — the SLA is the only knob you need

One master knob — **the SLA** — derives the whole game; everything is overridable.
This is `shikumi::prescribed_default` + the breathe band-law (one setpoint →
derived convergent behavior), reused verbatim.

```
derive_ladder(sla: Duration, attrs: TeamAttrs) -> EscalationLadder
```

- **Rungs are fractions of the SLA, scaled to its length** — a 2-week SLA →
  more, gentler, spread-out rungs (warming ~day 7, hot ~day 10, scalding+pass
  ~day 12, explode day 14); a 2-day sprint SLA → fewer, sharper rungs compressed
  into hours. Same shape, auto-scaled.
- **Business-hours-aware by default** — the fuse burns on *working* hours.
- **Attributes modulate the derived preset** (the "good for 2 weeks based on
  various attributes" requirement):
  - *repo criticality* → compress the ladder (escalate earlier/steeper),
  - *review-pool size* → how soon to **pass** (more catchers ⇒ pass sooner),
  - *team size* → DM-vs-channel-ping noise floor,
  - *timezone spread* → **follow-the-sun passing** (pass to an awake reviewer),
  - *typical PR size* → bigger diff ⇒ longer fuse + more reviewers.
- **Named presets** = `(sla, attrs)` bundles — "2-week relaxed", "2-day sprint",
  "critical-path", "follow-the-sun" — like breathe's spot-fleet recipes.
- **Two preset tiers**: curated/opinionated presets ship as the defaults (the
  `prescribed_default` tier); a **shareable preset library** lets teams publish +
  fork presets to each other (the shikumi "custom tier" as a community marketplace).
- **Fully configurable** — the derived ladder is the default; every rung is
  overridable through the shikumi chain (`env > yaml > derived-preset > bare`).

The lovely part: this is *literal reuse* of breathe's M1 `BreatheServiceConfig` +
band-law. The game configures itself from one SLA — not new infra, the substrate
you already have.

---

## 6. Game-native analytics — the analytics ARE the game

**One ontology, three faces: the game model IS the data model IS the analytics
model.** Because every potato's lifecycle (`ready → rung escalations → pass(es) →
catch | explode → defuse`) is recorded in the append-only **OutcomeChain**, the
analytics aren't *estimated* or *borrowed* — they're exact, **provable** queries
over the game's own events. No bolted-on DORA dashboard speaking a different
vocabulary (the Swarmia/LinearB approach); the analytics literally **are the game
you're playing.**

| Metric | Game-native meaning | Signal |
|---|---|---|
| **Catch rate** | % caught before explosion | headline + the SLA-adherence *proof* |
| **Time-to-defuse** | fuse-% consumed at catch | how close to the wire you live |
| **Pass-chain depth** | passes before a catch | high = a potato nobody wants → too-big PR / knowledge silo / overload |
| **Explosion rate** | SLA misses | the attested ~0 target |
| **Clean-board streak** | consecutive clean periods | team achievement / retention hook |
| **Heat map** | which repos/paths/labels spawn the hottest potatoes | the review bottleneck, *located* |
| **Catcher board** | catches, fastest defuses, clutch saves | team-positive celebration — **no "slowest" ranking** |
| **Load / fairness** | who's holding how many potatoes now | burnout/fairness, framed as load-balancing |
| **Fuse-burn timing** | when potatoes heat up | staffing/coverage → feeds follow-the-sun passing |

The analytics are **generated, not hand-built**: the analytics queries are part
of the same forge-gen'd API surface (one spec → analytics endpoints + SDKs + MCP);
the dashboard is Tela-rendered from generated widgets. Analytics is a *view* over
the OutcomeChain, not a second system. **Centralized** — one place, game-native,
provable.

---

## 7. The award engine — `(defaward)` over the game events

"Get crazy with awards" is **unbounded by design** because an award is a typed
`(defaward …)` form: a **behavior-predicate over the OutcomeChain + a badge + a
celebration**. Same Rust+Lisp typed-domain pattern as breathe's `band_kind!` and
the Viggy promessa — solve the predicate engine once, infinite awards fall out as
*data*. Teams + the community publish **award packs** (the preset-library pattern
again); the award API + badge widgets are generated (forge-gen + Tela).

Starter taxonomy, by **behavior type** (deliberately not just speed):

- **Clutch** 🚒 *Firefighter* (caught at >90% fuse) · ⚡ *First Responder* · 🧤 *The Closer*
- **Depth** 🔬 *Deep Diver* (substantive review) · 🐛 *Bug Hunter* (a requested-change that prevented a revert) · 🏛️ *Architect*
- **Generosity** 🧭 *Mentor* (teaching comments on first-timer PRs) · 🦸 *Unblocker* (caught a much-passed potato) · 🤝 *Tag Team*
- **Reliability** 🛡️ *Iron Reviewer* (never exploded a held potato in N days) · 📅 *The Reliable*
- **Team** 🧹 *Clean Kitchen* · 💚 *No Explosions* · 🔁 *The Relay*
- **Personality** 🦉 Night Owl / 🐦 Early Bird · 🦅 *Vulture* · 🍷 *Sommelier* · 🥔👑 *Hot Potato Hero* (seasonal)

**Non-negotiable guardrails (hard-wired):**
1. **Positive-only.** No "slowest reviewer" / shame award, *ever*. Accountability
   is team-collective (the team owns the streak), never individual blame.
2. **Quality-gated speed.** Speed/clutch awards require a *real* review (comments,
   files touched) — reward outcomes (caught-a-bug, unblocked, mentored), which you
   can't game without doing the good thing. No rubber-stamp incentive.
3. **Opt-in intensity.** Dial from "quiet board" to "full arcade," like the SLA
   presets.

---

## 8. Full-SaaS architecture — an Urdume service + a Tela frontend

batata-quente-as-SaaS is built the pleme-io way: the backend is an **Urdume
Method** service (one typed spine → every layer), the frontend is a **Tela Method**
app (generated widgets → Leptos), multi-tenant from L8.

| Urdume trama | batata-quente |
|---|---|
| **L0** data + migrations | the game/PR/team/score/award entities (SeaORM/Postgres + shinka migrations); the OutcomeChain event store — *add-only* (Urdume L0: never drop a column) |
| **L1** shikumi config | the SLA-preset model (§5) — per-tenant, hot-reloadable |
| **L2** ServiceBuilder/tsunagu | the service runtime (the game engine loop) |
| **L3** forge-gen API | the game + analytics + award API → generated SDKs/REST/gRPC/GraphQL/MCP/docs (§9) |
| **L4** hanabi BFF | the frontend's data seam |
| **L5** caixa | packaging |
| **L6** helmworks / pangea-operator | deploy (multi-tenant on the cluster) |
| **L7** mesh | (single-service for now) |
| **L8** saguão + cofre | **the multi-tenant boundary** — per-workspace tenant isolation + auth + the Slack/GitHub install secrets (cofre `SecretRef`, never plaintext) |
| **L9** observability + promessa | the **game engine itself IS a Viggy `(defpromessa)`** (SLA "no PR past deadline" + the `EscalationLadder`); the OutcomeChain is the attestation |

The **game engine = a PromessaController-shaped reconcile loop**: observe the
queue → advance each fuse → fire the rung's action via the generated integration
adapters. The frontend (board / leaderboard / heat map / award shelf) = Tela
(Leptos + ishou tokens + pleme-widget-gen, deployed at `batata.<domain>`-style).

---

## 9. The generation story — define once, generate everything

The whole SaaS pitch: **one typed game spec → every surface generated.**

- **The API** (game + analytics + awards) is defined once (OpenAPI / the
  sekkei→takumi typed IR) → **forge-gen** emits SDKs (Rust/Go/Python/JS/Java/C#) +
  REST + gRPC + GraphQL + the **MCP** server + completions + docs. (Per the org
  IaC-Forge pipeline.)
- **The integrations are generated from a typed integration IR**, the iac-forge
  `Backend`-trait pattern applied to platforms:
  - a **`ChatPlatform`** trait (Slack / Teams / Discord) — render the board,
    pings, fuse bar, leaderboard, award shelf; the per-platform message AST
    (slack-forge for Slack; the same shape generated for Teams/Discord),
  - a **`VcsPlatform`** trait (GitHub / GitLab / Bitbucket) — ingest webhooks
    (PR ready/reviewed), read the reviewer pool, **reassign** (the pass).
  One typed IR → N adapters generated, exactly as iac-forge emits terraform /
  pulumi / crossplane / ansible from one IR. Adding a platform = a new renderer,
  not a hand-rolled integration.
- **The award engine + analytics** are `(defaward)` / query forms over the
  OutcomeChain — typed, so the award/analytics SDK + dashboard widgets generate too.

Net: the operator authors the **game spec + the platform IR**; the SDKs, servers,
MCP, six platform integrations, analytics API, and web board are *emitted*. New
code is the game *rules* + the per-platform *render/ingest* impls; the plumbing is
generated. *(Exact generator wiring — forge-gen's confirmed output surface, the
slack-forge API, any existing integration-forge to reuse — is being recon'd; this
section states the approach; `SPEC.md` will carry the precise wiring.)*

---

## 10. Installation & data residency — bring your own boundary

**The trust inversion.** Normal SaaS puts *your* data in the *vendor's* cloud.
batata-quente puts the *vendor's* infrastructure in *your* boundary — the vendor
never holds your data. For a tool ingesting review-performance data, resident data
is an enterprise make-or-break, and **no incumbent (all vendor-cloud) offers it.**

### The hook (the customer's choice — escalating depth, all data-resident)

1. **A scoped cloud IAM role** — least-privilege, Pangea-declared, auditable; the
   customer grants cross-account assume. magma spawns a minimal data plane (managed
   K8s / serverless / a VM) in their account.
2. **A Kubernetes namespace + scoped RBAC** *(preferred)* — the customer gives a
   namespace + a ServiceAccount in their existing cluster; the data plane (the
   Urdume service + an embedded CNPG Postgres + the OutcomeChain) deploys there via
   FluxCD + pangea-operator. Lightest, fastest — their cluster does the hosting.
3. **A dedicated cluster** — engenho + FluxCD + pangea-operator run resident; full
   control+data plane in the customer's boundary.

### Provably spawn (the pleme-io thesis = the moat)

- The data-plane infra is a typed Pangea architecture / Urdume L6 chart — the
  exact, scoped, auditable spec.
- magma / pangea-operator **realizes** it *in the granted hook* (typed in-process
  provider-RPC apply, ★★ MAGMA-NATIVE — declare + observe, never a manual apply).
- Every spawned resource gets a tameshi/sekiban BLAKE3-chained, signed receipt; the
  customer runs **`kensa verify`** → a cryptographic **proof** that the vendor
  created *exactly* the declared, scoped infra — verifiable blast radius, no
  data-exfil path, no surprise resources. "Provably spawn" = the ★★ PROVABLE
  OUTCOMES doctrine applied to the install: the customer's security team
  *verifies*, doesn't *trust*. The scoped grant itself is Pangea-declared +
  least-privilege, so what the vendor *can* do is auditable and what it *did* is
  attested.

### Data residency by construction

The data plane (Postgres, the OutcomeChain, the analytics store) is resident; game
data never crosses out. The control-plane channel carries only **declarations**
(GitOps desired-state / version pins) inbound + **attestations** (receipts)
outbound + (optionally, customer-approved) aggregate non-PII telemetry. The raw
review data *cannot* leave — not by policy, by construction (resident data plane +
a declaration/receipt-only channel). This is data sovereignty as a structural
property, the build-time half of the Viggy `compliance.airgap` posture.

### Fully automated maintenance (Viggy — the "as long as they agree to")

The resident data plane is reconciled by an in-cluster operator (the Viggy
continuous-convergence model — **no customer ops**). Updates = the vendor pushes a
version (AUTOBUMP exact-pin digest / a GitOps commit); the operator reconciles it;
shinka runs migrations gated by a `shinka-wait` init-container, attested. The
customer agrees to auto-maintenance — and every maintenance action is **attested**,
so it's "verify what the automation did," not "trust us to update it." The
maintenance SLA (uptime, patch-currency) is itself a `(defpromessa)` the resident
operator proves it holds — same machinery as the review-SLA game.

### The honest cost

BYOC is operationally harder than vendor-cloud (N heterogeneous customer
environments, version skew, the customer's cluster state affecting the data plane).
The Viggy reconcile + attestation + AUTOBUMP make cross-environment ops automatable
in a way most vendors *can't* — but it is the price of the residency moat, and the
scoped capability must be genuinely airtight (the customer's security team will
scrutinize it). The Pangea-declared least-privilege grant + `kensa verify` is the
answer; the BYOC industry posture (Snowflake/Databricks/Confluent customer-hosted
data planes) is the GTM precedent to validate against.

### Why this is the substrate's first consumer product

This monetizes the pleme-io reason-to-exist — **declare → provably-spawn
(Pangea/magma/attestation) → auto-maintain (Viggy)** — and dogfoods the entire
stack (Pangea, magma, pangea-operator, engenho, tameshi, kensa, shinka, cofre,
saguão, AUTOBUMP, eclusa). "Promises become marketable, continuously-attested
theorems" stops being a slogan and becomes a product whose **deployment itself is
the moat**: resident data + provable scoping + zero-ops, which no vendor-cloud
incumbent can match.

---

## 11. Build + go-to-market

1. **Dogfood, private** — a private Slack channel: just the operator, tracking
   their own PRs, feeling the fuse + tuning the SLA-preset derivation.
2. **Dogfood, team** — a team that runs pull requests (pleme-io commits
   straight to main). Real-team data; watch the streak + the awards form.
3. **Beta** — a design partner as the first reference customer.
4. **Product** — Slack + GitHub marketplace, multi-tenant SaaS; the **attested-SLA
   + the game-native analytics** are the paid/enterprise differentiators.

De-risk order: the wedge is recon-confirmed (§3); next gate is willingness-to-pay,
proven via the team dogfood + beta before the full SaaS shell.

### Cost & pricing tiers

BYOC-resident costs more to *run* than vendor-cloud — but the cost lands such that
it's a **tier, not a problem**:
- **Compute shifts to the customer** (namespace/cluster hooks): they pay for the
  data plane; the vendor's COGS is the control plane + maintenance, not the DB.
- **The substrate collapses the cross-environment ops cost** (Viggy reconcile +
  AUTOBUMP + attested maintenance + GitOps) — a tiny team operates N resident
  installs: the "one operator + agents" leverage applied to a *fleet of customer
  deployments*, which is what normally makes BYOC unaffordable.
- **Two tiers from one typed spec** (the deployment target is just config —
  Urdume L6 + which hook):
  - **Vendor-cloud multi-tenant SaaS** → the SMB / low tier (shared infra, data in
    our cloud).
  - **Resident BYOC** → the **enterprise/regulated premium tier** — the segment
    that pays *most* for residency + provable scoping + sovereignty. Higher cost
    matched by higher willingness-to-pay; the customer bears compute + the
    substrate keeps ops cheap → potentially *higher* margin. **The cost IS the
    enterprise moat you charge for.**

---

## 12. Open decisions

- **Fuse pause semantics** — exactly when the reviewer clock pauses
  (changes-requested / draft / CI-red). Get this right or the game feels unfair.
- **Pass target selection** — round-robin / CODEOWNERS / load-balanced; fairness
  weight so the same person isn't always passed to.
- **Anti-gaming on the "catch"** — heuristics for a *real* review (comments,
  files-touched, time-on-diff) vs accepting `/lgtm` + trusting team norms.
- **Award packs + preset library distribution** — curated-only at launch, or open
  the community marketplace early?
- **Product name** (batata-quente is the codename) + repo home (this dir is the
  seed; `repo-forge` it when committing to build).
