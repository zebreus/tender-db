# Post-build canonical-layer verification suite

Read-only checks to run the moment the full canonical rebuild lands, to confirm
the tender layer is sound. Derived from CONTEXT.md, docs/adr/, and the schema
(crates/store/src/canonical.rs). **Read-only — nothing here writes or deploys.**

## A TEST RUN IS AN ARTIFACT TOO: BUILD IT THE WAY PRODUCTION IS BUILT

`cargo test --workspace` looked like the widest possible check and was a **proxy for a
different binary**. `crates/app` declares `default = []`, and its `server` feature is what
pulls in `store`, `axum`, `tokio/rt-multi-thread` and gates the `v1` and `supervisor`
modules behind `#[cfg(feature = "server")]`. Production is built by
`nix/package.nix` with `-p tender-db --features server`. So the workspace run **never
compiled `v1/` or `supervisor.rs` at all** — the deploy gate blessed a binary that was
not the one shipping.

Two runs of that command, by two people, agreed exactly — and the agreement was worth
nothing. **Two instruments agreeing means something only if they COULD have disagreed**,
and two identical invocations share every blind spot they have.

Use the feature set the release build uses. The project's own workspace incantation is in
`nix/package.nix` (`cargoClippyExtraArgs`):

```sh
cargo test --workspace --features tender-db/server --no-fail-fast
```

**`--no-fail-fast` is not optional.** `cargo test` stops at the first failing *target* by
default, so a failure early in the run **silently skips every later target** and the
summary still reads like a complete result. Measured on the same tree: without it,
328 passed / 1 failed across 47 targets; with it, **343 passed / 1 failed / 22 ignored
across 53** — fifteen tests that simply never ran.

It also corrupted an earlier verdict here. A run on `5c197e7` reported "290 passed,
5 failed" and I read the five as the whole story; in fact the lib target failed, cargo
stopped, and the app's five integration targets never executed. The disk-health probe
appeared to *pass* on that tree when it had never run — which then looked like evidence
that a later failure of the same test must be a code change rather than the box. Silence
about a target is indistinguishable from a target that passed.

**What the corrected command found immediately:** 290 passed, **5 failed** — five
`supervisor::tests` queue-recovery tests, invisible to every prior run because the module
did not compile.

**And what it cost, measured:** with `server` on, the app lib runs **54 tests** —
`v1` **20**, `supervisor` **19**, `accounts` 7, `coverage` 6, `ledger` 1. Without it those
modules are not compiled at all, so the gate had **zero coverage of the entire public HTTP
API surface** (`v1`) and of the job supervisor, while reporting a confident 243/0. The
missing coverage was not a corner: it was the two subsystems a deploy is most likely to
break. Same family as B5 and the by-name index parser: a check whose *subject*
was assembled from what came to mind (packages) rather than derived from what ships
(packages **and features**). The unifying form: **derive the check's subject from what
ships, not from what comes to mind.**

### Is the corrected command the WHOLE shipped feature set? (checked 2026-08-03, task #19)

Adding `--features tender-db/server` fixed the gap that was *noticed*. The rule says
derive from what ships, so the fix itself was checked rather than extended from the one
gap. Result: **closed, with one sized and benign residual.**

The shipped artifact is built **twice** (`nix/package.nix`) — `dx build --platform server`
for the native binary and `dx build --platform web` for the WASM client — so "the
artifact" is two compilations, not one, and the corrected command performs only the first.
What that costs was established exhaustively rather than reasoned about:

* **Exactly one feature gates any first-party code: `server`** (19 `cfg(feature = …)`
  sites, all of them `server`). **No code anywhere is gated on `web`**, so the web
  compilation contains no first-party code the server compilation lacks.
* `model`, `store` and `ingest` declare **no `[features]` at all** — nothing can hide in
  them under any flag.
* There is exactly **one** `cfg(not(feature = "server"))` in the workspace,
  `crates/app/src/main.rs:23`: the three-line WASM entry point
  (`fn main() { dioxus::launch(App); }`), containing **zero tests**. This is the only
  shipped code the corrected command does not compile.
