# 482 — a reused procedure UUID (BT-04) welds unrelated procedures into one Tender, and nothing checks the buyers

Status: ready-for-agent — NEXT: deploy unit 2 (landed 2026-10-03 with its review fixes, below) → the next fold applies the UUID-hub gate
to what it plans; for the ~150 existing hubs re-run `procedure-key-census` (stores `hub_tender_ids`) → `requeue-uuid-hubs`
dry → wet → the next daily (or a full re-plan) splits them → verify tender 430681 is split (its notices on
`refused:<key>:<buyer>` Tenders, 430681 itself retired). Then the 60-sample precision read of the two-cluster cases.
Kind: data correctness (a false merge under the declared rule)
Relates to: 369 (procedure-key placeholder gate: `key_shaped=1` keys with ≥ 3 distinct buyer sets are refused; UUID keys
are never checked), 481 (cross-source dedup; its weld guards), ADR-0003 (a declared link merges), 12/34 (the BT-04 key)

## What is wrong

The fold groups notices by procedure key. For a UUID key, nothing else is checked: issue 369's gate applies only to
shaped (non-UUID) keys, and only at ≥ 3 distinct buyer sets. So when a publisher reuses a procedure UUID (a copied
template, a platform that reuses one BT-04, a typo-identical key), unrelated procedures fold into one Tender.
Supersession then lets the newest notice's buyer and texts win.

**Exhibit 1, tender 1110706** (read 2026-10-02):

| seq | notice | publication | buyer org | published |
|---|---|---|---|---|
| 1 | 26265518 | DÖE `0a746ecc-…-01` | 22170532 | 2024-02-05 |
| 2 | 23783071 | TED `00081139-2024` | 22170532 | 2024-02-07 |
| 3 | 24701149 | TED `00274114-2025` | **3869685 (Община Белоградчик, Bulgaria)** | 2025-04-29 |

