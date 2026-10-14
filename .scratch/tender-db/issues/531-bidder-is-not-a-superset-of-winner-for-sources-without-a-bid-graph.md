# 531 — `?bidder=` is not a superset of `?winner=` for sources without a bid graph, although the served contract says it is

Status: ready-for-agent — 531 unit 1 BUILT 2026-10-14 (the "superset of winner" claim corrected on `/docs` and `/v1/openapi.json`, and in the `read.rs` and `api.rs` comments; RETIRED guard plus a positive-half test; two-reviewer round applied; focused tests green; see "Unit 1 — built"). Next: gate + deploy with the 522/524 batch, then read the Verify (expect outcome (a) until unit 2 decides). Then unit 2, the decision.
Kind: docs (the served contract), plus an API-semantics decision
Relates to:
- 217 (RESOLVED 2026-08-16/17): shipped `bidder=` with "a superset of `?winner=`" as its acceptance line. It was
  never measured on legacy, DÖE sdk-0.1 or FTS data. This is where the claim comes from.
- 393 (done 2026-09-28): line 228 already saw legacy winners that `?bidder=` cannot find ("tried `?bidder=` and
  `?buyer=` and concluded they were orphans; `?winner=` finds them"). It was recorded in passing and never filed.
- 484 (done 2026-10-07): unit 3 narrows `winner=` (drops `is_buyer` rows) and leaves `bidder=` alone on purpose.
  That is consistent with a superset and does not touch this gap.
- 388 (DONE 2026-09-18): the seeded pages walk ONE `(organization_id, tender_id)` index in windows of tenders. Any
  widening of `bidder=` must keep that bound.
- 416 (DONE 2026-09-18): the SSE snapshot walks the seed in windows too. A union seed would need the same treatment
  there.
- 370 / 391 (DONE 2026-09-18): the class of hand-written served claims that no gate re-derives. This is one more
  instance; neither issue covers the `bidder` wording.

## Verify

    curl -s "https://tenders.zebreus.click/v1/tenders?bidder=28338347&limit=100" "https://tenders.zebreus.click/v1/tenders?winner=28338347&limit=100" "https://tenders.zebreus.click/v1/openapi.json" | jq -c 'if .components then .components.parameters.bidder.description else [.items[].id] end'

It prints three lines: the `bidder=` ids, the `winner=` ids, and the served `bidder` description.
- **done**: one of these, depending on unit 2's decision.
  - (a) Contract corrected: line 3 no longer says "a superset of winner" without qualification. It names legacy TED
    and DÖE sdk-0.1 as winner-only and FTS as winners-only. Lines 1 and 2 still differ.
  - (b) or (c) Bidder widened: line 1 equals line 2, `[3885939,4116961,4194547]`.
- **open (2026-10-14)**: `[]`, then `[3885939,4116961,4194547]`, then
  `"Organization id that submitted a bid (a tenderer) on the Tender, won or not — a superset of winner."`

## Repro

Public GETs, 2026-10-14 (`B=https://tenders.zebreus.click/v1`):
- `curl -s "$B/tenders?winner=23871844&limit=100" | jq -c '[.items[].id]'` gives
  `[109781,232871,404673,534274,8842898]`. The same with `bidder=` gives `[]`. All five are DÖE sdk-0.1 awards,
  2023-02 to 2026-06.
- Org 6586990: `tenders?winner=` 29 items, `tenders?bidder=` 8. `lots?winner=&limit=100` returns 100 with
  `more: true`; `lots?bidder=&limit=100` returns 46 with `more: false`.
- Tender 7256352: published 2023-10-09 as `605902-2023` (TED R2.0.9, not eForms), 98 winner parties, `bids` empty.
  It is missing from `tenders?bidder=22092824`.
- Tender 4116961 (TED, 2009-10-22): 17 winner parties (4 distinct orgs), `bids: []`.

## What was found

The two filters read different tables (`crates/store/src/read.rs:1101-1121`):
- `winner=` is an EXISTS over `tender_version_result_winners` (`is_buyer IS NULL`);
- `bidder=` is an EXISTS over `tender_version_bid_parties` with `role = 'tenderer'`.

Both are tender-scoped at the current version, on `/v1/tenders` and `/v1/lots`. SSE and webhooks share the same
`Filter`. The SQL is correct for its table. The gap comes from where bid rows are written, which depends on the
source schema:
- **eForms bid graph** (`LotTender`/`TenderingParty`, `crates/ingest/src/project.rs` ~6745, ~6835): TED eForms
  and eForms-DE (DE1 maps `OPT-300-Tenderer`, project.rs:1332). Losing tenderers are present and the superset holds.
- **FTS publishes no bid graph.** `crates/ingest/src/fts/parse.rs:328-350` builds one `LotTender` +
  `TenderingParty` per `awards[].suppliers[]` entry. OCDS `bids` is read only for statistics (:421). So on FTS
  the bidders are exactly the winners. The superset holds by construction, but "won or not" is false: a losing
  FTS bidder is never served. On 7 FTS tenders from id 8800000, tenderer rows equal winner rows on every one, while
  the received-bid count is often higher (8800000: 1 vs 2; 8800005: 1 vs 6; 8800003: 6 vs 7).
- **Legacy TED** (`is_legacy_profile`, project.rs:9063: `text`, `internal-ojs`, `ted-export*`) goes through
  `read_legacy_results` (:6874), whose doc says "there is no bid/contract graph in the legacy schema".
  **DÖE `eforms:eforms-sdk-0.1`** (`is_sdk01_profile`, :9079) goes through `read_sdk01_results` (:7013). Neither
  writes any bid row. Every win from these profiles is served under `winner=` and never under `bidder=`.

The served contract makes the claim in four places (grep for "superset of", 2026-10-14):
- `/docs`: `crates/app/src/v1/docs.rs:171`, "won or not, a superset of `winner`";
- `/v1/openapi.json`: `crates/app/data/openapi.json:749`, "won or not — a superset of winner.";
- the `Filter.bidder` doc at `read.rs:185`, "a superset of `winner` (issue 217)";
- the seed-precedence comment at `read.rs:1231`, "winner rows ⊆ bidder rows". This one only orders the seed (the
  EXISTS decides membership), so it is harmless for correctness, but it is wrong.

`docs.rs:309` already mentions "a legacy era with no bids", so `/docs` contradicts itself.

Measured 2026-10-14 (bounded reads, by the verifier and the refute):
- **Recent window, tenders 8842000–8843659:** 797 winner rows (304 tenders, 486 orgs). 795 have a tenderer row in
  the same `(tender, seq)`. The 2 misses are DÖE sdk-0.1 awards with `bids: []`: 8842898 (org 23871844, published
  2026-06-30) and 8842988 (org 31958824, 2026-07-20).
- **eForms-era window 950000–951999:** all 5,299 TED winner rows (1,230 tenders) have a tenderer row. The 14 DÖE
  winner rows (12 tenders) have none. The 4 tenders opened are sdk-0.1, published 2023-08 to 2025-09.
- **Legacy window 5320000–5321999 (TED 2015):** 4,712 winner rows across 1,327 tenders, 0 with a tenderer row.
- **10 random winners from the recent window (seed 20261014), via the API:** 3 of 10 orgs lose wins under
  `bidder=`: 8097510 loses 3 of 7, 6586990 loses 21 of 29, 22092824 loses 1 of 3 (25 of 86 won tenders). Every lost
  tender is a legacy TED notice, 2015-04 to 2023-10.
- **Org 23871844 loses its whole history**: all 5 wins are sdk-0.1. So the gap hits DÖE suppliers as well, not
  only suppliers with a long TED history.
- **Not measured:** the corpus-wide share of winner rows with no tenderer row. That needs a corpus scan, which has
  no compliant path from the agent. An in-app census could produce it if unit 2 needs the number.

The gap follows the schema, not the date. TED R2.0.9 award notices run into 2024 (8536068, 336327-2024, published
2024-06-07, 60 winners, 0 bids; the "October 2023" in the first sample was only that sample's edge), and DÖE still
publishes sdk-0.1 in 2026.

### What the external review got wrong

- "Wins from before eForms never show up under bidder" understates the scope. Legacy TED R2.0.9 notices up to
  2023-10 and DÖE sdk-0.1 awards into 2026 are in the gap too. Inside the eForms bid graph (TED eForms, eForms-DE)
  the superset holds (795/797 recent; both misses are sdk-0.1).
- "33 of 40 sampled winners" cannot be reproduced, because the orgs were not named, and the rate depends entirely
  on how winners are sampled. A random sample gave 3 of 10 orgs and 25 of 86 tenders. Do not quote either figure as
  a corpus rate.
- It is not a bug in the filter SQL. Each filter reads its table correctly; the tables are filled by different
  source schemas.
- Do not re-file the first-pass wording "bid rows come from a published bid graph (TED eForms, eForms-DE, FTS)".
  FTS has no bid graph; its tenderer rows are synthesized from award suppliers, so FTS `bidder=` is winners only.

## Proposed units

1. **Correct the served contract (S).** Change all four copies: `docs.rs:171`, `openapi.json:749`, `read.rs:185`
   and `read.rs:1231`. Re-grep for "superset of" first in case another copy appeared. Proposed wording, roughly:
   "An organization named as a tenderer on a published bid (TED eForms, eForms-DE), won or not. On FTS, an award
   supplier only: FTS publishes no losing bidders. Legacy TED (up to October 2023) and DÖE sdk-0.1 publish winners
   without bids, so a win there appears under `winner` only. To get every tender an organization took part in,
   query both." (As built, the date became the format list and "query both" was narrowed; see "Unit 1 — built".)
   If unit 2 later picks (b) or (c), unit 2 rewrites this sentence again. Until then, the served
   claim is false today and this fix is correct.
2. **Decide whether `bidder=` should also count winners (M, measure first).** A winner did submit a tender, so the
   inference is sound. Record the decision here. CONTEXT.md line 59 lists `bidder` as a participation role, so a
   change of meaning needs an ADR note.
   - (a) **Docs only.** Unit 1 is the whole fix. Cheap, but a consumer needs two queries.
   - (b) **`bidder` = EXISTS tenderer OR EXISTS winner (`is_buyer IS NULL`).** The superset becomes true by
     construction, with no data change. Cost: everything below assumes one table per seeded org.
     - 388's seeded pages (`lots_seeded_page`, `tenders_seeded_page`) walk one `(organization_id, tender_id)` index
       in windows. A union needs two window walks merged by `tender_id`, with no un-windowed DISTINCT over the
       union.
     - `participation_seed`, `participation_role` (read.rs ~1249–1285) and `without_seeded_org` (~3790) map the
       seed to a single table.
     - `reachable` (read.rs:1362) probes one table per org leg.
     - The 416 SSE snapshot and the webhook matcher.

     Time org 357 (3.55M winner rows, 194k bid rows) through the union shape against 388's closing matrix
     (`/v1/lots?bidder=357&limit=100` 0.71 s, `/v1/tenders?bidder=357` 0.90 s) before choosing it. Neither those
     controls nor the 416 snapshot may regress. The detail's `bids[]` stays as published either way.
   - (c) **Synthesize bids at fold time** for legacy and sdk-0.1 winners. FTS already does this at parse time
     (`fts/parse.rs:328-350`), so the codebase has a precedent and the option is not wrong in principle. It is the
     least attractive option on cost and side effects:
     - it needs a re-parse and refold of the legacy cohort (issue 393 sized it at 6,998,915 notices) plus the
       sdk-0.1 cohort;
     - synthetic bid rows with no BT-720 enter `bids[]` and every bid-count and bid-value read, `/v1/sql`
       included.
3. **Gate the claim (S).** Add tests in the gate (`ops/check.sh`) that re-derive the served sentence from behaviour
   (the 370 class), whatever unit 2 decides:
   - a legacy-profile award fixture and an sdk-0.1 award fixture: the winner appears under `bidder=` under (b) or
     (c), or is pinned as absent under (a), next to the documented wording;
   - an FTS fixture whose `bidder=` set equals its `winner=` set, so "won or not" cannot drift back over FTS;
   - an eForms fixture with a losing tenderer: present under `bidder=`, absent under `winner=`.

## Unit 1 — built (2026-10-14)

The re-grep for "superset of" in `crates/` and `docs/` found the four copies listed above and one more: the doc
comment of `tenders_can_be_filtered_by_bidder` (`crates/app/tests/api.rs`). The other hits (`project.rs:4422`,
`partyname.rs`, `canonical.rs`, `read.rs:2521`, two store tests, `operations.md`) describe other sets and are
correct. No other copy appeared.
- **Served:** the `/docs` bidder row (`docs.rs`) and `components.parameters.bidder` (`openapi.json`) now read:
  "Organization id named as a tenderer on a published bid (TED eForms, eForms-DE), won or not. On FTS, an award
  supplier only: FTS publishes no losing bidders. The pre-eForms TED formats (text, internal-ojs, r2.0.8, r2.0.9)
  and DÖE sdk-0.1 publish winners without bids, so a win there appears under `winner` only. To find every tender
  where an organization is named as a tenderer or a winner, query both (issue 531)." The OpenAPI copy says "a
  published bid of the Tender". (Wording after the review round below.)
- **Comments:** the `Filter.bidder` doc (`read.rs`) names the source of bid rows per format. The seed-precedence
  comment no longer claims "winner rows ⊆ bidder rows". It now says the precedence is an order, not a containment.
  The `api.rs` doc says `bidder` does not find every winner.
- **Guard:** RETIRED in `no_served_surface_repeats_a_retired_claim` gains `a superset of winner` (the OpenAPI
  spelling) and `a superset of <code>winner</code>` (the `/docs` HTML spelling). The positive half is the new test
  `the_docs_say_bidder_misses_the_wins_of_sources_without_bids`. It reads the bidder row and the parameter, and
  each must carry the corrected sentences as phrases (eForms "won or not", the FTS award-supplier sentence, the
  pre-eForms format list with sdk-0.1, and the narrowed "query both" with its buyer carve-out).
- **Checked against the code for the wording:** the filter SQL (`read.rs`: `winner` reads
  `tender_version_result_winners`, `bidder` reads `tender_version_bid_parties` with role `tenderer`).
  `read_results` (`project.rs`) returns early into `read_legacy_results` or `read_sdk01_results`, and neither
  pushes a bid; only the eForms path pushes a `RawBid` per `LotTender`. `fts/parse.rs` builds one
  `LotTender` + `TenderingParty` per `awards[].suppliers[]` entry. The rule is by format (`is_legacy_profile`:
  text, internal-ojs, `ted-export*`), not by date.
- **Not done here:** unit 2's decision and unit 3's behaviour fixtures.
- **Review round (2026-10-14, two reviewers):** the first draft said "Legacy TED (up to October 2023)", but R2.0.9
  award notices with winners and no bids ran into 2024 (8536068, 336327-2024, published 2024-06-07), so the text
  now names the formats instead. "To get every tender an organization took part in" overclaimed, since no query
  finds a losing bid that was never published; it now reads "To find every tender where an organization is named
  as a tenderer or a winner". The `/docs` lookup example ("everything they ever bid on" over a lone `?bidder=`)
  now asks `?winner=` and `?bidder=`, and that phrase joined RETIRED. The `Params.bidder` comment (`v1/mod.rs`)
  no longer says "won or not" for every source. The retired walk now squashes whitespace on every surface, so a
  retired sentence wrapped across source lines is still caught. The positive test now pins one phrase per
  sentence of the bidder row, not single tokens that another sentence also carries. Focused tests green.