* All five `crates/app` integration targets — `accounts`, `admin`, `api`, `sql`,
  `webhooks` — compile and enumerate under the corrected command (verified with
  `cargo test … -- --list`), alongside both `tender_db` lib/bin targets.
* The workspace has **no doc-tests** (all four `Doc-tests` targets report 0).

**Residual, recorded as out of scope with the reason:** the WASM entry stub. Testing it
would require a `web`-featured build, which targets wasm and contains no tests, so it
would add a compilation and zero assertions. Re-open this if first-party code ever
appears behind `cfg(feature = "web")` or `cfg(not(feature = "server"))` — the greps above
are the check, and they are one command each.

**The other cargo invocation in this suite was checked, not assumed.** The checked-set
generator and the statement/LIKE probes all run `cargo test -p store …`, and
`crates/store` has **no `[features]` section at all** and zero `cfg(feature)` in its
source — so there is no feature under which its code could fail to compile, and those
probes are not exposed to this defect. Recorded as a verified negative so the next
person does not have to re-derive it; re-check it if `store` ever grows features.

## A BROKEN DETECTOR IS INVISIBLE WHEN SOMETHING ELSE SUPPLIES THE ANSWER

The sharpest instance of this is not in the gate — it is in the tooling around it
(proj-fix, 2026-08-03). Eleven background waiters were written as:

```sh
until ! pgrep -f "cargo test --workspace"; do sleep 30; done
```

That string appears in the **watcher's own command line**. `pgrep` excludes its own PID
but not its siblings or parent, so every watcher matched itself: `pgrep -fc "cargo test
--workspace"` returned **12 with no cargo running at all** — the twelve were the watchers.
**The condition was never satisfiable, and not one of them ever fired.**

It went unnoticed all day because **the harness's task-completion notifications supplied
every answer independently.** Each wait *appeared* to work; the result always arrived. The
detector's total failure was masked by a second, working channel delivering the same
information.

That is a category beyond "a check that has never been seen to fail" (rule 2). This check
had never been seen to **succeed** either — nobody could tell, because success and failure
produced identical observable outcomes as long as something else answered the question.

**The test:** if this check broke completely, would anything look different? If the answer
is no — because another path supplies the same result — the check is decorative until
proven otherwise, and it should be exercised in isolation at least once. A poll that never
fires looks exactly like a poll still waiting.

## ONE VERIFICATION OWNER PER OBLIGATION

Learned the expensive way, 2026-08-03. Two branches independently grew a **section I**
in `de1x_verify.sh` for the same obligation — the issue-98/100 winners-gap disclosure —
written weeks apart by authors who could not see each other's. Both passed, on their own
branch, indefinitely. The duplicate surfaced **only** when the branches merged.

They did not agree. One asserted the served JSON at `/api/dashboard`; the other grepped
`GET /` for the same words. The disclosure is rendered client-side, so the second
**cannot pass on a healthy deployment** — it was the predecessor that issue 110 was filed
to replace, still alive on the other branch, still reporting.

The rule that follows:

* **Each honesty obligation gets exactly one check, with one owner.** Not one per
  branch, not one per person who noticed the obligation.
* **Before adding a check, look for the one that already exists.** Two gates for one
  obligation are worse than either alone: when they disagree the red is
  uninterpretable, and the reliable human response to an uninterpretable red is to stop
  reading the section.
* **A duplicate is invisible from inside a branch.** Neither gate could have detected
  the other; nothing but the merge could. So the search has to be deliberate — grep the
  suite for the obligation, not for the filename you were about to create.

This is why convergence matters and not merely that it tidies history: it is what
establishes a single owner per check.

Expected top-line (deterministic, from the completed grouping):
- **tenders = 6,961,311**
- **islands (island_notice_id NOT NULL) = 640,745**  ⇒  keyed = **6,320,566**
- tender_versions ≈ **12.36M** (one per grouped notice; ~14k unprojected suffix
  notices are left for the incremental projection, so versions < 12.375M plan).