All three carry BT-04 `f3943baf-54ae-441a-aee9-3e802998024a`. Seq 1–2 are a correct DÖE↔TED pair for a German
buyer (org 22170532, Wesel per 481's sampler). Seq 3 is a Bulgarian award notice that reused the same UUID 15 months
later. The Tender now presents `Община Белоградчик` as its buyer (`/v1/tenders/1110706` → `buyers:["Община
Белоградчик"]`), and the German procedure has lost its own identity.

**Exhibit 2, tender 42726:** 36 versions, 36 distinct TED publications (2024-09 → 2025), contract notices and award
notices, under BT-04 `083519c7-e0ad-4aab-9d71-05b39f0e55e3`. Several buyers are named (Gemeinde Blekendorf über das Amt
Lütjenburg, Stadt Bad Segeberg, …; 481's sampler counted 8). This is either a platform reusing one UUID for many
procedures, or an authority running every procedure under one key. The census decides which.

Evidence rows: `.scratch/tender-db/481-dedup/data/skeptic-collision.json`.

## First unit: census (dry job)

Over every UUID-keyed Tender with ≥ 2 notices, compare the buyer sets the NOTICES name. Use each notice's own
buyer mentions (normalised identifier, or `buyer_key()` from project.rs), not the Tender's accumulated parties. Report:
- Tenders whose notices name pairwise-disjoint buyer sets, split by: same country vs different countries; one source
  vs both; the time span between the first and the last notice;
- 30 samples per bucket for a reader;
- the class sizes. That says whether a guard is a gate (refuse the key, as 369 does) or a split (cut the component at
  a buyer-disjoint edge).

## Design direction (decide after the census)

- A notice whose buyer set is disjoint from every other notice under the key, from another country, and more than N
  months away is not the same procedure. Split it out as an island (or onto its own key), and tally it as
  `key_collisions` beside 369's `plan_refused_key`.
- Joint procurement and central purchasing bodies legitimately name several buyers. The guard must read set
  *overlap*: a notice that shares ≥ 1 buyer with the rest stays.
- Pin both exhibits as fixtures: 1110706's Bulgarian notice splits; a joint-procurement fixture stays merged.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/1110706 | jq -c '[(.versions | length), ([.parties[]? | select(.role|test("uyer")) | .organization_name] | unique)]'

- **open** (2026-10-02): `[3,["Община Белоградчик"]]`, a German procedure presented under a Bulgarian buyer.
- **done:** `[2,[<the German buyer>]]`, with the Bulgarian notice on a Tender of its own.

## Unit 1: the census — LANDED 2026-10-02 (not yet deployed)

**What landed.**
- **Job `procedure-key-census`** (`Spec::ProcedureKeyCensus`, its own family fn `run_procedure_key_census`
  through `off_frame`, in the issue-467 stack tripwire at 1 KiB future / 144 KiB poll). Read-only, no dry
  flag; stoppable between windows; report `procedure-key-census`, stored only by a finished run. Admin docs:
  `docs/operations.md`, "`procedure-key-census`".
- **The walk** (`ingest::project::key_census`): Tenders by id in windows of 5,000
  (`Db::uuid_keyed_tender_versions`: PK range on `tenders`, key shape pre-filtered in SQL, `tender_versions`
  by PK), keeping genuine-UUID keys (`is_uuid`; never `refused:`/`island:`/`ojs:`/`ocds-`) with ≥ 2 notices.
- **The overlap test is 481's**, not a second one. `GuardSide` (new, in `project.rs`) is now what
  `Ident::read` derives the plan row's guard tokens and sections from, and the census reads the same struct,
  then the resolved organizations through the shared `add_org_tokens` (`add_buyer_org_tokens` delegates to
  it), compared with `store::buyer_tokens_disjoint`. Clustering is a union-find over pairs (transitive: a
  notice joins a cluster when it overlaps any member). A notice with no parsed buyer joins no cluster.
- **The report**: `tenders`, `notices`, `notices_without_buyers`, `undecidable` (< 2 notices naming a buyer),
  `split` (≥ 2 buyer-disjoint clusters), `split_with_buyerless`, `split_same_key` (every notice under the
  Tender's own key: the BT-04 reuse itself, no 481 link in it; each cluster also says `other_keys`); buckets `one-/several-jurisdictions`
  (several = two clusters with known, non-overlapping register jurisdictions), `one-/several-sources`,
  `span-le-90d`/`-le-1y`/`-gt-1y`, each with a bottom-30-by-hash sample (tender id, procedure key, span, each
  cluster's `source:publication_id`s, first buyer and jurisdictions, and the buyerless notices); `cross` (the
  three axes together), `cluster_counts` (2, 3, 4-5, 6-10, 11+), `max_clusters` and `hubs` (the 30 Tenders
  with the most clusters).
- **Cost (reasoned from 20 bounded `/v1/sql` windows of 2,000 Tender ids, 21 requests, 0 errors):**
  UUID-keyed Tenders fill ids 1–~1.16M (1,948–1,957 per 2,000), none sampled in 1.175M–8.79M, the dailies'
  new mints at the top (8.805M: 1,166 of 2,000, nearly all single-notice); 62–64 % have ≥ 2 notices, ~3.2
  each. So ~735k Tenders, **~2.4M full parses** at ~3 ms (job 1903's endpoint parse: +1,030 s over job
  1882 for ≤ ~360k endpoints) ≈ 2–3 h; the ~1,760 window reads are minutes. No writer held.
- **Tests:**
  - `the_procedure_key_census_splits_only_buyer_disjoint_clusters` (ingest, `project_incremental.rs`):
    1110706's shape (DÖE/TED pair for Stadt Wesel + Община Белоградчик 449 days later under one BT-04, plus
    a buyerless TED notice) is 2 clusters, several jurisdictions, several Sources, span > 1 y, the buyerless
    notice listed and never a third cluster; a joint procurement (A+B / B+C / C) is 1 cluster; a central
    purchasing body's framework (Dataport + call-off buyers Hamburg, Kiel on the CN, one call-off award per
    buyer) is 1 cluster; one buyer around a buyerless notice is 1 cluster, one buyer beside a buyerless notice
    is undecidable. Same report in windows of 1 and 1,000; a stop before the second window stops it.
  - `the_procedure_key_census_job_stores_its_report_and_a_stop_stores_none` (supervisor).
  - Unit tests: transitive clustering; the bucket sample independent of arrival order.
- **Gate:** `ops/check.sh` GATE-EXIT=0, 145 suites, 859 s (dirty tree at start, so no marker; re-gate the
  commit before deploy).

**Open.** The jurisdiction is the buyer's own (`BT-514` through `register_jurisdiction`); a cluster whose
buyers carry no country counts as unknown and never makes a Tender "several". The census does not read
issue 369's refused keys (`refused:` Tenders are not UUID-keyed). It DOES walk every version of a Tender
whose surviving key is a UUID, so a 481 link weld (OPP-090 / BT-701 across keys) that the 2026-10-03 daily
has not split yet also shows as clusters; run the census after that daily, and read a sample's clusters
against their keys (`other_keys`, `split_same_key`) before calling it a BT-04 reuse.


## Unit 1 review fixes — 2026-10-02 (not yet deployed)

An adversarial review of `0b9aacf` found the report could not set the split rule's threshold. Fixed:
- **Gap axis** (major): each cluster now carries `first_published`/`last_published` (`YYYY-MM-DD`),
  `sources` and `gap_days` (distance of its time range from the largest cluster's, 0 when they overlap);
  the sample's `gap_days` is the smallest minority gap, bucketed `gap-le-90d`/`-le-1y`/`-gt-1y`; `cross` is
  now jurisdictions × Sources × gap. New totals `interleaved` / `sequential` (gate vs split shape) and
  `singleton_minorities` (the island shape). The Tender span stays as a secondary axis.
- **`unknown-jurisdiction`**: a third value; `one-jurisdiction` now means every pair shares a known
  country. Blank BT-514 codes are dropped before `register_jurisdiction`.
- **Source axis per cluster**: several only when two clusters' Sources are disjoint; 1110706 (a TED
  notice against a DÖE/TED pair) is now `one-source`, as it should be.
- **Bounded samples**: at most 20 clusters (`clusters_total`) and 40 buyerless notices
  (`without_buyers_total`) per sample; the hub list ranks by `clusters_total`.
- **SQL pre-filter trims** (`length(trim(procedure_key)) = 36`), as `is_uuid` does.
- **Tests**: the census fixture adds an interleaved hub, a countryless-buyer cluster (DÖE-only beside
  TED-only: several Sources, unknown jurisdiction), an OPP-090 link-joined split (`other_keys` 1,
  not `split_same_key`), and a padded key; a unit test pins the sample caps.
- **Docs**: the job holds the single worker for its run (queue it right after a daily), and a restart
  re-runs it from id 0.

**Deferred** (ready-for-agent, not blocking the census run):
- No checkpoint/resume: `run_procedure_key_census` ignores `resume_after`, so a deploy mid-run costs the
  whole 2–3 h again. Worth adding only if the census is re-run routinely.
- Clustering is O(n²) for a Tender whose notices are all pairwise disjoint (fine at the observed max of
  372 notices; a pathological Tender of tens of thousands would cost seconds of CPU). The report row is
  now bounded; the CPU is not.
- A UUID group a link merged under a non-UUID key is not walked (documented as out of scope).
- 2026-10-03 04:0x UTC: the Verify exhibit **1110706 reads `[2,["Stadt Wesel"]]`** after 481's fold. Its Bulgarian
  notice was joined by an opp-090 placeholder and is now refused buyer-disjoint, not by the shared BT-04. The census
  (unit 1, built and gated at 75dd23f) still runs for real BT-04 collisions, e.g. 526284 (Tenerife + SCB under
  `73206638-…`).

## 2026-10-03 10:xx UTC — census read (job 1927, 7,968 s); decision: refuse UUID hubs (unit 2)

- 756,457 UUID-keyed Tenders with 2 or more notices (2.37M notices). **2,879 (0.38 %) split** into buyer-disjoint
  clusters: 2,778 with every notice under the Tender's own key, 481 interleaved, 2,398 sequential, 1,734 where every
  minority cluster is a single notice.
- Cluster counts: 2,730 with 2, 75 with 3, 25 with 4–5, 28 with 6–10, 21 with 11+ (max 319).
- **Hubs are platform-wide UUID reuse.**
  - 430681: 789 notices from 319 Swiss buyers under one BT-04 (ASTRA, the Hochbauamt, Stadt Zürich, …).
  - 440935, 737405, 1004011, 204280 and 489950 are Finnish (Hansel, universities, agencies).
  - 126427, 442018, 429083 and 461158 are Polish municipalities.
  - 928341 is Bavarian Landkreise.
- **The two-cluster cases are mixed.** Real collisions: Olkusz / Siewierz, Ministerie BZK / BuZa, POLREGIO / UZP.
  One body under two names: Health Insurance Organisation / Οργανισμός Ασφάλισης Υγείας. Several-jurisdiction cases
  are often one EU body filing from two countries (EEAS BE/XK, EP LU/FR).
- **Decision (owner).**
  - Unit 2 refuses a UUID key whose notices form **3 or more** buyer-disjoint clusters. This extends issue 369's
    placeholder gate to UUID keys with the 481 buyer_guard overlap, so each notice falls to `refused:<key>:<buyer>`.
    That covers about 150 Tenders, including the 789-notice Swiss hub.
  - Two-cluster splits are NOT acted on yet. A 60-sample hand read must first measure their precision; the
    name-variant share decides.

## Unit 2: the UUID-hub gate — LANDED 2026-10-03 (not yet deployed)

**What landed.**
- **The gate** (`Db::refuse_uuid_hubs`, called by `build_plan_groups` after 369's and 386's refusals, before the
  batched election). It refuses a UUID key (`store::is_uuid_key`, `key_shaped = 0`) whose planned notices form
  `store::UUID_HUB_CLUSTERS` (3) or more buyer-disjoint clusters. The clustering is the census's:
  `store::buyer_clusters` (moved from `key_census`, which now calls it) over `plan_notice.buyer_guard`, using
  `buyer_tokens_disjoint`, transitive. Notices without buyer tokens join no cluster and never count. The plan is
  read in two streamed passes, never a grouped aggregate. Pass 1 keeps the first two distinct guard-set digests per
  key digest; a third distinct digest makes the key a candidate (~40 B a UUID key with a buyer). Pass 2 reads the
  candidates' notices whole and clusters them exactly.
- **Group keys.** A refused key goes into `plan_refused_key`, like 369's. Each clustered notice is point-written
  `refused:<key>:<label>`, where the label is the cluster's smallest `buyer_key` (`#g<token>` if none). A buyerless
  notice becomes its own `refused:<key>:#n<notice_id>`. A legacy notice with an OJS self number stays with the
  legacy closure.
- **Tally:** `PlanGroupTally::uuid_hubs` / `Report::uuid_hubs` (`keys`, `notices`, `clusters`). The fold job row
  gets `; issue-482 uuid hubs refused: …` (silent at zero), and the journal logs `group step uuid-hubs` on every
  grouping.
- **Incremental equality.** 369's gate did NOT see the full key on a daily. A refused key names no Tender, so
  `touched_existing_tender_ids` (exact `procedure_key IN`) never found its `refused:` Tenders. A new notice under a
  refused key was planned alone and founded a bare-key Tender that a full fold never makes. Fixed for every refused
  key (369, 386, 482):
  - the touched expansion range-seeks `refused:<key>:` on `tenders_procedure_key` and on `tender_key_merges`
    (`REFUSED_TENDERS_SQL` / `REFUSED_MERGE_TARGETS_SQL`, plans pinned in `the_tender_link_statements_seek`);
  - `record_key_merges` now records `refused:<key>:<label>` → `<to_key>` when an issue-481 link absorbs a refused
    group (it used to drop them), and an incremental plan replaces the `refused:<key>:` rows of each key it holds.
  - `ingest::project::refused_sibling_closure` runs after the link closure. When a plan reaches a `refused:<key>:`
    Tender some other way (a new key's OPP-090 into one hub cluster, or a re-parse's old membership), it plans every
    Tender of that key (`Db::refused_keys_of_tenders`, then the touched lookup) and closes their links. It repeats to a
    fixpoint, with the link cap and full fallback. Without it, the test's step 4 diverged: the daily un-refused the hub.
  - When a key crosses the threshold on a daily, the touched bare-key Tender is not reproduced, so
    `retire_regrouped_tenders` retires it (`removed` events, no ghost).
- **Re-planning existing hubs.** The census report now carries `hub_tenders_total` / `hub_tender_ids` (every Tender
  with ≥ 3 clusters, ascending, cap 10,000). The new job **`requeue-uuid-hubs`** (`Spec::RequeueUuidHubs { dry_run }`,
  dry by default, `off_frame` with stack gauges) reads that list from the stored report and re-queues the Tenders'
  notices (`Db::requeue_tender_notices`: `tender_versions` seeks, then `requeue_notice_ids`). It refuses without a
  report, or with a pre-unit-2 report that has no `hub_tender_ids`. Docs: `docs/operations.md`, "The UUID-hub gate
  and `requeue-uuid-hubs`".
- **Tests:**
  - `a_uuid_hub_splits_per_buyer_cluster_on_full_and_daily_folds` (ingest, `project_incremental.rs`) compares a full
    fold and the daily step by step (content and change events). The fixture:
    - a Swiss-hub-shaped key with 2 buyers stays merged at step 0, then crosses to 3 on a daily and splits per buyer.
      ASTRA's cluster stays merged with another key's Tender through an OPP-090 link (`refused:` group recorded in
      `tender_key_merges`);
    - a second hub with no links crosses on the daily: its Tender is retired, with a `removed` event and no ghost;
    - a later ASTRA notice and a buyerless one under the hub key are folded by a daily that finds the split Tenders
      by prefix. The buyerless notice gets `refused:<key>:#n…`;
    - a new key whose OPP-090 cites one hub cluster's notice joins that cluster's Tender, and the hub stays split;
    - a 2-cluster key (Olkusz / Siewierz), a joint procurement and a CPB framework each stay one Tender;
    - afterwards the census lists no hub, a re-queue (dry / wet) plus a daily reproduces the split, and a full
      re-plan moves nothing.

    A mutation check (prefix lookup disabled) fails the test at step 2.
  - `requeue_uuid_hubs_reads_the_census_hub_list_and_refuses_without_one` (supervisor). The census test pins
    `hub_tender_ids` empty for its 2-cluster corpus.

**Open.**
- A key whose clusters merge back below 3 is un-refused: a later bridging joint notice can collapse them. On a
  daily, its buyerless `#n` Tenders are re-found by the prefix, so the daily and a full fold agree. A 369
  shaped-key island (`island:<id>`) is still not re-found that way. That gap is pre-existing and rare.
- An admitted same-Source OPP-090 into a buyerless notice can still bridge two clusters of a refused key into one
  Tender. That is issue 481's link step (a tokenless endpoint is unknown, not disjoint), not this gate.

## Unit 2 review fixes — 2026-10-03 (not yet deployed)

An adversarial review of `8b76651` found two majors and five minors. Fixed:
- **(major) A hub cluster merged into another key's Tender lost the hub's rows.** A daily reaching that
  Tender by its own key (a new notice under the link partner) planned one cluster only, and
  `record_key_merges`' range delete dropped the hub's OTHER clusters' merge rows; the next daily under the
  hub then missed a cluster, counted 2 and welded the third into a link partner's Tender. Now
  `Db::refused_keys_of_tenders` also reads the `tender_key_merges` rows pointing AT each touched Tender
  (`MERGED_INTO_SQL`, new index `tender_key_merges_to`, pinned), so the sibling closure plans the hub key
  whole; the range delete runs only for UUID-hub keys the plan holds whole (non-legacy, `key_shaped = 0`),
  and other refused groups are replaced exactly (the plan's own pre-merge `refused:` group keys). Test
  `a_hub_cluster_merged_into_another_keys_tender_keeps_the_hub_whole_on_the_daily` (two clusters merged
  into KEY_A and KEY_B, a KEY_A daily, then a hub daily; compares `tender_key_merges` too). Mutation (no
  to_key read) fails it at step 1.
- **(major) 369/386 keys were planned whole on every daily.** Now only UUID-hub keys
  (`is_uuid_hub_class`: uuid, not placeholder-shaped) get the prefix expansion and the sibling closure.
  369/386 keys: an incremental grouping seeds `plan_refused_key` with every placeholder/FTS key of the plan
  that already has a `refused:<key>:` Tender or merge row (`seed_split_refused_keys`; their gates only grow),
  and a changed notice pulls in only its own `refused:<key>:<buyer_key>` Tender (or the Tender a link merged
  that group into) — `Db::refused_family_tender_ids(family_keys, group_keys)`. Test
  `a_split_placeholder_key_is_planned_per_buyer_on_the_daily` pins the plan size (3, 1, 1 notices) and
  full/daily equality. Mutation (no seed) fails it at step 1.
- **(minor) `refused:<key>:#n` was a keyed member in the link step.** `link_rank_class`: it is an island
  for the weld guard and the fan-in count, and ranks between keyed and islands (so `keyed(root)` stays exact
  and an island-only component keeps the `refused:` name). The new merged-hub test's DÖE twin rejoins its
  TED notice by logical-notice; mutation fails it at step 0.
- **(minor) Label churn.** The cluster label is now the `buyer_key` of its smallest-notice-id member that
  has one, so a later notice never renames it (unit test `a_uuid_hub_cluster_keeps_its_first_members_label`).
- **(minor) O(n²) clustering.** `store::buyer_clusters` unions per token digest (FULL holder joins all;
  ACRONYM + INITIALS holders join each other), exactly the pairwise `buyer_tokens_disjoint` components —
  pinned against the pairwise oracle over 3,000 random sets
  (`buyer_clusters_match_the_pairwise_overlap_graph`). The `group step uuid-hubs` line logs the largest
  candidate's notice count.
- **(minor) Census list vs gate.** Documented as Tender-scoped vs key-scoped in `docs/operations.md`, with
  the not-walked case (hub groups merged under a non-UUID key: full re-plan only); the job summary says so.
- Also: legacy OJS-chain notices under a UUID key no longer count toward its clusters (they fold into the
  legacy closure, which a daily cannot reach by the key's prefix).

**Deferred** (ready-for-agent):
- **':' in keys** (minor 5): the `refused:<key>:` range of a key `k` also covers a key `k:x`'s groups, and the
  base-key parse reads `refused:k:x:<buyer>` as `k`. Narrowed, not closed: the prefix machinery now runs
  only for UUID-hub keys (no ':'), and the 369/386 seed skips keys holding ':'. A UUID-hub key `K` whose
  range meets a BT-04 literally `K:…` would still over-plan (harmless) and range-delete that key's merge rows.
  A real fix is an escaped or length-prefixed group key, which renames every refused Tender — not worth it
  until such a key is seen.
- **369/386 un-refusal on a daily.** A re-parse that shrinks a seeded key below its threshold leaves it
  refused on the daily until the next full re-plan (the seed is monotonic by design).
- **Stale `refused:` merge rows** for a 369/386 group that vanished (its last notice re-parsed to another
  buyer) stay in `tender_key_merges`; they only widen a later plan (the table's documented safe direction).
- **Pre-deploy merge rows.** 369/386 refused groups a link absorbed before this deploy have no
  `refused:` row (the old code dropped them) until the next full re-plan; a daily notice of such a buyer
  founds its own group meanwhile. UUID hubs are unaffected (none exist before the deploy).
- **Prod sizes unmeasured.** The reviewer asked for bounded `/v1/sql` counts of the largest `refused:`
  families before deploy; the read was not run this session (prod reads not permitted). With 369/386
  planned per buyer the remaining exposure is UUID hubs (census max 789 notices) — measure the
  `group step refused-keys (seeded)` and `refused-sibling closure` journal lines on the first daily.
