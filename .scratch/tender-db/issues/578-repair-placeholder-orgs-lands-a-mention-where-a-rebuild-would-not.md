# 578 — repair-placeholder-orgs (issue 440) re-resolves a dissolved org's mentions on the raw country with a bare LIMIT 1, not the resolver's key

Status: **DONE 2026-10-24** — U2 (6e73ed0) is deployed in a63cf23, 11:32Z; Verify `git grep -c Legacy440 <live rev>` = 0. U4's issue exists (582). Was: ready-for-agent — **U4 DONE 2026-10-24: filed as 582** (5,076 pre-358 provisional rows under mapped overseas codes; 30920766 → 2660729 is its first member). U2 is gated (6e73ed0) and ships in the 10-28 batch. NEXT: once U2 is deployed, run Verify (`0`); then 578 is DONE (closure: U2 deployed and U4's issue exists). Was: ready-for-agent — **U2 BUILT and GATED 2026-10-23** (6e73ed0, `DissolveTarget::Parity`; `grep -c Legacy440 canonical.rs` = 0; gate green at 406529a), on main, NOT deployed: deploys in the batch after 539's drain (decision-w-on-drains), then the twin re-gate. U4 (file the 358 follow-up) still open. Was: ready-for-agent — TRIAGED 2026-10-21 (wf triage-575-578: measurer + 2 adversarial skeptics; 2 + 2 bounded /v1/sql
reads answered, 2 more lost in SSH transport, 0 of 3 GETs answered, no 408/429; skeptic-corrected, `## Triage (2026-10-21 …)`
below). Reproduced in code at HEAD 969afba: 440's own lookup (`DissolveTarget::Legacy440`, canonical.rs:41875-41939) is not the
resolver's no-identifier branch (:23619-23834) in four shapes — it keys and mints on the RAW country (no `register_jurisdiction`),
mints one NULL-country singleton per named country-less mention (no 351 reuse), mints nameless mentions under the raw country,
and skips the 513 U4 nhs-body alias. The issue's lowest-id twin shape is latent on turso 0.7.2 (both probes seek
`organizations_name_country` in rowid order). Operator-only job (no daily/weekly caller); last wet run 2489 on 2026-10-14.
Damage, measured lower bound: 1 surviving 440-minted mapped-code row, 30920766 (MQ "rectorat de martinique", job 835, 4
mentions; its FR key row 2660729 stands); the reuse half and the country-less singletons are not measurable (440 keeps no
per-mention log). **D578-1 DECIDED**: edit Legacy440 in place into a resolver-parity lookup (register key + `ORDER BY id` + one
mint per key; the 351 arm; nameless mints under the folded country; nhs-body left as a residual) and rename it — not the bare
`Resolver` arm, whose `resolver_target` refuses nameless/country-less mentions and would roll back whole windows. NEXT: U2 build
(canonical.rs, a 2-argument signature change, tests); code may land now (551 U7 drained, U8 PASS); its deploy re-gates 551's
switch-off twin (C7) and waits for 539's drain. U4 files the 358 follow-up (5,076 frozen mapped-code rows; 30920766 → 2660729
rides it). Was: needs-triage — filed 2026-10-20 from issue 530 Build 2's review round 2 (the OQ4 follow-up its build plan
proposed and never filed). Next: unit 1, decide whether 440's own lookup (`DissolveTarget::Legacy440`) switches to the
resolver arm (`DissolveTarget::Resolver`, as issue 530's wet run uses), and measure what a re-run of a past 440 cohort
would land differently.
Kind: data quality (organization identity): a dissolve and a rebuild disagree on where a mention binds; low severity —
440 runs only on reviewed condemned classes
Relates to:
- 440 (RESOLVED-VERIFIED 2026-09-28): `repair-placeholder-orgs`, whose per-org body issue 530 Build 2 factored into
  `Db::dissolve_org_mentions`.
- 530: Build 2's wet run re-resolves through `DissolveTarget::Resolver` — the issue-234 resolver arm's key
  `(org_name_norm(name), register_jurisdiction(country))`, `ORDER BY id LIMIT 1`, one mint per key — so its mentions
  land where a fresh fold would put them. 440 kept its own lookup unchanged (Build 2's plan, "440 stays byte-identical").
- 234 / 432: the resolver's reuse key.

## Verify

    grep -c "Legacy440" crates/store/src/canonical.rs

- **done**: `0` — U2 landed: 440's dissolve runs D578-1's parity lookup and no code or comment keeps the old variant's
  name (the grep reads the tree; U2 itself closes on its deploy).
- **open (2026-10-21, HEAD 969afba)**: `7` — the variant (:45171), `dissolve_condemned`'s call site (:41436), the
  `None if matches!(key, DissolveTarget::Legacy440) => 0` winner arm (:42061) and four doc/comment lines. No other file
  under crates/, docs/ or CHANGELOG.md names it.
- Replaced 2026-10-21 (triage): the earlier verify (`grep -n "DissolveTarget::Legacy440)\|&mut DissolveTarget::Legacy440"`)
  also counted "keep Legacy440 and say why in its doc" as done, which D578-1 rejects.

## What was found

- `Db::dissolve_org_mentions` under `DissolveTarget::Legacy440` (the `else if !name_norm.is_empty() && country.is_some()`
  branch) looks a moved mention's target up as `name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1` with the
  mention's RAW country and no `ORDER BY`; a mention with no name or no country gets a fresh provisional row each.
- The resolver keys the same lookup on `register_jurisdiction(country)` with `ORDER BY id LIMIT 1` and reuses one row
  per key. So an RE/GP/MQ mention, or a key with two identifier-NULL rows, can land on a different row under 440 than a
  fresh fold or a from-archive rebuild would bind (issue 530's A11 shape, which Build 2 fixed for its own path only).
- Two comments claimed the parity before review round 2: `repair_placeholder_orgs_batch`' "re-resolves through the
  post-234 provisional path ((name_norm, country) reuse)" and "the resolver's reuse key (issue 432), so a re-resolved
  mention lands where a fresh fold would put it" above all three `DissolveTarget` branches. Round 2 rewrote both to name
  the difference and point here.
- Not affected: issue 530's wet run and restore (`Resolver`, `Fixed`).

## Proposed units

1. Decide: switch `dissolve_condemned` to `DissolveTarget::Resolver` (its whole-row residue refusal and its "winner with
   no target" refusal come with it), or keep `Legacy440` and say why in its doc. Measure first: a dry 440 plan over the
   placeholder classes it ran on, comparing the two lookups' targets per mention.
2. If switched: the `placeholder_dissolve` tests pinned to the resolver's target (register country, lowest id).

## Triage (2026-10-21, wf triage-575-578)

Read-only: no cargo, no writes, no git state change. Code read at HEAD 969afba (= origin/main: the 551 flip e718684 plus its
U7/U8 records). The measurer's first line numbers matched the pre-flip tree (d209375, which the switch-off twin d569107 on the
local branch `hold/551-switchoff` restores); HEAD's are about 30 higher, and every number below is HEAD's. Reads:
`.scratch/tender-db/578-triage/` (probe*.sql, r1.json, r3.json). The skeptic's k1/k2 stayed in the session scratchpad
(`/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/578-skeptic/` k1.json, k2.json,
probe.sql); their results are below.

### Evidence

Code path:
- Entry: `/admin/jobs` kind `repair-placeholder-orgs`, dry by default (supervisor.rs:5211-5226) → `Spec::RepairPlaceholderOrgs`
  (:11335) → `repair_placeholder_orgs_batch(condemns, ORG_NAME_FIELD_IDS, normalize_lang, BACKFILL_BATCH, watermark, dry_run)`
  (:11342; canonical.rs:41198, one `BEGIN IMMEDIATE` window per batch, ROLLBACK on any error) → `dissolve_condemned` (:41418) →
  `dissolve_org_mentions(.., &mut DissolveTarget::Legacy440, ..)` (:41436 → :41482).
- No scheduled caller: `enqueue_daily` (supervisor.rs:19996) and the weekly tick push none; the only other constructor is a
  supervisor test (:23329). It is on the heavy-writer list (:20145).
- Legacy440 (canonical.rs:41875-41939). The mention read takes only `notice_id, section_id, name, country` (:41504).
  - Named + country: `SELECT id FROM organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1`, bound to
    the mention's RAW country (the mention keeps the published code, :23642-23646), no ORDER BY; a miss mints under the raw
    country (:41896-41916).
  - Anything else (no name or no country): a fresh row per mention under the raw country (:41919-41939).
  - No per-window cache: in a dry run every same-key miss counts `fresh` again — 365's documented dry/wet artifact (365:134;
    839 dry 11,507 fresh vs wet 863).
  - Winner arm `None if matches!(key, DissolveTarget::Legacy440) => 0` (:42061).
- Rebuild path: `mentions_and_dropped` (project.rs:5888, gated :5911) → `gated_identifier` (:5322) → `normalise_identifier`
  (:11835) → `idgate::condemns` (:12003). A condemned id is None, so the mention takes `resolve_one_mention`'s `(None, None)` arm
  (canonical.rs:23619-23834):
  - the country folds through `register_jurisdiction` (:23647; fn :7395: GP/MQ/GF/RE/YT/PM/BL/MF/WF→FR, AX→FI, GL→DK, SJ→NO);
  - the 513 U4 nhs-body alias first (:23650-23679; armed only by an `nhs-body` ledger row);
  - named + country: per-fold cache, then `... ORDER BY id LIMIT 1`, else ONE mint per key under the folded country
    (:23680-23727);
  - named, no country, `resolver.norm` set (prod: `mention_resolver(.., Some(match_norm), .., STOPLIST_CAP)`, project.rs:2269 and
    :3622): the 351 arm (:23728-23819). `echo_tier_on` (:12824) memoized per name; generic or a failed probe → mint NULL-country
    (a failed probe that ended the transaction returns the error, issue 501, :23759); under the wall → reuse via
    `ORG_COUNTRYLESS_NAME_PROBE_SQL` (:3135, lowest id), else one mint per name (:23805-23816);
  - everything else (nameless; or named, no country, no norm) → mint under the folded country (:23820-23833).
- `resolver_target` (530's `Resolver` arm, :42182) refuses a mention with no name or no country (:42195-42201).
- Foreign keys: the store opens with `PRAGMA foreign_keys = ON` (store lib.rs:158), checked per statement (turso 0.7.2 has no
  `defer_foreign_keys`, lib.rs:1926-1927). `organization_id` REFERENCES organizations(id) on tender_version_parties (:409),
  organization_mentions (:473), tender_version_result_winners (:926) and tender_version_bid_parties (:988).
- Plan probe (measurer: `target/debug/plan-probe` on a copy of v551/plan.db with the deferred org indexes, turso =0.7.2): the
  Legacy440 probe and the resolver probe both plan `SEARCH organizations USING INDEX organizations_name_country (name_norm=? AND
  country=?)`. Rowid order, so a bare LIMIT 1 already returns the lowest id (the resolver's own comment says so, :23685-23690).
- Fixture: `crates/store/tests/placeholder_dissolve.rs:78-99` keys CZ/SI/DE (none fold); its one nameless mention (102) also has
  no country; :327 pins `(fresh, reused) = (13, 1)`. Bare `Resolver` refuses mention 102; the parity lookup mints it under NULL,
  as Legacy440 does.

Box reads (/v1/sql, each plan-probed locally; no 408/429):
- R1 (measurer) `SELECT id,country,identifier_kind,identifier,provisional,created_at,name_norm FROM organizations WHERE country IN
  (12 mapped codes) LIMIT 300` (organizations_country_id): 300 rows, the query's own LIMIT (`truncated:false`). AX 37 + GF 263,
  all provisional, identifier NULL, created_at 1786785861 (the 2026-08-15 rebuild).
- R3 (measurer, `probe4.sql` r_combined, `truncated:false`): AX 37, GF 348, GL 14, GP 941, MF 3, MQ 1,108, PM 6, RE 2,433, SJ 1,
  WF 10, YT 176, BL 0 = 5,077 rows, 0 with an identifier. Exactly one created after 1788739200 (2026-09-07): 30920766, MQ,
  `rectorat de martinique`, created 1788924539 = 2026-09-09 03:28:59 UTC, 4 mentions. 358 counted 5,127 provisional rows under
  these codes on 2026-09-07 (358:115-118): 51 have gone since, 5,076 remain.
- k1 (skeptic) `SELECT notice_id, section_id, name, country, raw_identifier, scheme FROM organization_mentions WHERE
  organization_id = 30920766 LIMIT 20`: 4 rows (notices 22487869, 22487870, 22937294, 22938275, each ORG-1), "rectorat de
  Martinique", MQ, raw `2022`, scheme national. `2022` is idgate's `bare_four_digit` class, made condemning by 365 unit 2
  (idgate.rs:97, :1321); 839 dissolved LEITWEG/BERICHT (365:226-236), so 835 minted the row. A rebuild gates `2022` to None and
  keys the 4 mentions on (rectorat de martinique, FR).
- k2 (skeptic) `SELECT id, country, identifier, provisional, created_at FROM organizations WHERE name_norm = 'rectorat de
  martinique' LIMIT 50`: 2660729 (FR, identifier NULL, provisional, 2026-08-15), 14624875 (FR, SIRET 17972430700030, not
  provisional), 30920766 (MQ). The resolver's key row stands: 2660729.
- Lost: 2 measurer SSH attempts ("Connection closed by UNKNOWN port 65535", rc 255, before the endpoint) and all 3 public-API GETs
  (HTTP 000 / reset). Reading stopped there.

Records:
- Wet dissolves through Legacy440, all completed: 835-837 (365, 2026-09-09: 259,342 mentions, 4,982 fresh), 839 (365: 73,021,
  863 fresh), 1615 (440, 2026-09-28: 1,008, 81 fresh; 440:225), 2489 (522, 2026-10-14: 7, 2 fresh; 522:171).
- 530 Build 2 plan (session scratchpad `530-build2-plan.md:183-184`, OQ4 :508-510): "440 stays byte-identical in this build, and
  a follow-up issue fixes 440" — this issue.
- 473:163-166 names `repair-placeholder-orgs` as one option for dissolving its DB file-number class.

### Root cause

530 Build 2 factored 440's body into `dissolve_org_mentions` and kept 440's lookup byte-identical as `Legacy440` (OQ4). That
lookup predates 358 (register country), 439 (`ORDER BY id`) and 351 (country-less reuse), so it differs from the resolver's
no-identifier branch in four shapes:
1. **Overseas codes (live; the only frozen split).** It keys and mints on the raw code. An RE/GP/MQ/… mention re-attaches to one
   of the 5,076 pre-358 rows under a mapped code, or mints a new mapped-code row (seen once: 30920766). A fold or rebuild binds
   the FR/FI/DK/NO row. The resolver never probes a mapped-code row (:23647-23695), and 434's `publishes` (canonical.rs:15644)
   compares only name, country, raw identifier and scheme, so a recorded mention never moves back.
2. **Named country-less mention (live, unmeasured).** One NULL-country singleton per mention, where the resolver's 351 arm reuses
   the standing row for a name its echo tier admits. Healable on demand: `fold-provisional-echoes` (`Db::fold_provisional_echoes`,
   canonical.rs:37879; admin only, supervisor.rs:5089, no schedule) folds identical-name NULL-country provisional rows under the
   wall into the lowest id.
3. **Nameless mention.** Both sides mint per mention: Legacy440 under the raw country, the resolver under the folded one.
4. **NHS bodies.** No 513 U4 alias (GB reviewed body spellings only).

Also:
- The issue's twin shape (bare LIMIT 1 vs `ORDER BY id`) is latent on turso 0.7.2 (plan probe). It goes live only if a planner
  change picks another index; 457 (turso 0.8.1) is that risk (the 0.8.x plan note at canonical.rs:3128-3134).
- The `=> 0` winner arm is a latent window abort, not a bad write: on a wet run the INSERT of org 0 fails its FK and the window
  rolls back; a dry run counts the row in `winners`. Tier 5 could not rescue that window either. A winner has no target only
  when tier 2 or tier 4c took its section from a party or bid-party row on the loser (:41663-41693, :41711-41730) whose mention
  is not the loser's. That row follows no moved mention (the UPDATEs at :41985 and :41998 key on a moved mention AND
  `organization_id = loser`), so the org's DELETE fails its FK anyway. Not seen: every wet run above completed.
- Not a defect: tier 3 keys on the raw country (`c.3 == first.3`, :41776) in code shared by every arm. The same raw code gives
  the same register key and so one target; under parity it stays correct and is only stricter (more rows go to tier 5).

### Size

- Measured (R3 complete, `truncated:false`): shape 1's mint half is 1 SURVIVING row, 30920766 (4 mentions), across every
  Legacy440 wet run since 358. A lower bound: R3 sees only rows that still exist.
- Not measurable (440 writes no per-mention log, unlike 530's `guid_twin_dissolve_log`): shape 1's reuse half (835 alone reused
  254,360 targets; some may be legacy mapped-code rows) and shape 2. All Legacy440 mints together: 5,928 (4,982 + 863 + 81 + 2),
  mostly named-with-country misses; shape 2's share of them is foldable by `fold-provisional-echoes`.
- Twin shape: 0 live.
- Context, not caused by 440: the 5,076 pre-358 identifier-NULL rows under mapped codes (R1's 300 all provisional, all from the
  2026-08-15 rebuild). → U4. Re-take: `probe4.sql`'s summary half (a `organizations_country_id` range per code).
- Code change: S-M (one branch in canonical.rs, a 2-argument signature change, tests). Severity: low.

### Decision D578-1 (owner, 2026-10-21; amended by the skeptics)

Replace `Legacy440` with a resolver-parity lookup, scoped to the resolver's no-identifier branch. Edit Legacy440's two lookup
branches in place (canonical.rs:41875-41939) and rename the variant — skeptic 2's smaller fix; a third mode beside it buys
nothing.
- (a) Named + country: key `(org_name_norm(name), register_jurisdiction(country))`, `ORDER BY id LIMIT 1`, else ONE mint per key
  under the register country. A per-window key cache (as `ResolverTargets.cache`), so a dry plan's `fresh` equals the wet run's.
- (b) Named, no country: the 351 arm. `echo_tier_on(conn, name_norm, match_norm(name), STOPLIST_CAP)`, memoized per name;
  generic → mint NULL-country; under the wall → `ORG_COUNTRYLESS_NAME_PROBE_SQL` reuse, else one mint per name. A failed probe
  mints, unless `transaction_ended_under` says turso ended the transaction (issue 501; the dissolve runs inside `BEGIN
  IMMEDIATE`) — then return the error.
- (c) Nameless: mint under the folded country.
- (d) Winner with no target: delete the `=> 0` arm so the mode falls through to the existing refusal (:42065-42071), with a
  job-neutral message (it says `dissolve-guid-twins` today). Extend the residue check (:42148, `matches!(key, Resolver(_))`) to
  the mode. Both only swap an FK abort that already happens for a clear message.
- Left alone: `resolver_target` and its refusals (530), the shared tier 3, the `Fixed` arm.
- Residuals, recorded and not built: (i) the 513 U4 nhs-body alias (needs the resolver's ledger state; GB body spellings only);
  (ii) `dissolve_org_mentions` reads only name and country (:41504), never `raw_identifier`, so a mention that reached a condemned
  org through a canon/anchor/altid bind while carrying its own valid identifier would bind by that identifier in a rebuild, under
  any dissolve mode. Unexamined; reopen if a future 440 cohort shows such mentions.

Alternatives:
- Keep Legacy440 and document it: rejected. Every future run (473) re-attaches overseas mentions to frozen pre-358 rows or mints
  raw-code rows (30920766), and mints country-less singletons that need a separate fold.
- Bare `DissolveTarget::Resolver`, as the issue proposed: rejected. `resolver_target` refuses nameless/country-less mentions, so
  any 440 window holding one rolls back whole (fixture mention 102 is one).
- One shared function for `resolve_one_mention` and the dissolve: right long term, but it refactors the hot fold path for a job
  run a few times a year. Not now.

### Units

- U1: decide (S). Status: **DONE 2026-10-21** (D578-1, this section). Blocked by: nothing.
- U2: build (S-M). Status: ready-for-agent. Blocked by: nothing for code (551 U7 drained, U8 PASS, 969afba). Its deploy: 539's
  drain finished, and 551 C7 (rebuild and re-gate both twins on the deployed base first, 22-35 min).
  - canonical.rs: D578-1 (a)-(d). Rewrite the docs on `dissolve_condemned` (:41405-41416), `dissolve_org_mentions`
    (:41467-41469), the enum (:45164-45171) and the comment at :41854-41858; no code or comment keeps the old name (the Verify
    counts it).
  - Signature: `repair_placeholder_orgs_batch` gains `norm: fn(&str) -> String` and `stoplist_cap: usize` (store cannot call
    ingest). Callers: supervisor.rs:11342 (`ingest::project::match_norm`, `ingest::idgate::STOPLIST_CAP`) and every call in
    placeholder_dissolve.rs. The supervisor test at :23320-23335 goes through `run_spec` and needs no change.
  - Put the 351 probe in a helper and Box::pin it, like `resolver_target` (CLAUDE.md, the run_spec stack note; the arm at :11335
    is already boxed).
  - Tests. Add cases; the existing fixture should pass unchanged (CZ/SI/DE do not fold, mention 102 mints under NULL either way,
    all 13 named keys distinct):
    - an RE mention → the lower id of two identifier-NULL FR rows;
    - two RE mentions with no FR row → ONE mint under FR, and dry `fresh` 1 = wet `fresh` 1;
    - a country-less name under the wall → the standing NULL-country row; a generic one → a mint;
    - a nameless RE mention → a mint under FR;
    - a winner whose tier-2 pick is a foreign party row → the refusal, nothing of the window kept.
  - Gate: `ops/check.sh` (it enables `server`; read the `GATE-EXIT=` value). Deploy: no job run needed; the change takes effect
    on the next 440 run.
- U3: repair 30920766. Status: folded into U4's issue (skeptic 2: the same shape and the same remedy as the 5,076 legacy rows).
  Blocked by: —.
  - The move: rehome its 4 mentions (k1) onto 2660729, the (rectorat de martinique, FR) identifier-NULL row, then sweep the empty
    row. NOT a country move MQ→FR (`apply-country-verdicts`): 2660729 already stands on that key, so a move would create the
    identifier-NULL twin shape. NOT onto 14624875 (the SIRET row): the resolver's identifier-less key binds 2660729; whether
    2660729 and 14624875 merge is registry/R2 work.
- U4: file the 358 follow-up (S, docs only). Status: ready-for-agent. Blocked by: nothing. Next free number in your band (579 is
  taken), announced first.
  - Shape: "5,076 pre-358 provisional rows under mapped overseas codes never fold to the register country".
  - Evidence: R3 here (per-code counts, 2026-10-21; re-take with `probe4.sql`'s summary half). 358:120-122's unit-2 premise
    ("fold naturally as mentions re-project") is contradicted by `resolve_one_mention` probing only the register key
    (:23647-23695) and by 434's `publishes` (:15644) keeping recorded mentions in place.
  - Its first unit: one bounded read of those rows' mentions (a per-code count through `organization_mentions_org`,
    plan-probed); then per row, a rehome onto the register-key row where one stands (k2's shape), else a country move. 30920766 →
    2660729 is its first member. Bar: 0 rows under mapped codes created after 2026-09-07 (R3's row half).
  - It lands before 473 dissolves through this job. Relates to 358, 434, 473, 578.
- Closure: 578 is DONE when U2 is deployed (Verify `0`) and U4's issue exists.

### Interactions

- **551 B6.** The flip e718684 is on main and deployed; U7 drained (3,467 fired) and U8 PASSed (969afba); the switch-off twin
  d569107 sits on `hold/551-switchoff` only. No textual overlap: the flip's hunks are canonical.rs 1987-5040 and tests from
  47439, and supervisor.rs 774 (the Spec enum); U2 touches canonical.rs 41198-42251 and 45160-45185, supervisor.rs ~11342, and
  store tests. Code may land; its deploy follows C7. A 440 wet run, and U4's rehome, move party and winner rows between orgs,
  which C9 counts as path-W inputs (org merges feed R1/R6): not on a U9 check morning, and record their tender ids for C7's
  switch-off list.
- **539.** Its drain (repair-notice-instants + the DÖE R3 refold) waited on 551 U8 and is next on the single writer. U2's deploy
  waits for it, and no `repair-placeholder-orgs` run goes during it (a heavy writer; each window holds the writer).
- **530 (DONE).** `resolver_target` (:42182) and its refusal stay as they are (530's planner, wet path and tests rely on them),
  and so does the shared tier 3 (530's tier-3 tests, guid_twins.rs:5728 and :5809). The residue check U2 extends is the same SQL.
  The 530-T gate in `mentions_and_dropped_gated` is not involved (a 440-condemned id is already None by `idgate::condemns`,
  project.rs:12003). guid_twin.rs and guid_twins.rs are untouched.
- **473.** A likely caller (473:163-166). U2, and U4's issue, come before 473 dissolves through this job. After U2 an RE mention
  it moves lands on the FR key while its siblings stay on a frozen legacy RE row; that entity is already split today, so it is
  not a regression, and U4 closes it.
- **358 / 434**: U4. **351**: `fold-provisional-echoes` heals shape-2 singletons on demand. **457** (turso 0.8.1): the latent
  twin shape goes live only under a plan change; U2's `ORDER BY id` removes it first.

### What the skeptics corrected (each checked against code or files before recording)

- **Line numbers and main.** The measurer's numbers were the pre-flip tree's; HEAD 969afba = origin/main is about 30 higher.
  "d569107 landed on main" was wrong: it is on `hold/551-switchoff` only.
- **551's gate has passed** (969afba: U7 drained, U8 PASS). U2's deploy waits only on 539's drain and C7.
- **The `=> 0` arm** is a latent FK window abort (lib.rs:158, canonical.rs:926), not a write onto org 0; a dry run only counts
  it. The residue refusal likewise only renames an FK abort.
- **Tier 3** keying on the raw country is not a defect (shared, stricter); dropped from the units.
- **"Never heal"** holds for shape 1 only; shape 2 is foldable by `fold-provisional-echoes`, and the 5,928 bound overstates live
  damage.
- **The 351 arm** also mints when no standing row exists (:23805-23816) and runs only with `resolver.norm` set (armed on prod,
  project.rs:2269/3622); without it a named country-less mention falls to the plain mint (:23820).
- **R1's 300** is the query's LIMIT (`truncated:false`). **"Exactly 1 row"** is 1 surviving row, a lower bound; the Status line
  says so.
- **The FR twin exists** (k2): 2660729, plus the SIRET row 14624875. U3's target is 2660729, by rehome rather than a country
  move, and the repair moves to U4's issue.
- **U2's signature**: `match_norm` and `STOPLIST_CAP` must be passed in, and a failed probe keeps the issue-501
  `transaction_ended_under` check.
- **The fixture** should pass unchanged under parity, so "re-pin" became "add cases".
- **Smaller fix adopted**: edit in place and rename, not a separate mode.
- **Residual added**: the dissolve never reads `raw_identifier` (:41504).
- **Dropped, not confirmed by the code**: skeptic 2's "send a winner with no target to tier 5 instead of refusing". Such a
  winner's section comes from a party or bid-party row on the loser whose mention is elsewhere; that row follows no moved mention,
  so the org's DELETE fails its FK regardless. Tier 5 cannot unstick the window, so the refusal stays.

**551 W-on drain protocol (2026-10-23, `551-lot-sum/b6/decision-w-on-drains.md`).** U2 is class D (no fold change). It ships
in the batched deploy, with 579 U4 as its own commit beside it, one twin re-gate (last) and P1. The batch runs after 539's
drain and the 10-28 day-7 fetch. It ships only if it fits before that gap's latest switch-off start, and only with ≥ 15 GB
free before each gate (4.5 GB on 10-23). The 358 follow-up (5,076 frozen rows; 30920766 → 2660729), every 440/473 run and
519 M6 are class I: in-place W-input writers, since org identity feeds R1/R6. They wait until after 11-04. Each then runs
under 551's C9': an offline per-id port prediction on R ∩ touched (the dry plan's moves on cached facts, port 23cbd850); a
refold-lot-sum of the touched cohort Tenders in the same segment (built in once 579 U4 ships); and gd5 class I with D1
semantics at tolerance 0. An unpredicted fire, or a wrong S, currency, n or vouched_by, means switch off. Cost: U2 adds
nothing beyond P1; each follow-up costs about 10–20 statements and under 1 h. **Next step:** U2's deploy waits for the batch
slot, not only for 539's drain. U4's filed issue carries `Blocked by: 551 C9' (after 11-04)`.