## How to run — and the turso hash-cliff caveat

turso 0.7.0 spins on an in-RAM hash for high-cardinality `GROUP BY` /
`COUNT(DISTINCT)` / `SELECT DISTINCT` at this scale (memory: the grouping's
`COUNT(DISTINCT)` cliff), and a per-row correlated subquery over millions of rows
is catastrophic. So every check is tagged:

- **[LIGHT]** — bounded: a `COUNT(*)` with a `WHERE`, or a `GROUP BY` on a
  LOW-cardinality column (kind, source, provisional, op, entity_kind,
  identifier_kind). Safe on the live server via **`/v1/sql`** (single SELECT) or
  `/admin`. A full `tenders`/`tender_versions` scan is still seconds — fine.
- **[HEAVY]** — high-cardinality `GROUP BY` (tender_id, caused_by_notice_id,
  organization_id), anti-joins, or correlated existence over millions. **Do NOT
  run these through turso** — run them with **stock `sqlite3` (real SQLite, no
  hash cliff)** against the idle DB or a post-build snapshot. Each is annotated
  with why.

**Executing safely** (no live-writer contention, no turso engine): the turso file
is SQLite-format, so stock `sqlite3` runs the WHOLE suite (LIGHT + HEAVY — real
SQLite has no hash cliff) once the service is idle OR against a post-build snapshot:
```
sqlite3 file:/data/db/tender-db.db?mode=ro < checks.sql           # service stopped
# or, zero-downtime, on a snapshot taken after the build completes:
sqlite3 file:/data/db/snapshots/<post-build>.db?mode=ro < checks.sql
```
(run-driver reported a read-only sqlite3 scan of the 278GB file is ~30s; a
`mode=ro` open while the server is actively writing may report "database is
locked" — prefer a snapshot or a brief idle window.) The **[LIGHT]** subset also
runs one-at-a-time through the live server's **`/v1/sql`** (each is a single
SELECT) without stopping the service — though a few bundle several full-table
scans, so if `/v1/sql` caps the timeout, fall back to sqlite3.

### The two runners + the post-build execution plan

- **`run_light.sh`** — runs the [LIGHT] set through `POST /v1/sql` against the LIVE
  service, prints a PASS/FAIL/EYE report, and **exits nonzero if any hard-fail gate
  misses**. Re-runnable. Needs `bash`, `curl`, `jq`, and:
  `TDB_TOKEN=tdb_… [BASE_URL=https://tenders.zebreus.click] ./run_light.sh`
  (create the API token on the dashboard; auth is `Authorization: Bearer tdb_…`).
- **`heavy.sql`** — the [HEAVY] set as a self-contained stock-`sqlite3` script
  (`.mode box`, labelled sections, expected results inline), assumes an idle/stopped
  DB or a snapshot, no server dependency.
- **`checks.sql`** — the full combined reference (LIGHT + HEAVY, annotated). The two
  runners are the executable split of it.

**Recommended sequence (matches the deploy window):**
1. Build completes, service idle+responsive → `./run_light.sh` against live (early
   read; hard-fail gates gate the layer).
2. In the same window you stop the service to deploy the write-side speedups →
   `sqlite3 -readonly "file:/data/db/tender-db.db?mode=ro" < heavy.sql` against the
   now-unlocked file (or a post-build snapshot for zero downtime).
3. Deploy + restart.

This README is the index + pass criteria.

---

## 1. COUNT invariants  (all [LIGHT])

