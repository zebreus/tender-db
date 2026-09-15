# 367 — 218,876 notices published "1970-01-01", 7,417 with NULL, and every date-only publication shifted: one resolver, run at the wrong layer, returning a scalar

Status: UNITS 1-2 DONE 2026-09-07 (owner) — fix deployed (`4b40606`) and the 226,293 standing rows repaired (jobs 798 dry / 799 wet). Unit 3 (the offset/has_time triple and the date-only rendering) remains ready-for-agent: a date-only publication still serves a shifted instant. Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings)
Kind: defect (parse → canonical instants; the notice layer vs the version layer)
Relates to: 18 (fixed this gap for sdk-0.1 by adding SDK01-* to these two field lists),
85 (fixed it for DE-1.x FACTS by aliasing — which is projection-only, hence this), 255
(the decided_utc / decided_offset / decided_has_time triple that already does the right
thing on another axis), ADR-0013

## Observed

**DE-1.x: 100% of the dialect, 0% of its neighbours.** Bounded id-window group-bys. Window 26,200,000–26,260,000: `eforms-de-1.1` n=31,162 / published_at=0 for **31,162**; `eforms-de-1.0` 26/26; while `eforms-sdk-0.1` 28,609/**0** and `eforms-sdk-1.0` 204/**0** in the same window. Window 26,320,000–26,380,000: de-1.1 28,517/28,517, de-1.2 2,601/2,601, sdk-0.1 28,733/0, sdk-1.0 132/0, sdk-1.10 18/0. Cohort via the profile index: de-1.0 31 + de-1.1 145,859 + de-1.2 72,986 = **218,876**.

Named notice: `SELECT id, source, profile, published_at, dispatched_at, parse_state FROM notices WHERE id=26244735` → `doe`, `eforms:eforms-de-1.1`, **0**, NULL, `parsed`. Its own stored content carries the dates: `notice_dates` → `DE1-RequestedPublicationDate` 1704841200 (offset 60, has_time 0) and `DE1-IssueDate` 1704841285 (offset 60, has_time 1) = 2024-01-10 +01:00. Its VERSION is right: `tender_versions WHERE caused_by_notice_id=26244735` → published_at 1704841200, dispatched_at 1704841285 — byte-identical to the two DE1 values. `curl /v1/notices/26244735` serves `"published_at":"1970-01-01T00:00:00Z"`, `"dispatched_at":null`.

**A contiguous TED prefix with NULL.** `SELECT COUNT(*), SUM(published_at IS NULL), MIN/MAX(id) … WHERE id BETWEEN 1 AND 50000` → 40,016 rows, **7,417** NULL, ids 1..9,984; `… id BETWEEN 9985 AND 3000000 AND published_at IS NULL` → **0**, so it is one prefix, not a scatter. All `parse_state='parsed'`: ted/sdk-1.12 896, sdk-1.13 4,535, sdk-1.14 1,986 — so not the documented identity-only case. Their versions are fine (notices 1/2/3 → published_at 1784239200 with real dispatched_at). Their `ingested_at` = 1784490077 = 2026-07-19T19:41Z, one day before issue 18 shipped.

**Date-only publications get an invented time, in two conventions.** `/v1/tenders?limit=6&source=doe&sort=published_at&order=desc` → four rows at `2026-09-05T22:00:00Z`, two at `2026-09-04T22:00:00Z`; an unsorted doe page → `2024-02-05T00:00:00Z` (131), `2024-09-18T00:00:00Z` (153), `2023-02-16T00:00:00Z` (159) beside `2025-06-24T22:00:00Z` (151) and `2026-07-12T22:00:00Z` (204). The split is by profile at the value level: 131/26263780 `eforms-sdk-0.1` (`SDK01-RequestedPublicationDate`, offset_minutes **0**, has_time 0); 151/26688196 `eforms-de-2.0` (`BT-738-notice`, offset_minutes **120**, has_time 0). Consequence: tender 7954578 serves published_at `2026-09-04T22:00:00Z` with dispatched_at `2026-09-05T17:32:26Z` — **published a day before it was dispatched**. Not doe-only: TED notices 1–3's versions carry 1784239200 = `2026-07-16T22:00:00Z`, the same +02:00 shift.

## Why, exactly

One resolver — `notice_instants` (`crates/ingest/src/project.rs:4990-4998`) — with three separate faults.

**(a) It runs at a layer where the vocabulary has not been folded.** It matches only the ids in `PUBLICATION_DATE_FIELDS` (`crates/ingest/src/project.rs:390-397`) and `DISPATCH_DATE_FIELDS` (`crates/ingest/src/project.rs:401-408`); neither list has a `DE1-*` entry. The translation exists — `("DE1-Publication-PublicationDate","OPP-012-notice")`, `("DE1-RequestedPublicationDate","BT-738-notice")`, `("DE1-IssueDate","BT-05(a)-notice")` at `crates/ingest/src/project.rs:527-529` — but it is applied by `normalise_de1` (`crates/ingest/src/project.rs:4341`), called at exactly five sites, **all inside the projection**: project.rs:1321, 1886, 1995, 2156, 2565. The processor resolves the notice row's own dates from the RAW, un-normalised parse — `crates/ingest/src/process.rs:319-323` — so nothing matches. Then `.unwrap_or(0)` at `crates/ingest/src/project.rs:4996` turns not-found into the unix epoch; dispatch has no such fallback, which is why `dispatched_at` is NULL on the same rows. The version is right because the fold reads the same resolver *after* `normalise_de1` ran on its chunk.

**(b) The notice row is stamped once and never re-derived.** `notices.published_at` was added by `MIGRATIONS` (`crates/store/src/lib.rs:628-629`), an ADD-COLUMN-only array by policy ("Anything beyond ADD COLUMN stays out of scope by policy", lib.rs:622-627) — no UPDATE ever fills pre-existing rows. The only writers are the process-time INSERT (`crates/store/src/lib.rs:2469-2481`, fed by `resolved_notice` at process.rs:312-337) and the two re-parse/reclaim UPDATEs (lib.rs:1894-1896, 1968-1971). A cleanly-parsed notice reaches neither: reclaim's already-parsed arm short-circuits with "Already good — never re-touch a parsed notice" (`crates/store/src/lib.rs:1938-1948`). The projection writes back `projected` alone (canonical.rs:3947, 4754, 4847, 5172). So the 2026-08-15 rebuild refreshed those 7,417 notices' `tender_versions` and left the notice rows NULL forever. The resolver's own doc claims the opposite — "Shared by the processor (which stamps the notice row) and the projection (which stamps the version), so both agree" (`crates/ingest/src/project.rs:4988-4989`) — and nothing asserts it. It also falsifies the schema's "Null until the payload is parsed (identity-only rows)" (`crates/store/src/lib.rs:149-151`): these rows are parsed.

**(c) The Date triple is flattened to a scalar on this axis only.** The parse layer stores `(utc_seconds, offset_minutes, has_time)` (`crates/store/src/lib.rs:327-336`), and `timestamp` computes `days_from_civil(y,m,d)*86400 + seconds - offset*60` (`crates/ingest/src/eforms/value.rs:180-182`) — so a date-only value carrying a published `+02:00` becomes local midnight re-expressed in UTC = 22:00Z (23:00Z in winter). Then `first_date` destructures `NoticeValue::Date { utc_seconds, .. }` and throws the other two away — `crates/ingest/src/project.rs:5000-5005` — `notice_instants` returns a bare `(i64, Option<i64>)`, and both canonical columns are plain INTEGER (`crates/store/src/canonical.rs:186`, `crates/store/src/lib.rs:152`). With the offset gone the API cannot recover the publisher's civil date, so it renders through `instant()` (`crates/app/src/v1/json.rs:18-23`, used for published_at at json.rs:66, 118, 306) instead of `stamp()` (`crates/app/src/v1/json.rs:28-41`) — the renderer built for exactly this case, which returns the bare date when `has_time` is false and which `submission_deadline` already uses (hence `"submission_deadline":"2024-03-06T10:00:00+01:00"` beside a Z-shifted published_at in the same payload). The two "conventions" are not two policies: `offset_or_utc` appends a literal `Z` for `SDK01-`, `UBL-` and `OPT-999` fields whose lexical value carries no offset (`crates/ingest/src/eforms/value.rs:145-153`, the documented sdk-0.1 relaxation at :122-138), giving offset 0 and T00:00:00Z, while every other field keeps the publisher's offset — and `source=doe` ingests both. The r209/text era publishes no offsets at all (`crates/ingest/src/r209/value.rs:3-8`), which is the third route to T00:00:00Z.

**Not our artifact, for the record:** the workspace has no timezone database (`Cargo.toml:69` pins `chrono = { version = "0.4", default-features = false, features = ["std"] }`; no tz/jiff/chrono-tz crate anywhere), and the offset only ever comes from `split_offset` reading the published text (`crates/ingest/src/eforms/value.rs:194-213`). CONTEXT.md:139 records the intended representation as "timestamps as UTC + original offset" — which is what the parse layer does and this pipeline undoes.

## Units

1. Resolve the notice row's instants from the same normalised parse the projection uses: call `normalise_de1` before `resolved_notice` (`crates/ingest/src/process.rs:319-323`), or add the `DE1-*` ids to the two field lists (the 18/85 precedent). Then replace `.unwrap_or(0)` (`crates/ingest/src/project.rs:4996`) with an `Option` — a not-found publication date is NULL, never 1970.
2. Backfill and assert: a job that re-derives `notices.published_at` / `dispatched_at` for parsed rows whose stored value disagrees with the resolver (the 218,876 + 7,417), plus an assertion in the projection that the notice row and its version agree — the invariant project.rs:4988-4989 already claims.
3. Carry the triple on the publication/dispatch axis (255's shape): offset + has_time columns on `notices` and `tender_versions`, and render published_at/dispatched_at through `stamp()`. This is also what makes `/docs`' "never an invented time" (`crates/app/src/v1/docs.rs:113`) true.

## Done when

- `/v1/notices/26244735` serves 2024-01-10 and notices 1–3 carry a published_at;
- a doe date-only tender serves `2026-09-05`, not `2026-09-04T22:00:00Z`;
- no tender serves a published_at earlier than its own dispatched_at (7954578 is the probe);
- a test fails if the notice row's instants and its version's diverge.

*One issue because:* all three defects are the same six lines (`notice_instants` / `first_date`) plus the one call site that reads them too early — the epoch dates, the NULL prefix and the shifted date-only values are one resolver used at two layers, run once, and stripped of the offset it was given.

## Units 1-2, built and run (2026-09-07)

**The fix.** `notice_instants` returns `(Option<i64>, Option<i64>)` — the `.unwrap_or(0)` that
flattened "not found" into 1970 is gone — and the two date lists now name the `DE1-*` dialect
ids as well, each placed immediately after the eForms id it aliases. Adjacency is the load-bearing
part: the processor resolves from the RAW parse (the notice layer keeps the publisher's own ids
on purpose) while the projection resolves after `normalise_de1`, so an id ranked differently from
its own alias target would make the two layers resolve two DIFFERENT real dates. Pinned by
`every_de1_date_alias_sits_beside_its_target` and
`the_raw_and_the_normalised_parse_resolve_the_same_instants`, with the corpus invariant
`notices_and_their_versions_carry_the_same_instants` on top. `tender_versions.published_at` is
NOT NULL and the fold orders by it, so the two projection call sites keep an explicit epoch
fallback for the dateless notice — that column's nullability is unit 3's business.

**The repair.** `repair-notice-instants`, dry by default, plan stored as report
`notice-instant-repair`; the wet arm refuses a plan that was itself wet, aborts outside a
max(2%, 5) tolerance, rewrites only rows still matching their pre-image, and writes no change
events (the versions were never wrong, so nothing is re-projected).

**Dry (job 798) reproduced the review's independent measurement exactly:**

| | |
|---|---|
| parsed notices walked | 14,346,064 |
| already agree with the resolver | 14,119,771 |
| planned | **226,293** |
| stamped the epoch | **218,876** |
| NULL while the parse states a date | **7,417** |
| parse states NO date, value REMOVED | **0** |

By profile the two classes separate cleanly: the epoch class is entirely eForms-DE
(`eforms-de-1.0` 31, `-1.1` 145,859, `-1.2` 72,986 = 218,876) and the NULL class entirely TED
eForms (`eforms-sdk-1.12` 896, `-1.13` 4,535, `-1.14` 1,986 = 7,417). The subtractive class the
repair reports separately turned out to be **empty** — no parsed notice in the corpus states no
date at all — so nothing was removed.

**Wet (job 799)** applied the plan. The issue's own probes now pass:

- notice 26244735 (`eforms-de-1.1`) serves `published_at` 1704841200 = **2024-01-10**, the
  `IssueDate` the review found in its payload;
- notices 1, 2 and 3 carry real instants instead of NULL;
- a bounded window of half a million ids holds **0** parsed notices at the epoch and **0** NULL.

Not done, and still true: `/v1` renders through `instant()`, so a date-only publication still
serves `2026-09-04T22:00:00Z` for a source date of 2026-09-05. That is unit 3.

## Comments

### 2026-09-15 — API/data-quality review fan-out: date-only publications served as shifted instants, quantified (unit 3, still live — not a regression)

Four lenses of the 2026-09-15 fan-out (api-tender-detail, api-notices, dq-dates,
dq-islands-sdk01-fts) independently hit unit 3's defect. The adversarial judge ruled it is
this issue, not a new one, and **not** a regression of units 1-2: the Status line above
already says unit 3 "remains ready-for-agent: a date-only publication still serves a shifted
instant", and no commit since `4b40606` touches the rendering in `json.rs` / `project.rs`.
Status is therefore left alone. What is new here is the *measured rate* of the
dispatched_at > published_at inversion, which the issue asserted only from the single probe
7954578.

**Notice layer** (bounded id window, ids 26,200,000–26,250,000):

```
ssh root@zebreus.click 'echo "SELECT profile, count(*) AS notices, sum(dispatched_at IS NULL) AS disp_null, sum(dispatched_at > published_at) AS disp_after_pub FROM notices WHERE id BETWEEN 26200000 AND 26250000 GROUP BY +profile" | /root/sq.sh'
```

| profile | notices | dispatched_at NULL | dispatched_at > published_at |
|---|---|---|---|
| `eforms:eforms-de-1.0` | 21 | 0 | 0 |
| `eforms:eforms-de-1.1` | 25,869 | 0 | **22,418 (86.7%)** |
| `eforms:eforms-sdk-0.1` | 23,955 | 17,264 | **6,454 (96.5% of the 6,691 carrying a dispatch)** |
| `eforms:eforms-sdk-1.0` | 156 | 0 | 0 |

**Version layer** (current versions only, bounded tender ids):

```
ssh root@zebreus.click 'echo "SELECT n.profile, count(*) AS tenders, sum(v.dispatched_at > v.published_at) AS dispatch_after_pub, sum(v.dispatched_at IS NULL) AS no_dispatch FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq JOIN notices n ON n.id = v.caused_by_notice_id WHERE t.id BETWEEN 1 AND 50000 GROUP BY n.profile" | /root/sq.sh'
```

| profile | tenders 1–50,000 (inverted / dispatch-bearing) | tenders 100,000–150,000 (inverted / dispatch-bearing) |
|---|---|---|
| `eforms-de-1.1` | 23 / 26 | 24 / 31 |
| `eforms-de-1.2` | 26 / 28 | 24 / 27 |
| `eforms-de-2.0` | 313 / 345 | 183 / 214 (85.5%) |
| `eforms-de-2.1` | 416 / 449 | 253 / 298 (84.9%) |
| `eforms-sdk-0.1` | 1,266 / 1,435 | 1,466 / 1,677 (87.4%) |
| TED `eforms-sdk-1.x` | 9 of ~49,000 (sdk-1.8 6, sdk-1.10 2, sdk-1.13 1) | 12 of ~46,700 (sdk-1.8 10/2,431, sdk-1.10 2/4,723, rest 0) |

Window 1,460,000–1,510,000 (49,333 tenders, 38,173 without a dispatch): sdk-0.1 **10,766 of
11,160** dispatch-bearing versions inverted. TED-only notice window 27,300,000–27,350,000
(3,753 rows, sdk-1.12/1.13/1.14): **0** — the inversion is a DÖE/DE-dialect phenomenon plus a
handful of TED rows where a no-offset `OPP-012` date sits beside an offset-bearing `BT-05(a)` time.

The 9 TED rows, listed:

```
ssh root@zebreus.click 'echo "SELECT t.id, v.publication_id, n.profile, v.published_at, v.dispatched_at FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq JOIN notices n ON n.id = v.caused_by_notice_id WHERE t.id BETWEEN 1 AND 50000 AND v.dispatched_at > v.published_at AND +n.profile LIKE '"'"'eforms:eforms-sdk-1%'"'"'" | /root/sq.sh'
```
→ tenders 5724, 15315, 17671, 18557, 22786, 32428, 33301, 38608, 47421.

**Named rows, live at rev `9e082fd`:**

| probe | served `published_at` | served `dispatched_at` | what the source says |
|---|---|---|---|
| `curl -sS https://tenders.zebreus.click/v1/tenders/15315` | `2024-03-27T00:00:00Z` | `2024-03-27T12:55:32Z` | `/v1/notices/23838327/content`: `OPP-012-notice` = `"2024-03-27"` (date only), `BT-05(a)-notice` = `"2024-03-27T13:55:32+01:00"`; stored notices row (1711497600, 1711544132), `notice_dates` OPP-012 = (1711497600, offset 0, has_time 0), BT-05(a) = (1711544132, 60, 1) |
| `curl -sS https://tenders.zebreus.click/v1/tenders/1542367` | `2025-01-07T23:00:00Z` | — | notice 26542889 content: `SDK01-RequestedPublicationDate` = `"2025-01-08"`; stored triple (1736290800, 60, 0) — **served a calendar day early** |
| `curl -sS https://tenders.zebreus.click/v1/tenders/5724` | `2024-05-08T00:00:00Z` | `2024-05-08T06:58:57Z` | |
| `curl -sS https://tenders.zebreus.click/v1/tenders/7954578` | `2026-09-04T22:00:00Z` | `2026-09-05T17:32:26Z` | the issue's own probe, still failing |
| `/v1/tenders?source=doe&sort=published_at&order=desc` | whole page at `2026-09-11T22:00:00Z` | | |
| `/v1/tenders?sort=published_at&order=asc&limit=5` | `1993-01-02T00:00:00Z` … | | r209/text era, no offsets published; date-only dispatches days earlier, so **no** inversion there |

**One refinement to the issue's line 18.** The T00:00:00Z vs T22:00/23:00Z split is per published
*value* — whether that lexical date carried an offset — not per profile or era. In window
26,200,000–26,250,000 every publication-date field has `has_time=0`, while
`SDK01-RequestedPublicationDate` carries offset 60 on 22,076 rows and 0 on 1,879, and
`DE1-RequestedPublicationDate` offset 60 on 23,493 and 0 on 2,396. The sdk-0.1 named row
(1542367) itself carries offset 60, so "sdk-0.1 ⇒ T00:00:00Z" does not hold as a rule.

**Contract.** The live `/v1/openapi.json` still states, unconditionally,
`components.schemas.Tender.properties.published_at.description` = *"ISO 8601; a source that
published a date only yields a date only."* (source `crates/app/data/openapi.json:905`), while
`/docs` (`crates/app/src/v1/docs.rs:113`) now discloses the gap and cites this issue — the two
served documents contradict each other. Note the render sites have moved since this issue was
filed: `instant()` is called for published_at at `crates/app/src/v1/json.rs:66, 118, 331`
(the issue body says 306). `stamp()` is right there in the same file and is already used for
`submission_deadline` in the same payload — 15315 serves `"2024-04-26T11:00:00+02:00"` beside
its Z-flattened published_at.

**Judge's reasoning for why this is ours and belongs here:** *"System-introduced and still live:
the parse layer stores the (utc_seconds, offset_minutes, has_time) triple (CONTEXT.md decision
'timestamps as UTC + original offset'), but notice_instants/first_date flatten it to a bare UTC
scalar and /v1 renders published_at through instant() (crates/app/src/v1/json.rs:66,118,331), so a
date-only publication carrying +01:00/+02:00 becomes local midnight re-expressed in UTC (T23:00Z/
T22:00Z of the previous day) and an offset-less date-only value becomes T00:00:00Z. Verified live
at rev 9e082fd: /v1/tenders/15315 → published_at 2024-03-27T00:00:00Z, dispatched_at
2024-03-27T12:55:32Z; /v1/tenders/1542367 → 2025-01-07T23:00:00Z for a portal date of 2025-01-08.
The OpenAPI schema description for Tender.published_at still promises 'a source that published a
date only yields a date only', while /docs (crates/app/src/v1/docs.rs:113) now discloses the gap
and cites 367 — the two documents contradict each other. This is exactly issue 367 unit 3, whose
Status line says it 'remains ready-for-agent: a date-only publication still serves a shifted
instant'; units 1-2 shipped 2026-09-07 but no commit since touches json.rs/project.rs for the
rendering, so it is open, not regressed and not resolved. Issue 18 (published_at = publication not
dispatch) is resolved and does not cover the rendering. The finding's new contribution over 367 is
the quantified dispatch-after-publication rate (86.7% of de-1.1 notices and 96.5% of
dispatch-bearing sdk-0.1 in the id window; ~90% of current eforms-de-2.x versions; 9 TED sdk-1.x
rows where a no-offset OPP-012 date sits beside an offset-bearing BT-05 time) — which should be
attached to 367 rather than filed twice. Medium: a documented field on ~90% of DÖE/DE rows serves
the wrong calendar day and violates the dispatch ≤ publication invariant, but the fix is already
specified (unit 3) and /docs discloses it."*

One caveat the judge recorded against the finding's own framing, which does not defeat it:
CONTEXT.md does not state "dispatch ≤ publication" as a literal invariant (line 215 names only
"publication-date order, dispatch fallback" for the supersession fold), so the *expected* behaviour
here rests on the OpenAPI contract, which is verified live.

**To close:** unit 3 as already written — offset/has_time columns on `notices` and
`tender_versions`, backfilled, with published_at/dispatched_at rendered through `stamp()` at
`json.rs:66/118/331` — after which 15315, 1542367, 5724 and 7954578 stop inverting and the
OpenAPI promise at `openapi.json:905` becomes true instead of contradicted by `/docs`.
