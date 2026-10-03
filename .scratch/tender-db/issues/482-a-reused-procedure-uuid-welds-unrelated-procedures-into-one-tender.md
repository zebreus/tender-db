# 482 — a reused procedure UUID (BT-04) welds unrelated procedures into one Tender, and nothing checks the buyers

Status: ready-for-agent — NEXT: deploy unit 1 (the census + its review fixes, landed 2026-10-02, not yet deployed) → run
`procedure-key-census` (~2–3 h, read-only) → read the buckets and their samples on the portals → decide gate
(refuse the key, as 369 does) vs split (cut the Tender at a buyer-disjoint edge).
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