| # | Check | Pass criterion |
|---|-------|----------------|
| 1.1 | `COUNT(*) FROM tenders` | **= 6,961,311** |
| 1.2 | `COUNT(*) … island_notice_id NOT NULL` | **= 640,745** |
| 1.3 | `COUNT(*) … procedure_key NOT NULL` | **= 6,320,566** (= 1.1 − 1.2) |
| 1.4 | mutual exclusivity: `(procedure_key IS NULL) = (island_notice_id IS NULL)` | **= 0** (every tender has exactly one identity; neither-both) |
| 1.5 | tenders with no head version: `current_seq IS NULL` | **= 0** (every tender has ≥1 version; current_seq is the maintained head) |
| 1.6 | `GROUP BY kind` | only **procedure** + **registration** (CONTEXT.md: BRIN → registration) |
| 1.7 | `GROUP BY source` | expected **ted** + **doe** only; any other value = bug |
| 1.8 | `COUNT(*) FROM tender_versions` | **≈ 12.36M**, and **≥ 6,961,311** (≥1 per tender) |
| 1.9 | organizations / organization_mentions counts | both > 0; mentions ≥ orgs |
| 1.10 | `GROUP BY provisional` (organizations) | both 0 and 1 present; sane split |

## 2. Domain invariants (CONTEXT.md / ADR)

| # | Check | Tag | Pass criterion |
|---|-------|-----|----------------|
| 2.1 | islands are single-notice: `island_notice_id NOT NULL AND current_seq <> 1` | [LIGHT] | **= 0** (an island is one notice ⇒ one version; it upgrades to keyed if it gains notices) |
| 2.2 | `MIN(seq), MAX(seq) FROM tender_versions` | [LIGHT] | **MIN = 1** (seq dense from 1) |
| 2.3 | one tender per notice: a `caused_by_notice_id` in >1 tender | [HEAVY] | **0 rows** (grouping never puts a notice in two tenders; a violation = grouping bug) |
| 2.4 | head consistency: `current_published_at` ≠ the current version's `published_at` | [HEAVY] | **= 0** (issue-25 head pointer matches its version) |
| 2.5 | `current_seq` is really the max: a version with `seq > current_seq` | [HEAVY] | **= 0** |
| 2.6 | provisional ⟺ no normalised identifier: `(provisional=1) <> (identifier IS NULL)` | [LIGHT] | **= 0** (merge only on an identifier; name-only stays provisional — CONTEXT.md) |
| 2.7 | `GROUP BY identifier_kind` (organizations) | [LIGHT] | only **NULL / vat / national** |
| 2.8 | org identity uniqueness: `(country, identifier_kind, identifier)` dup among non-provisional | [HEAVY] | **0 rows** (the `organizations_identity` UNIQUE index; a dup means it failed to build) |
| 2.9 | `GROUP BY op` and `GROUP BY entity_kind` (changes) | [LIGHT] | op ⊆ {added,changed,removed}; entity_kind ⊆ {tender,lot,organization,lot_result,bid,contract} |

## 3. Data-quality / weirdness scans

| # | Check | Tag | What it catches |
|---|-------|-----|-----------------|
| 3.1 | mega-tender tail: `COUNT(*) … current_seq > {50,100,500,1000}` | [LIGHT] | **junk-hub red flag** — a tender with thousands of versions = a bad legacy-OJS transitive merge collapsing distinct procedures. Expect a thin, decaying tail; >1000 should be ~0 or a handful of genuine frameworks — inspect via 3.2 |
| 3.2 | top-30 by version count: `ORDER BY current_seq DESC LIMIT 30` | [MED] | the actual biggest chains — eyeball: legit framework/DPS vs a junk-hub (a single procedure_key or island with thousands of notices) |
| 3.3 | notices-per-tender histogram (threshold counts on current_seq) | [LIGHT] | shape sanity: most tenders 1–3 versions; islands are exactly 1 |
| 3.4 | orgs by mention count: `GROUP BY organization_id ORDER BY c DESC LIMIT 30` | [HEAVY] | over-merge: a huge count on one org may be a real big buyer OR a junk identifier merging distinct companies — eyeball the top |
| 3.5 | null rates on key fields (notice_subtype; current-version title) | [LIGHT]/[HEAVY] | subtype-null high for legacy is fine; a tender with NO title in its current version is worth counting |
| 3.6 | date sanity: `published_at` out of `[1990, ~2027]` | [LIGHT] | **= 0** absurd/zero/negative/future publication dates |
| 3.7 | amount/result sanity: negative `cents` / `awarded_cents` | [LIGHT] | **= 0** negatives |
| 3.8 | results-layer presence: lot_results / bids / contracts counts | [LIGHT] | all > 0 (award data projected); ballpark sane |

## 4. changes feed sanity

| # | Check | Tag | Pass criterion |
|---|-------|-----|----------------|
| 4.1 | `MIN(cursor), MAX(cursor), COUNT(*) FROM changes` | [LIGHT] | cursor is AUTOINCREMENT ⇒ monotonic; MAX ≥ COUNT |
| 4.2 | op / entity_kind domains | [LIGHT] | (= 2.9) |
| 4.3 | **fallback-prefix note** (not a failure): `changes` is deliberately NOT cleared on rebuild (canonical.rs:1261), so it carries BOTH the throwaway 50k-fallback run's rows AND the full rebuild's appended rows. Expect `changes` "tender/added" ≥ 6,961,311 (fallback ~226k + full 6.96M). **Harmless — no consumers yet.** Cursor never renumbered (ADR-0001). | [LIGHT] | count > one run's worth is EXPECTED, not a bug |

## 5. Referential integrity (orphans) — [HEAVY], sqlite3-idle

FK enforcement is OFF during projection (issue 19), so orphans are *possible* if a
projection bug slipped through. The authoritative check is
`PRAGMA foreign_key_check` (whole-DB or per-table) via sqlite3 on the idle file;
`checks.sql` also gives portable `LEFT JOIN … IS NULL` anti-joins per satellite
(→ tender_versions / lots / tenders / organizations). All expected **0 orphans**.
Heavy (millions of indexed probes) — sqlite3 only, never through turso.

## 6. Incremental watermark / post-plan suffix  (all [LIGHT])

| # | Check | Pass criterion |
|---|-------|----------------|
| 6.1 | unprojected parsed notices: `parse_state='parsed' AND projected=0` | **≈ 14k** — the notices parsed AFTER the plan was built, correctly LEFT for the incremental fold (not dropped). Instant (partial index). |
| 6.2 | the unprojected set is a **clean contiguous suffix** (every parsed notice at/above the first unprojected id is itself unprojected) | **the two counts are EQUAL** — proves nothing was silently dropped mid-corpus |
| 6.3 | folded (projected=1) notices vs versions | ≈ equal (each folded notice → one version); cross-checks 1.8 |

---

## Reading the results — HARD-FAIL GATES vs eyeball vs expected-noise

**HARD-FAIL gates** — must ALL pass before the layer is trusted (run_light.sh exits
nonzero if any live-checkable one misses; heavy.sql gates are marked `[GATE]`):
- §1: 1.1 (=6,961,311), 1.2 (=640,745), 1.3 (=6,320,566), 1.4 (=0), 1.5 (=0),
  1.6/1.7 (domains), 1.8 (≥ tenders)
- §2: 2.1 (=0), 2.2 (min_seq=1), 2.3 (0 rows), 2.4 (=0), 2.5 (=0), 2.6 (=0),
  2.7 (domain), 2.8 (0 rows), 2.9 (domains)
- §3: 3.6 (=0 absurd dates), 3.7 (=0 negative money)
- §5: every orphan count = 0
- §6: 6.2 (suffix is a clean tail)

**Eyeball** (judgement, not pass/fail): 3.2 (top mega-tenders — legit framework vs
junk-hub), 3.4 (top orgs by mentions — real buyer vs over-merge), 3.5 (title-null
count), 3.1/3.3 (distribution shape), 6.1 (~14k suffix), 3.8 (results present).

**Expected-noise** (document, don't fix): 4.3 fallback prefix (changes not cleared
on rebuild); high legacy `notice_subtype`-null rate (3.5a).

---

## V1 — the eForms-DE 1.x facts verification (issue 85)

Two extra scripts, scoped to the German 1.x cohort (218,635 parsed + 241 held),
to run **once proj-fix's re-fold lands**. Both are READ-ONLY and re-runnable:

- **`de1x_verify.sh`** — the statistical half. Parse-layer-unchanged gates, the
  one-version-per-parsed-notice invariant, per-fact density over 5 rowid windows
  per minor (title/description/CPV/NUTS/**lots**/buyer/subtype/amounts/dates),
  the lot-kind + lot_key-prefix gates, the island-vs-keyed split, and the
  quarantine ledger counts. Exits nonzero on any hard-fail.
- **`de1x_spotcheck.sh`** — two NAMED tenders (the issue-75 fixtures, real
  archive payloads) rendered end to end, incl. the buyer resolving to
  "Städtisches Klinikum Görlitz gGmbH", plus the REST views a user sees.

`TDB_TOKEN=tdb_… BASE_URL=http://127.0.0.1:8080 ./de1x_verify.sh`
(nginx may still be down — tunnel with `ssh -N -L 8080:127.0.0.1:8080 root@zebreus.click`).

Three things worth knowing before reading their output:

1. **Lots are checked separately and on purpose.** The issue-85 fix has two
   halves; a field-only fix would leave `tender_version_lots` at zero while every
   text/CPV check passed. `C5`, `D1`-`D4` are the half that can only pass if
   `de1_lot_kind` (the LOT-/GLO-/PAR- prefix read-back) works.
2. **Expected rates are the publisher's own rates**, from the full-corpus scan of
   all 218,876 payloads (`.scratch/de1x-scan.json`): title 100%, CPV 100%,
   NUTS 98.1%, lots 99.99%, buyer ref 100%, subtype 100%, deadline ~48%,
   any estimate ~18%. A projected rate far under those means the fold dropped facts.
3. **The "no new quarantine reason bucket" check is worthless** — see issue 87.
   `Reclaim::StillHeld` never rewrites a reason, so that check cannot fail. What
   the script reports instead is the ledger's resolved/held split (218,635/241)
   and the member list for an offline re-parse of the 241.

**Cohort-wide zero-shell gate [HEAVY]** — the sampled `C*` rates are evidence, not
proof. The exhaustive form scans 145k+72k notices × 2 index seeks, which is a
turso timeout risk on the live endpoint, so run it with stock `sqlite3` against a
post-fold snapshot (real SQLite, no hash cliff):

```sql
-- EXPECT 0: a DE-1.x version with no text at all is the issue-85 symptom
SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id = n.id
 WHERE n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2')
   AND NOT EXISTS (SELECT 1 FROM tender_version_texts x
                    WHERE x.tender_id = v.tender_id AND x.seq = v.seq);
-- EXPECT 0: same for lots (the second fix)
SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id = n.id
 WHERE n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2')
   AND NOT EXISTS (SELECT 1 FROM tender_version_lots l
                    WHERE l.tender_id = v.tender_id AND l.seq = v.seq);
```

`bin/data-quality --token …` is the third option: it already reports per-profile
title/buyer/value/cpv/deadline/winner density, so the three `eforms:eforms-de-1.*`
rows are the whole-cohort answer in one command — but its per-field
`SELECT DISTINCT tender_id, seq FROM <satellite>` is exactly the high-cardinality
shape turso cliffs on at 12.4M versions, and it has **no lots column**.

### The re-fold is a SCOPED INCREMENTAL (`project rebuild=false`) — not a rebuild

Corrected 2026-08-01 (team-lead): `rebuild:true` was ruled out — the stall is a
latent quadratic in the fold itself, which a rebuild runs too. The mechanism is
fix-the-quadratic → deploy → scoped incremental re-fold of the DE cohort only.
Three consequences for this suite:

**1. No renumber, so a control band becomes possible — and mandatory.** Only the
cohort and the TED tenders its new procedure keys merge into may change; every
other tender keeps its id. `--baseline` records a fixed 10k-wide tender id band
plus how many DE-1.x islands it holds, and `G8` asserts the band's count fell by
at most that many. Under a rebuild this check could not exist. Capture it before
the fold:

```
TDB_TOKEN=tdb_… ./de1x_verify.sh --baseline
```

(Less perishable than under a rebuild, but still the only thing the Δ arithmetic
in `G1`-`G5`/`G8`/`G9` can be checked against.) This suite's own pinned §1 gates
(`6,961,311` / `640,745` / `6,320,566`) predate the reclaims — **drop them**, they
are stale, not a baseline.

**2. `G6` is the check this mechanism needs and a rebuild would not have.** A
rebuild empties the layer first, so a notice can land exactly once. The
incremental path instead *upgrades* a notice from its island Tender to a
uuid-keyed one, and the old island is removed by `retire_regrouped_tenders`
(project.rs:931), which retires a touched Tender only when the freshly built plan
fails to reproduce its `group_key`. `tender_versions`' UNIQUE is
`(tender_id, caused_by_notice_id)` — **per Tender**, so it cannot see the same
notice holding a version in both the surviving island and the new keyed Tender.
A missed retirement silently double-counts the corpus and inflates every density
rate in section C. `G6` samples for it directly; `G7` catches the other half (a
Tender left with no head version); `G9` cross-checks the retirement count against
the `removed` tender events `retire_tender_tx` appends (canonical.rs:2915).

Both were verified to FAIL against a seeded layer (a notice given versions in two
Tenders, and an orphaned shell) — a gate never seen to fail is not a gate, which
is the issue-87 lesson.

**3. `a572544` does NOT auto-build here.** `tender_version_bid_parties_version`
is a `DEFERRED_TENDER_INDEXES` entry and that const is built by
`build_tender_indexes` at a *rebuild's* end. With no rebuild it rides the deploy
and needs the reindex op to build it. Until that runs, `/v1/tenders/{id}` stays
down and the spot-check's `?cursor=<id-1>&limit=1` workaround is the path; after
it, the detail endpoint can be used directly.

### Read path: post-refold snapshot (preferred) or the live app

Both scripts take either, running the identical SQL:

```
# preferred — immutable snapshot, no token, no live contention
nix shell nixpkgs#sqlite --command \
  env TDB_SNAPSHOT=/data/db/snapshots/<post-refold>.db ./de1x_verify.sh

# live app (needs an API token; /v1/sql is account-gated)
TDB_TOKEN=tdb_… BASE_URL=http://127.0.0.1:8080 ./de1x_verify.sh
```

Snapshot mode refuses to run if a `-wal` sibling exists: `immutable=1` skips
locking but also ignores the WAL, so a snapshot copied mid-write would report
confident numbers off a stale file. Checkpoint (TRUNCATE) before copying.

**Snapshot mode unlocks section H, which is the actual acceptance wording of
issue 85.** Real SQLite has no turso hash cliff and no 10 s request cap, so the
whole-cohort forms run for real rather than sampled — the difference between
"500 sampled notices all had facts" and "**zero** of 218,635 lack them":

| | check | gate |
|---|---|---|
| H1 | cohort versions with no text at all | **0** (the issue-85 symptom, exhaustively) |
| H2 | cohort versions with no CPV | **0** (source rate 100.0%) |
| H3 | cohort versions with no lots | **≤ 50** (~23 payloads genuinely carry none) |
| H4 | cohort notices with versions in >1 Tender | **0** (G6, exhaustively) |
| H5 | exact title/NUTS/amount/date/buyer rates | eyeball vs the source rates |
| H6 | exact Lot/LotsGroup/Part split | eyeball |

The spot-check's REST half needs the live app, so it is skipped in snapshot mode
— set **both** `TDB_SNAPSHOT` and `TDB_TOKEN` to read facts from the snapshot and
rendering from the app in one pass.
