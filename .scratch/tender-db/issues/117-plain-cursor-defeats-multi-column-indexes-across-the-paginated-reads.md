# 117 — a plain `id > ?` cursor defeats every multi-column index across the paginated reads

Status: RESOLVED-VERIFIED (2026-08-25, owner) — live prod timings of the measured
shapes, all sub-second network-inclusive: `/v1/organizations?country=DE&limit=50`
0.90s (was 15.16s server-side), `?kind=vat&limit=50` 0.86s (the 99.08s kind-only
case), `?country=DE&cursor=2000000` 0.75s. The issue-111 detector's silence at the
last two restarts confirms the `(filter, id)` indexes exist on the live DB. Was:
landed on main (merge `3485e3d`, 2026-08-08) — indexes rebuild via the issue-111
startup builder; prod verification pending. Originally: open — LIVE defect, measured
on prod 2026-08-03 at rev `1830d50`. Pre-existing; not caused by
`1830d50`, which fixed one instance of this class and left the rest.
Kind: performance / availability
Blocked by: —
Blocks: —
Priority: high — unauthenticated, trivially reachable, tens of seconds to minutes per request.

## The rule (this is the finding; the individual defects follow from it)

**The cursor column must participate in the index being sought.**

A paginated read written as `WHERE <filter> = ? AND id > ? ORDER BY id LIMIT ?` makes turso drive from
the table in rowid order to satisfy the `ORDER BY`, filtering per row. Whether that is a defect depends
on the shape of the index that *could* have served the filter:

* **single-column index on exactly the filter** → turso uses it anyway. Not a defect.
* **multi-column index whose LEADING column is the filter** → turso refuses it and walks. **Defect.**
* **filter on a non-leading column** → no index can serve it at all. A different problem (see §Kind-only).

Same table, same cursor, different index shape, different outcome — `notices` demonstrates both halves.

Written as a row value, `AND (<filter>, id) > (?, ?)`, the filter column enters the cursor comparison and
turso seeks the index. `<filter>` is already pinned by the equality above, so `(f, id) > (f, after)`
reduces identically to `id > after` — same rows, same order, every page, no schema change. This is
exactly the fix `1830d50` applied to `lots`; it generalises.

`read::changes_since` is the counter-example that proves the rule from the other side: issue 61 gave it
`changes_entity_cursor(entity_kind, cursor)` — the cursor column is *in the index* — so its natural
`entity_kind = ? AND cursor > ? ORDER BY cursor` seeks correctly with no row value needed. Index design
and query shape are two routes to the same requirement.

## Confirmed cases (planned on turso 0.7.0, prod catalogue; timings measured live)

| read | filter | index available | plan | live |
|---|---|---|---|---|
| `read::organizations` | `country` | `organizations_identity(country, identifier_kind, identifier)` | `SEARCH o USING INTEGER PRIMARY KEY (rowid=?)` | **22.0 s** cold, 1.15 s warm |
| `read::organizations` | `identifier_kind` | none usable — 2nd column | rowid | **99.08 s** |
| `read::notices` | `source` | `sqlite_autoindex_notices_1(source, publication_id, content_hash)` | rowid | **≥226 s** (client capped at 300 s) |
| `read::notices` | `profile` | `notices_profile(profile)` — single column | `SEARCH notices USING INDEX notices_profile (profile=?)` | fine |
| `read::tenders` | `source` | `tenders_island(source, island_notice_id)` | rowid | not timed |
| `read::changes_since` | `entity_kind` | `changes_entity_cursor(entity_kind, cursor)` | `SEARCH changes USING INDEX changes_entity_cursor (entity_kind=? AND cursor>?)` | fine |
| all readers | unfiltered | — | rowid | **correct by design** — a global id-ordered page *should* drive from the table |

`read::tenders` was confirmed by reconstructing its full nine-correlated-subquery emitted shape, not by
planning a paraphrase. The rule predicted it before it was measured.

Controls run, so the mechanism is not misattributed:

* stripping the correlated mentions-count subquery from `read::organizations` still gives a rowid walk —
  the subquery is not the cause;
* `SELECT id FROM notices WHERE source = ? AND id > ? ORDER BY id LIMIT ?` (no wide select list, no
  subqueries) also walks — the select list is not the cause;
* the unfiltered form of every reader walks and *should*, so "walks" alone is not the signal.

## Why it hid

The walk stops as soon as it has collected `LIMIT` matching rows. A filter matching **early or densely**
returns fast: `/v1/organizations?country=DE` is 19 ms, `?country=MT` 40 ms. A filter matching **nothing,
or only late**, walks the whole table: `?country=ZZ` 22 s. So the endpoints look healthy under every
ordinary query and collapse on an unusual one — including one a crawler produces by accident.

## Fix

Row-value cursor, per reader, keeping the unfiltered arm on the plain cursor — the same
`Some(x) => row value, None => plain cursor` split `1830d50` used. Verified GREEN on turso 0.7.0:

| read | fixed shape | resulting plan |
|---|---|---|
| `organizations` (country) | `AND (o.country, o.id) > (?, ?)` | `SEARCH o USING INDEX organizations_identity (country=?)` |
| `organizations` (country+kind) | `AND (o.country, o.identifier_kind, o.id) > (?, ?, ?)` | `… (country=? AND identifier_kind=?)` |
| `notices` (source) | `AND (source, id) > (?, ?)` | `SEARCH notices USING INDEX sqlite_autoindex_notices_1 (source=?)` |
| `tenders` (source) | `AND (t.source, t.id) > (?, ?)` | `SEARCH t USING INDEX tenders_island (source=?)` |

### Kind-only is NOT fixable this way — do not certify it with the rest

`/v1/organizations?kind=` (the **99 s** case, the worst measured) filters `identifier_kind`, the *second*
column of `organizations_identity`, and that is the only index on the table. No leading-column seek
exists, so no cursor shape helps. It needs its own answer — an index, or a different read — and must be
tracked separately. Shipping the row-value change and reporting "organizations fixed" would certify a
half-fix as whole, which is precisely what `1830d50` was careful to avoid.

## The row-value cursor is the WEAKER of the two routes — plan evidence, sdk-vendor 2026-08-03

This issue already names both routes ("Index design and query shape are two routes to the same
requirement", with `changes_since` as the counter-example). Planned side by side, they are **not
equivalent**, and the difference matters most exactly where the endpoints are fast today.

Local lab, turso `=0.7.0` (workspace pin, the version the on-box probe pins), `Db::open` catalogue plus
every deferred index, stats-free — the same basis 112's gate rests on. Statements extracted from the
builders, not paraphrased.

| statement | today | + row-value cursor | + `(filter, id)` index, cursor left PLAIN |
|---|---|---|---|
| `notices?source=` | `SEARCH notices USING INTEGER PRIMARY KEY (rowid=?)` | `SEARCH … sqlite_autoindex_notices_1 (source=?)` **+ USE SORTER** | `SEARCH … notices_source_id (source=? AND id>?)` **no sorter** |
| `organizations?country=` | rowid walk | `SEARCH o USING INDEX organizations_identity (country=?)` **+ USE SORTER** | `SEARCH o USING INDEX organizations_country_id (country=? AND id>?)` **no sorter** |
| `tenders?source=` | rowid walk | `SEARCH t USING INDEX tenders_island (source=?)` (sorter already present) | `SEARCH t USING INDEX tenders_source_id (source=? AND id>?)` (sorter still present) |

**Read the third column carefully: the cursor is still the plain `id > ?` there.** With an index whose
second column is `id`, the plain cursor needs no rewrite at all — turso seeks on `(filter=? AND id>?)`,
using the filter AND the cursor as index bounds. That is precisely the `changes_entity_cursor` shape
this issue cites as the counter-example that proves the rule.

### Why the sorter is the point, not a detail

`USE SORTER FOR ORDER BY` means the seek delivers rows in the index's order, not in `id` order, so
**`LIMIT` cannot truncate early**: every row of the filter partition must be visited on every request,
whether the engine then sorts all of them or keeps a bounded top-N. Cost becomes O(partition), not
O(page).

For the **sparse** filters this issue was filed about (`?country=ZZ`, 22 s) that is still an enormous
win — the partition is empty. But the dense ones are the ones that are **fast today**:
`?country=DE` is 19 ms because the current walk stops as soon as it has `LIMIT` matches. Under the
row-value cursor alone, that same request must visit every DE organization, on every page. **The fix as
scoped could regress the common case while fixing the rare one.** That is a prediction from a plan, not
a measurement — it needs a clock (see below) — but it is the kind of prediction worth having before
shipping rather than after.

### An ordering hazard if both land

With `(filter, id)` present, the ROW-VALUE cursor plans as `SEARCH … (source=?)` — it **loses the
`id>?` bound** that the plain cursor keeps. So row-value + index is *worse* than plain + index for deep
pages: page N re-enters the partition from its start. If the row-value change lands now and someone adds
the index later, the combination silently keeps the weaker plan and nothing reports it.

### Recommendation

For the three cases above, prefer **`CREATE INDEX … (filter, id)` with the cursor left alone**. It is
one DDL statement per read, needs no change to `read.rs`, gives an O(page) plan for sparse *and* dense
filters, and matches the shape issue 61 already chose for `changes_since`. Where a suitable multi-column
index already exists and a new one is unwanted, the row-value cursor remains a real improvement over a
full walk — it is a floor, not the ceiling.

Two caveats stated plainly. These are **plan-level** results: EQP proves the access path and whether an
ordering sort is required, and this issue's own Verification section is right that it proves nothing
about elapsed time. The O(partition)-vs-O(page) claim follows structurally from the sorter, but the
dense-case regression must be **timed** — `?country=DE` before and after, not just `?country=ZZ`. And
the deferred-index caveat of issue 111 applies to any new index: a `(filter, id)` index needs a
guaranteed builder, or it is absent exactly when it is needed.

## Verification

The row-value plans above are necessary but not sufficient: **turso's EQP text lies for exactly this
access path** — it renders the 13.2M-row walk as `SEARCH l USING INTEGER PRIMARY KEY (rowid=?)`, which
reads like a point lookup. Plans are sound as a regression detector, not as proof of a speedup. So each
fix must be validated by **timing** the live endpoint with a filter that matches nothing
(`?country=ZZ`, `?source=zz`), not by reading its plan.

Issue 112's gate is the standing detector. B7 (organizations by country) is extracted and reports the
red today; the kind-only case deliberately has no check, because a check with no reachable green is a
permanent alarm rather than a test.

## Availability note (measured, and narrower than it first looked)

These are public and unauthenticated. A request abandoned by its client **keeps running**: after the
client was killed, the app consumed ~65 CPU ticks per 10 s window for 60 s+, against a 0-tick idle
baseline. An nginx-level timeout would therefore free the connection but not the work.

A request-duration bound is **not cleanly available** in turso 0.7.0 — see the feasibility findings in
the run log: `turso::Connection` exposes no `interrupt()` (the capability exists one layer down in
`turso_sdk_kit::rsapi`), and `tokio::time::timeout` cannot abort a step loop that is not yielding.
The proper fix above is the mitigation.

---

## STOP — the row-value fix above is NET-NEGATIVE for `organizations` and `notices` (proj-fix, measured)

**Do not implement the fix table above.** It repairs the pathological filter and makes the ordinary one
**4,209× slower**. Measured locally: 400k organizations all `country='DE'`, scattered identifiers,
against the real `organizations_identity(country, identifier_kind, identifier)`.

| filter | cursor | time | rows |
|---|---|---|---|
| dense (`DE`) | plain | **0.0003s** | 1000 |
| dense (`DE`) | row value | **1.1989s** | 1000 |
| absent (`ZZ`) | plain | 0.0510s | 0 |
| absent (`ZZ`) | row value | 0.0000s | 0 |

The plan for the slow one is the **green** one this issue certified:

```
SEARCH o USING INDEX organizations_identity (country=?)
USE SORTER FOR ORDER BY
```

`organizations_identity` does not contain `id`. Seeking `country = ?` yields the slice in
`(identifier_kind, identifier)` order, so `ORDER BY id` sorts the **whole slice** and `LIMIT` applies
after it. The plain cursor has the opposite profile — it walks in rowid order and stops at `LIMIT`
matches, which is instant when the filter is dense and a full walk when it matches nothing.

So the two shapes trade places, and **§Verification above is structurally unable to see it**: it
specifies timing a nothing-matching filter, which measures only the half that improves. That is the
same instrument error as the rest of today — an instrument aimed narrower than the claim it carries.

## The fix is the INDEX; then the query needs no change at all

Measured, not assumed. Adding `organizations(country, id)`:

| filter | cursor | time |
|---|---|---|
| dense (`DE`) | plain | **0.0007s** |
| dense (`DE`) | row value | 0.0009s |
| absent (`ZZ`) | plain | **0.0000s** |
| absent (`ZZ`) | row value | 0.0000s |

```
SEARCH o USING INDEX organizations_country_id (country=? AND id>?)
```

Seek the country, range-scan the id, **no sorter**, `LIMIT` truncates immediately. Both filters fast,
and `read::organizations` needs **no code change**.

This is this issue's own rule — *the cursor column must participate in the index being sought* —
reached by the other route. `changes_entity_cursor(entity_kind, cursor)` is named above as the
counter-example that proves the rule; it is also the **template**. Index design and query shape are
both routes, and which one applies depends on the trailing column of the index, not on preference.

**Why `1830d50` was nonetheless correct**, and why it must not be transplanted: `UNIQUE(tender_id,
lot_key)` also lacks a trailing `id`, so the tender-scoped lots read *does* still sort — but a
Tender's slice is ~2 rows (corpus max 2,604), so the sort is free. Issue 115's page sweep confirms it:
flat from `LIMIT 125` to `LIMIT 1000`, at ~15ms. **The row value is safe when the slice is small and
harmful when it is large.** That distinction is the whole finding, and nothing in the original fix
table records it.

## Revised plan (needs re-costing before anyone writes code)

- `organizations(country, id)` — new index over 25.3M rows.
- the `notices` equivalent — `sqlite_autoindex_notices_1(source, publication_id, content_hash)` has no
  trailing `id` either, over 27.35M rows.
- `tenders`: `tenders_island(source, island_notice_id)` has the same shape, so the same trade is
  expected — **not measured**, and must be before it is assumed.
- Both new indexes carry issue 111's deferred-builder obligation. This is materially larger than the
  original scope: a schema change at prod scale, not a query edit.

**Limits of these numbers:** synthetic 400k table at 100% density — an extreme. Real share is lower so
the sort is smaller, but at 25.3M rows even a 10% share means sorting ~2.5M rows per request. The
direction is certain; the magnitude at prod scale is run-driver's to establish. The four numbers that
settle it: `?country=DE` and `?country=ZZ`, before and after adding the index on a scratch copy.

Probe kept at `crates/store/tests/org_cursor_probe.rs`.

## Prod slice sizes and dense timings (run-driver, on the snapshot / live `2751ce3`)

The inputs that turn the trade-off above from a shape into a magnitude:

| slice | rows |
|---|---|
| `organizations WHERE country='DE'` | **3,854,017** |
| `organizations WHERE country='FR'` | **4,253,550** |
| `notices WHERE source='ted'` | **~13.1M** |

Current dense timings on the plain cursor (the shape live today):

| request | now |
|---|---|
| `/v1/organizations?country=DE&limit=50` | **11 ms** |
| `/v1/organizations?country=DE&limit=1000` | 3.24 s |
| `/v1/notices?source=ted&limit=1000` | 72 ms |
| `/v1/notices?kind=text&limit=1000` | 84 ms |
| `/v1/tenders?source=ted&limit=1000` | 2.24 s |

**`?country=DE&limit=50` at 11 ms is the case that decides this.** Under the row-value fix it becomes a
sort of 3.85M rows *regardless of page size*, because the sorter runs before `LIMIT` — the same
ordering seen in issue 115's flat `LIMIT 125→1000` sweep. A 50-row page of German organizations is
about as ordinary a request as this API serves.

So the trade as originally designed is: `?country=ZZ` 22 s → fast, against `?country=DE&limit=50`
11 ms → seconds. **A net loss on real traffic.** `ZZ` is a crawler artifact; `DE` is the use case.

The plan pair, free EQP on the prod catalogue, confirming the mechanism on both tables:

```
organizations, CURRENT (plain cursor)
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)              <- walk, LIMIT truncates it
organizations, ROW-VALUE FIX
  SEARCH o USING INDEX organizations_identity (country=?)
  USE SORTER FOR ORDER BY                                   <- the whole 3.85M slice
```

`notices` gives the identical pair. **`tenders_island(source, island_notice_id)` has the same defect**,
so `/v1/tenders?source=` — 2.24 s today — would gain a ~13M-row sort.

### Class B has a confirmed worst member

`/v1/tenders?country=ZZ` **exceeded 380 s** (did not complete at the client cap). Not cursor-fixable
under any shape: `country` is an `EXISTS` over `tender_version_classifications` evaluated per row
across 4.26M Tenders. It is now the worst measured member of the class and belongs to the
not-cursor-fixable half.

### Attribution of what is and is not established

run-driver's contribution here is **the mechanism confirmed at plan level and the input sized** — the
plans, the slice counts, the current timings. The post-fix dense case has **not** been clocked
end-to-end on prod; "sort 3.85M rows costs X" is inferred from the plan, not measured, and run-driver
flagged it as such. Closing that gap is what `crates/store/tests/org_cursor_probe.rs` is for, run at
`TDB_ORG_ROWS=3854017` to match the DE slice exactly.

## Prod-scale measurement: the row-value fix is 151,648× slower (proj-fix)

Re-run of the probe at run-driver's exact measured DE slice — 3,854,017 rows.

```
--- today's indexes (organizations_identity only) ---
filter        limit        plain    row value
dense (DE)       50      0.0001s     15.1648s     <-- 151,648x SLOWER
dense (DE)     1000      0.0003s     15.6361s     <--  52,120x
absent (ZZ)      50      0.4421s      0.0000s
absent (ZZ)    1000      0.4375s      0.0000s
```

`?country=DE&limit=50` — 11 ms live today — becomes **15 seconds**. Page size is irrelevant (15.16 s
at 50 against 15.64 s at 1000) because the sorter runs before `LIMIT`.

```
organizations(country, id) built in 9.2s   (3.85M rows)

--- with organizations(country, id) ---
filter        limit        plain    row value
dense (DE)       50      0.0001s      0.0001s
dense (DE)     1000      0.0008s      0.0010s
absent (ZZ)      50      0.0000s      0.0000s
absent (ZZ)    1000      0.0000s      0.0000s

plain plan: SEARCH o USING INDEX organizations_country_id (country=? AND id>?)
```

Every case fast, **and `read::organizations` needs no code change**. For issue 111: 9.2 s per 3.85M
rows suggests roughly a minute at 25.3M — the builder obligation is the cost, not the build itself.

Limits: 100% DE synthetic with scattered identifiers. run-driver's real-distribution bed is the
confirmation; if the two disagree, the real-distribution one wins.

## Class B: an existence short-circuit, and the precedent for it

For the not-index-fixable half (`?country=`, `?cpv=`, `?buyer=`, `?winner=` on `/v1/tenders` — all
`EXISTS` subqueries), the *matches-nothing* case has a cheap correct answer that is **already an
established pattern in this file**:

`read::changes_since` short-circuits an unknown `entity_kind` to an empty result (issue 61 finding 2) —
*"a kind with no rows must never trigger a table walk to discover it has none."* That is precisely
`/v1/tenders?country=ZZ` at >380 s.

Probe the existing `tender_version_classifications_code(scheme, code)` index before building the query:

```sql
SELECT 1 FROM tender_version_classifications
 WHERE scheme = 'nuts' AND code >= ? AND code < ?   -- 'ZZ', 'Z['
 LIMIT 1
```

One index seek. If nothing comes back, no Tender can satisfy the `EXISTS`, so the empty page is the
**correct** answer rather than an approximation. Prefer the explicit prefix range over `LIKE` — turso's
`LIKE` prefix optimisation is not something to assume.

Preferable to the alternatives considered: it is not a rejection of a documented filter, not a
bounded/partial response (which would reintroduce the self-contradicting body issue 116 exists to
kill), and not really a stopgap — it stays correct after the real fix, so nothing has to be unwound.
Generalises to `cpv` (same index) and to `buyer`/`winner` via `tender_version_parties_org` and
`tender_version_result_winners_org`.

**Limitation, which decides how much it is worth:** it fixes *matches-nothing*, not *matches-late*. A
prefix that exists but only on high `tender_id`s still walks. So it removes the worst measured case and
the naive crawler, but **it is not a complete DoS defence** and must not be described as one.

**Not yet measured.** Whether that seek is actually index-served needs the stopwatch before anything
relies on it — the same discipline that killed Class A's row-value fix.

## THE DENSE BASELINE — measured live, unrecoverable once a fix deploys

The Class A row-value fix was killed by a **dense** measurement (proj-fix: 4,209× slower on
`?country=DE`), and the verification this issue originally specified — time a nothing-matching filter —
is *structurally blind* to that regression: an empty partition sorts instantly, so the gate goes green
precisely because it measures the only case the fix helps.

So any 117 fix must be validated against **both** halves, before and after. The sparse half is recorded
above. This is the dense half, captured on prod at rev `a39d53a` (the reads below are untouched by 115,
so these are the pre-117 numbers). **Once an index or a query change lands, the "before" is gone** — it
cannot be recovered from a snapshot, because it is a property of the live engine, the live cache state
and the real data distribution together.

Two samples per request, cold-ish then repeated, because the spread between them is large and a single
figure invites the same cold-vs-warm confusion that made an earlier deploy report pessimistic:

| request | 1st | 2nd |
|---|---|---|
| `/v1/organizations?country=DE&limit=5` | 0.0118 s | 0.0009 s |
| `/v1/organizations?country=DE&limit=50` | 0.158 s | 0.0097 s |
| `/v1/organizations?country=DE&limit=200` | 0.250 s | 0.0273 s |
| `/v1/organizations?country=FR&limit=50` | 1.088 s | 0.0241 s |
| `/v1/organizations?country=FR&limit=1000` | 0.0349 s | 0.0286 s |
| `/v1/organizations?country=MT&limit=50` | 0.0985 s | 0.0408 s |
| `/v1/notices?source=ted&limit=50` | 0.0034 s | 0.0010 s |
| `/v1/notices?source=ted&limit=1000` | 0.0521 s | 0.0093 s |
| `/v1/notices?kind=text&limit=50` | 0.0099 s | 0.0019 s |
| `/v1/tenders?source=ted&limit=50` | 0.103 s | 0.0310 s |
| `/v1/tenders?source=ted&limit=1000` | 1.112 s | 0.299 s |

Earlier samples of the same endpoints on rev `2751ce3`, taken with a colder page cache, read
`?country=DE&limit=50` at 11 ms and `?source=ted&limit=1000` at 2.24 s. **Both sets are honest; the
spread is cache state, not disagreement.** Compare like with like — a post-fix number taken warm against
a pre-fix number taken cold would manufacture a win, which is the mirror of the mistake that made an
earlier deploy report look worse than reality.

Partition sizes the sorter would have to sort, measured on prod:

| partition | rows |
|---|---|
| `organizations WHERE country = 'DE'` | 3,854,017 |
| `organizations WHERE country = 'FR'` | 4,253,550 |
| `notices WHERE source = 'ted'` | ~13.1 M |

### The acceptance gate for any 117 fix

1. every sparse case at least as fast as recorded above (`?country=ZZ`, `?source=zz`, `?kind=zzz`,
   `?country=ZZ` on tenders);
2. **every dense case above no slower than its recorded figure**, compared like-for-like on cache state;
3. both measured with a clock on the deployed engine — not from a plan.

### Why (3) is not negotiable, and why no plan gate can replace it

The row-value fix was certified GREEN from a plan that read
`SEARCH o USING INDEX organizations_identity (country=?)` — a walk becoming an index seek, the exact
transition a plan gate exists to reward. Issue 112's B7 would have flipped **RED → GREEN** on the change
that made the common query 4,209× slower: the gate would not merely have missed the regression, it
would have **endorsed** it.

Nor is "assert no sorter" the repair. Issue 115's *fixed* `lots` read ends in `USE SORTER FOR ORDER BY`
and is correct, because its partition is ~2 rows. The discriminator is the **size of the partition being
sorted**, and that quantity appears nowhere in EQP output. **This defect class is therefore not
detectable by any plan assertion**, and 112's B-checks should say so about themselves rather than imply
a coverage they cannot have.


## VERIFICATION BOUNDARY — no plan gate can validate this fix (sdk-vendor)

Issue 112's plan gate **must not be used to sign off 117**, and the reason is not that
the gate is immature — the information it would need is not in a query plan.

I added a check (112's B7) asserting that `read::organizations` must be index-served
rather than a rowid walk. proj-fix then measured the row-value fix: on a dense filter
it is **4,209x slower** (151,648x at prod scale) than the walk B7 called broken.

```
plain cursor (today)    0.0003s   SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
row value  (proposed)   1.1989s   SEARCH o USING INDEX organizations_identity
                                  USE SORTER FOR ORDER BY
```

**B7 would have flipped RED to GREEN over that regression**, because it presents as an
improved plan. The check was withdrawn.

The mechanism generalises to every read this issue touches. `organizations_identity` is
`(country, identifier_kind, identifier)`, so a seek on `country` returns rows ordered by
`identifier_kind`, not by `o.id`. The query asks `ORDER BY o.id`, so every matching row
must be materialised and sorted before `LIMIT` applies. Therefore:

| filter | plain cursor | index seek |
|---|---|---|
| sparse (`?country=ZZ`) | walks 25.3M for nothing — **22.0s** | instant |
| dense (`?country=DE`) | `LIMIT` stops early — fast | seek + sort **all** matches — slow |

**Which access path is correct depends on selectivity, and selectivity is not in the
plan.** So for this class the plan is not merely insufficient evidence, it is
*misleading* evidence: it points the wrong way exactly when the fix is wrong.

### What this fix must be validated by instead

A **clock**, over a **selectivity spread** — at minimum one sparse and one dense value
of each filter, on prod-scale data — with the pre-fix numbers taken first. A single
"before/after on one value" cannot distinguish a fix from a swap of which inputs are
slow, which is precisely what the row-value cursor does.

### And a design note that may remove the tradeoff entirely

The dilemma is an artefact of the available index, not of the query. An index leading
with the filter column and ending in the cursor column —

```sql
CREATE INDEX organizations_country_id ON organizations(country, id);
```

— gives the seek **and** `o.id` ordering, so there is **no sorter** and `LIMIT`
truncates early. Fast at both densities, with no selectivity tradeoff to measure. It is
the shape `tenders_current_published(current_published_at, id)` already uses in this
codebase for exactly this problem.

This also overturns a claim I made earlier in 112 and should be corrected wherever it
was repeated: I wrote that **no index can serve the kind-only filter**, reasoning from
the index that exists. `organizations(identifier_kind, id)` would serve it — seek and
ordered — which makes the 99.08s case fixable rather than a design dead end.

If the fix takes that form, a plan check becomes meaningful again for these reads,
because "seek on the filter column with no top-level sorter" is then both *achievable*
and *equivalent* to the property we care about. Until then, timing only.

### The Class B short-circuit, measured (proj-fix)

200k Tenders, all `nuts = 'DE300'` except the last 20 at `MT001` — so `MT` exists but only at high
`tender_id`s, which is the *matches-late* case the guard cannot fix.

```
case                          time    rows
filter dense (DE)          0.0026s    1000
filter late (MT)           0.5000s      20
filter absent (ZZ)         0.4954s       0

guard  dense (DE)          0.0000s       1   (range form)
guard  late (MT)           0.0000s       1
guard  absent (ZZ)         0.0000s       0
```

`absent (ZZ)` goes **0.4954s → 0.0000s** at 200k Tenders; prod's 4.26M is where that walk is the
>380 s hang.

**The prefix-range form is REQUIRED — `LIKE` does not seek.**

```
guard, LIKE:
  SEARCH … USING INDEX tender_version_classifications_code (scheme=?)
guard, range:
  SEARCH … USING INDEX tender_version_classifications_code (scheme=? AND code>=? AND code<?)
```

turso uses `LIKE` only to seek `scheme`, then scans every `nuts` row — 0.0273 s against 0.0000 s. So
write `code >= ? AND code < ?` with the prefix and its successor; the obvious `LIKE 'ZZ%'` version
would scan a whole scheme's slice on every request.

**Measured limitation.** `filter late (MT)` is **0.5000 s and the guard does not help**: it reports
"found", the query proceeds, and the walk happens — the same order as the absent case. So the guard
removes the *matches-nothing* walk and nothing else. `MT` is a real country, not a contrived value, so
this is reachable without adversarial intent.

**It is therefore a correctness-preserving performance fix, NOT a security control, and must not be
recorded as mitigating the DoS.** Bounding worst-case work on `/v1/tenders?country=` needs something
else.

Worth doing on those terms: one seek, correct rather than approximate, no schema change, matching the
established `changes_since` unknown-`entity_kind` pattern (issue 61 finding 2), surviving the real fix
so nothing is unwound, and generalising to `?cpv=` (same index) and `?buyer=`/`?winner=` via
`tender_version_parties_org` / `tender_version_result_winners_org`.

Probe: `crates/store/tests/exists_shortcircuit_probe.rs`, which keeps the MT case so the limitation
stays visible in the tree.

## THE SELECTIVITY SPREAD — the pre-fix half, warm-vs-warm, on the deployed engine

The dense table above was captured cold-then-repeated. This is the same set taken **warm** —
first request discarded as cache-warming, then two consecutive samples — so a post-fix number can be
compared like with like. Rev `a39d53a`, loopback, deployed turso 0.7.0. **Compare warm to warm.**

Three selectivity classes per filter, because two are not enough to see the shape:

* **dense** — the filter matches early and often. Fast today; the case a fix can *regress*.
* **matches-late** — the filter matches, but not near the start of the rowid order. Middling today.
* **matches-nothing** — no rows at all. Pathological today; the case the fix is *for*.
* **matches-exhausted** — *added 2026-08-05, after prod turned out to be this and not matches-late.*
  The filter matches EARLY and then runs out, so paging past the cluster leaves fewer than `LIMIT`
  rows ahead while most of the table still is. Indistinguishable from matches-nothing after the
  cluster is consumed, and invisible if you only time page 1. The three classes above were chosen by
  where matches *start*; this one is about where they *stop*, which is why the original set could not
  express it. See the `?kind=registration` correction below.

**The taxonomy itself was the unstated assumption.** All three original classes are properties of the
FIRST match's position, so a filter was implicitly assumed to keep matching once it started. Nothing
measured that, and prod's `registration` does not: 120 rows, all inside deciles 1–2. A classification
scheme can be the thing that is wrong, not just the classification — and it fails silently, because
every case still lands in *some* bucket.

| read | filter | class | limit=50 | limit=1000 |
|---|---|---|---|---|
| `organizations` | `country=DE` (3.85 M) | dense | 0.0119 / 0.0096 s | 0.0574 / 0.0568 s |
| `organizations` | `country=FR` (4.25 M) | dense | 0.0211 / 0.0229 s | — |
| `organizations` | `country=MT` (24,911) | matches-late | 0.0409 / 0.0467 s | 0.2755 / 0.2776 s |
| `organizations` | `country=ZZ` (0) | matches-nothing | 22.0 s cold / 1.15 s warm | — |
| `organizations` | `kind=zzz` | matches-nothing, **no index possible** | 99.08 s | — |
| `notices` | `source=ted` (~13.1 M) | dense | 0.0023 / 0.0014 s | 0.0112 / 0.0067 s |
| `notices` | `source=doe` (~1.1 M) | matches-late | 0.0998 / 0.1126 s | 0.1136 / 0.1146 s |
| `notices` | `source=zz` (0) | matches-nothing | ≥226 s (client cap) | — |
| `tenders` | `source=ted` | dense | 0.0369 / 0.0334 s | 0.2976 / 0.2947 s |
| `tenders` | `source=doe` | matches-late | 0.0084 / 0.0065 s | 0.1222 / 0.1112 s |
| `tenders` | `country=ZZ` | matches-nothing, **EXISTS, no index possible** | **>380 s (did not complete)** | — |

Two things worth reading off this table that the sparse/dense pair alone would hide:

1. **`notices?source=doe` is 40–70× SLOWER than `?source=ted`** despite matching ~12× fewer rows. The
   walk starts at rowid 0, where `ted` rows dominate, so collecting 50 `doe` rows means walking past a
   great many `ted` ones. Selectivity is not the variable — **position in rowid order** is. A fix
   validated only on `ted` would look like it had nothing to improve.
2. **`organizations?country=MT` at `limit=1000` is 0.276 s** — the matches-late floor. It is the control
   that keeps the short-circuit honest: a short-circuit fixes matches-nothing and leaves this untouched.

### The short-circuit is a PERF fix, not a DoS mitigation

Measured (plan on turso 0.7.0, timings via sqlite3 on the real data — engine caveat stated):

| probe | result | time |
|---|---|---|
| `scheme='nuts' AND code >= 'ZZ' AND code < 'Z['` | absent | **0.01 s** |
| same, `XX` | absent | 0.00 s |
| same, `DE` / `MT` | present | 0.00 s |
| `scheme='nuts' AND code LIKE 'ZZ%'` | absent | **41.05 s** |

`SEARCH tender_version_classifications USING INDEX tender_version_classifications_code
(scheme=? AND code>=? AND code<?)` — both bounds used. So `/v1/tenders?country=ZZ` goes **>380 s → ~0**.

But `LIKE` and the range form **both plan as `SEARCH … USING INDEX`**, and differ by ~4,000× because
`LIKE` keeps only the `scheme=?` bound. Third instance in one day of the cost living in partition size,
which EQP does not express. **Write the explicit range; never the `LIKE`.**

And the limitation, which must stay visible: the short-circuit fixes **matches-nothing only**. A prefix
that exists but only on high ids still walks — `?country=MT`-class behaviour, 0.276 s here and far worse
at prod scale on a filter that goes through `version_predicates`. **It removes the worst measured case
and the naive crawler. It is not a DoS defence and must not be described as one.**


## The `(filter, id)` route is measured and committed — `8cdc35d` (proj-fix)

Four indexes, and **no `read.rs` change at all**:

```
organizations(country, id)          organizations(identifier_kind, id)
notices(source, id)                 tenders(source, id)
```

The plain cursor stays, which is the whole point: with a `(filter, id)` index it uses
**both** the filter and the cursor as index bounds, so no sorter runs and `LIMIT`
truncates early. The row-value route is not needed and is the weaker of the two.

### The kind-only overturn, confirmed by measurement rather than argument

This issue originally recorded that no index could serve the kind-only filter. I argued
that was wrong — reasoning from the index that *exists* rather than the one the read
*needs* — and proj-fix measured it on the real `read::organizations`:

| | dense (`vat`) | absent (`zzz`) |
|---|---|---|
| before | 0.0002s | 0.0423s |
| after | 0.0001s | **0.0000s** |

Worth keeping the *shape* of that error, not just its correction: an index's existence
was treated as the boundary of what was achievable, so a read with no good index was
recorded as a read with no good plan. Those are different claims, and only the second
one was ever checked.

proj-fix measured all four rather than generalising from `organizations` — including
`notices`, which is shape-identical to `tenders`. "Same shape, therefore same result"
is precisely the reasoning that produced this issue's original fix table, and that was
wrong by 151,648×.

### Verification, and an operational caveat that will look like a regression

112's B7/B8/B9 assert `cursor-bound` — that the seek carries `id>?`. They are keyed on
the **property**, not on `8cdc35d`, so they need no guard and no sha: red now, green by
themselves once an index giving both the seek and the id bound is serving.

**But three of the four indexes are DEFERRED** — only `notices(source, id)` is in the
schema batch. `organizations(country,id)`, `organizations(identifier_kind,id)` and
`tenders(source,id)` are built at a rebuild's end or by a `Reindex` job (issue 111). So
between deploying `8cdc35d` and the first reindex, **B7/B8/B9 stay red for an
operational reason**, and the deploy will look like it did not work.

Section A distinguishes them: an index `DECLARED at <rev> but ABSENT from the DB` means
a reindex is owed; the index present while the plan ignores it would be the regression.
The B-rows say this in their own output, so nobody chases a phantom.

**A deploy of `8cdc35d` is therefore not complete until a reindex has run.** That is an
ordering requirement, not a caveat.

### Class B short-circuit: implemented (`8d7afd3`), and the bug it nearly shipped

Generalised to `?country=`, `?cpv=`, `?buyer=`, `?winner=`. `Scope::Page` only — `Scope::At` is a
point lookup for the SSE diff loop and never walks, so guarding it would cost a seek per event.

**The first version was silently WRONG, and the way it was wrong is worth keeping.** It probed one
range over the prefix as given. SQLite's `LIKE` folds ASCII case; `>=`/`<` do not. Verified:

```
stored 'DE300' :: LIKE 'DE%'     -> MATCH
stored 'DE300' :: LIKE 'de%'     -> MATCH
stored 'DE300' :: range DE..DF   -> MATCH
stored 'DE300' :: range de..df   -> no match
```

So `/v1/tenders?country=de` would have returned an **empty page while the real query returns rows** —
silently, on a documented filter, and *faster*, which is the shape that survives review. The obvious
test (present matches, absent empty, both uppercase) would have passed.

Fixed by probing every ASCII-case variant of the prefix; their union is exactly what `LIKE` matches.
Past a bounded variant count the guard declines to apply and the full query answers — slow, correct.

**The general rule this yields:** a guard that stands in for a predicate must be **no narrower than
the predicate**. Too generous costs a walk that could have been avoided; too narrow returns wrong rows
rather than slow ones. The hazard is entirely one-sided, so the design must be too — and every
`None`/decline path in `prefix_ranges` deliberately falls to the slow-but-correct side.

Mutation-verified: restoring the single-range form fails the test on the lowercase case, naming the
rows it wrongly dropped.

One correction to the test's own claims: the superseded-version case does NOT falsify a seq-correlated
guard (mutation shows such a guard passes), because that variant would be equivalent in scope and
merely costlier. The comment now says so. A test comment claiming coverage it lacks is how a suite
stops meaning what it says — the same failure as a gate implying coverage it cannot have.

## `organizations_kind_id` — measured at real scale, and Class B shrinks to one entry

The `?kind=` read was recorded above as **not fixable by any cursor shape**, because `identifier_kind`
is the *second* column of `organizations_identity` and no leading-column seek exists. That was correct
about the cursor and wrong about the conclusion: the answer is the same `(filter, id)` index the rest of
Class A gets. `organizations(identifier_kind, id)` gives `identifier_kind` a leading column of its own.

Measured on the 8,132,478-real-row copy, **query unchanged** (plain `id > ?` cursor throughout):

| class | filter | rows | limit | before | after | factor |
|---|---|---|---|---|---|---|
| dense-early | `kind=vat` (ids from 31) | 49,421 | 50 | 0.2 ms | 0.3 ms | *noise, see below* |
| | | | 1000 | 544.8 ms | **5.0 ms** | **109×** |
| matches-late | `kind=national` (ids from 1.17 M) | 60,981 | 50 | 103.1 ms | **0.3 ms** | **344×** |
| | | | 1000 | 241.5 ms | **4.1 ms** | 59× |
| matches-nothing | `kind=zzz` | 0 | 50 | 1,666.1 ms | **0.1 ms** | **~16,600×** |
| | | | 1000 | 1,667.5 ms | **0.1 ms** | ~16,700× |

`SEARCH o USING INDEX organizations_kind_id (identifier_kind=? AND id>?)` — no sorter, `LIMIT`
truncates. Prod's live `?kind=zzz` was **99.08 s**; this is the read that fixes it.

**The one cell that is not an improvement is `vat@50`, 0.2 → 0.3 ms.** Recorded rather than rounded
away, because "dense no-slower" is the acceptance criterion and this is the only cell that does not
strictly meet it. It is a single tick at the probe's resolution, and the mechanism cannot make a
dense-early case meaningfully slower — the seek positions and reads 50 rows, exactly as the walk did
when the matches were early. The same-class `vat@1000` cell at 109× faster is the decisive dense
evidence.

**No cross-index regression.** With both `organizations_country_id` and `organizations_kind_id`
present, the country classes are unchanged: DE 0.3 ms, MT 0.3 ms, ZZ 0.1 ms at `limit=50`.

**Build cost — a second point for the memory budget:** 34.9 s, peak RSS 330,320 KB ≈ 323 MiB at
8.13 M rows, i.e. **~41 B/row** against `organizations_country_id`'s ~45 B/row on the same table. The
narrower key does cost less, so the 45 B/row constant used for the size guard is **conservative,
measured rather than assumed**.

### Class B now has exactly one member

With `?kind=` moved into Class A, the only read left that no `(filter, id)` index can serve is
**`/v1/tenders?country=`** (>380 s, did not complete). Its filter is not a column of the driven table at
all — it is an `EXISTS` over `tender_version_classifications` evaluated per row across 4.26 M Tenders —
so it needs the restructure, not an index. The existence short-circuit fixes its *matches-nothing* case
only, and remains a performance fix rather than a DoS defence.


## GROUND TRUTH: how turso 0.7.0's `LIKE` actually folds (sdk-vendor, measured)

The `tenders?country=` short-circuit rests on **"the union of ASCII-case variants of the
pattern ⊇ everything `LIKE` matches"**. That holds only if turso folds *at most* ASCII.
Nobody had established it on the deployed engine — it was assumed — so it is measured
here, on turso **`0.7.0`**, the version pinned both in the workspace and at the deployed
rev `a39d53a`. Probe kept as `canonical-verify/like-folding-probe.patch`; **re-run it on
any turso bump**, because this is a property of the engine, not of our code.

### 1. Case folding is ASCII-ONLY — the assumption HOLDS

| value | pattern | result |
|---|---|---|
| `abc` | `ABC` | **MATCH** |
| `AbC` | `aBc` | **MATCH** |
| `ä` | `Ä` | no |
| `é` | `É` | no |
| `ø` | `Ø` | no |
| `ß` | `SS` | no |
| `σ` | `Σ` | no |
| `а` (Cyrillic) | `А` | no |
| `İ` | `i` | no |
| `ı` | `I` | no |

Eight non-ASCII pairs, none folded. So enumerating ASCII-case variants does cover
everything `LIKE` matches, and the short-circuit is sound **on this axis**.

### 2. But metacharacters are NOT covered by that argument, and this is the live risk

| value | pattern | result | |
|---|---|---|---|
| `axb` | `a%b` | **MATCH** | `%` is a wildcard |
| `ab` | `a%b` | **MATCH** | `%` matches empty |
| `abc` | `a_c` | **MATCH** | `_` is exactly one char |
| `abc` | `a_` | no | …exactly one, not one-or-more |
| `a\b` | `a\b` | **MATCH** | **backslash is NOT an escape by default** |
| `a%b` | `a\%b` | no | so `\%` without ESCAPE matches nothing useful |
| `a%b` | `a\%b` **ESCAPE `\`** | **MATCH** | escaping works only with the clause |
| `axb` | `a\%b` **ESCAPE `\`** | no | and then it is exact |

**The consequence for the short-circuit is a correctness one, not a performance one.**
If a user-supplied value is interpolated into a `LIKE` pattern *without* an `ESCAPE`
clause, any `%` or `_` in that value is a **wildcard**. Then:

* `?country=d%` — `LIKE 'd%%'` matches `DE`, `DK`, `d-anything`; a short-circuit that
  treats the value as a literal and enumerates `{d%, D%}` as exact strings matches
  **nothing**. Different rows, silently.
* `?country=_E` — same shape, `_` matching any single character.

So the short-circuit is equivalent to `LIKE` only for values containing neither `%` nor
`_`. Whether that is guaranteed is a question about the *input path*, not about `LIKE`,
and it is the part I cannot answer from the engine — it belongs with proj-fix's point 1.

### 3. Adversarial case list, offered as a SPEC for proj-fix's test

Not an edit to `tenders_shortcircuit.rs` — handing over the cases, since the value of an
independent list is that its author did not write the code:

1. `de` — the ordinary path; short-circuit and `LIKE` must return identical rows.
2. `DE`, `De`, `dE` — every ASCII case variant, same rows as (1).
3. `d%` — **`%` in the input.** Must not silently diverge.
4. `_E`, `%`, `%%` — `_` and bare-wildcard inputs, including the pattern that matches
   everything.
5. `d\` and `\%` — backslash, which is NOT special without `ESCAPE`; verify whichever
   behaviour is chosen is the one implemented.
6. `` (empty) — `'' LIKE ''` is **MATCH** but `'x' LIKE ''` is **no**; an empty filter
   must mean the same thing on both paths.
7. `Ä`, `ß`, `İ` — non-ASCII, which do NOT fold. The short-circuit must not fold them
   either, or it returns rows `LIKE` would not.
8. A value longer than any stored country code, and one containing a NUL or a newline.

Cases 3–6 are where I would expect a divergence if there is one; 7 is the one this
measurement proves is safe *provided the short-circuit does not do its own folding*.

## The short-circuit's real scope: matches-nothing **on a ≤4-letter prefix**

`reachable()` (`3c5ae52`) is narrower than "matches-nothing is now fast", and the difference is
load-bearing for what Class B still owes.

**What it guards.** Only `country`, `cpv`, `buyer` and `winner`. Each arm is `let Some(x) else
{ continue }`, so a request filtering on anything else — `?source=`, `?kind=`, `?status=`,
`?min_value=` — issues **no guard SQL at all**. Confirmed by reading the four loops, not inferred:
`/v1/tenders?source=ted` costs one async call and four `Option` checks, which is why the
`tenders(source, id)` measurement above is unaffected by the guard in either direction.

**What it costs where it does apply** (measured on the real data, deployed engine for the plan):

| case | seeks | time |
|---|---|---|
| present prefix, first variant hits (`DE`) | 1 | 0.00 s |
| absent prefix, single variant | 1 | 0.02 s |
| **`MAX_CASE_VARIANTS` ceiling — 16 consecutive miss-seeks** | 16 | **0.00 s** |

The ceiling is the case worth knowing: `prefix_ranges` probes every ASCII case variant and breaks on the
first hit, so a code stored in the case tried *last* pays for all of them. At the cap that is 16 index
seeks in **under 10 ms**, against the >380 s walk it prevents. The comment's own justification — "16
seeks at ~0.01 s is still four orders of magnitude under the walk" — is now measured rather than
estimated, and it is conservative: sixteen seeks came in below the 0.01 s it assumes for one.

**The boundary, which must not be rounded off.** `prefix_ranges` returns `None` — declining the guard
entirely — when the prefix carries more than four ASCII letters (`1 << letters > 16`), or contains a
`LIKE` metacharacter, or is non-ASCII. So:

* `?country=ZZ` (2 letters) — **guarded**, >380 s → ~0;
* `?country=DE300` — **guarded**. The cap counts **ASCII letters, not characters**
  (`prefix.chars().filter(char::is_ascii_alphabetic).count()`), so a full NUTS-3 code is 2 letters +
  3 digits = 4 variants, well inside the 16 cap. Same for `?country=ZZ999`.
* `?country=ABCDE` (5 letters) — **not guarded**, and neither is any other 5+-**letter** value.

**CORRECTION (2026-08-03).** An earlier revision of this section stated that `DE300` and `ZZ999` decline
the guard and still walk. **That was wrong** — it read the cap as counting characters when it counts
alphabetic characters only. Every real NUTS and CPV code is letters-then-digits and therefore *is*
guarded. The residual matches-nothing gap is only a **5+-ASCII-letter** prefix such as `ABCDE`, which is
not a shape any real code takes — i.e. adversarial-only rather than reachable by an ordinary user or a
naive crawler. The error is left visible rather than silently rewritten, because the corrected boundary
is *narrower* than what was recorded and someone may have planned against the wider claim.

The guard therefore covers every real code shape and declines only prefixes that no real code takes.
That is still the **right** failure direction — declining is correct-but-slow, never wrong — and it is
still a performance fix rather than a defence, for the reason below, which is unchanged by the
correction:

> **The `/v1/tenders?country=` DoS is NOT closed by the short-circuit.** The load-bearing reason is
> **matches-late, not matches-nothing**: a filter value that exists but only on high `tender_id`s makes
> the guard report "found", the query proceeds, and the walk happens exactly as before. Rare-but-real
> NUTS codes are ordinary user input, so this is reachable without adversarial intent. (A 5+-letter
> matches-nothing prefix like `ABCDE` also still walks, but that is adversarial-only.) Class B's
> restructure is still required for the actual DoS.

Stating it this way because "the short-circuit fixes matches-nothing" would read as unconditional, and
someone would reasonably close Class B on it.

## `notices(source, id)` — the index that FAILED the gate, and the exception that ships it

This is the one Class A index that did not meet the acceptance criterion. It ships anyway, under a
**named, bounded exception with the numbers on record** — not by quietly widening the rule.

Measured on 14,237,839 real `notices` rows, query unchanged, deployed engine, best-of-3 per run:

| class | filter | rows | limit | before | after | |
|---|---|---|---|---|---|---|
| **dense** | `source=ted` | 13,134,618 (**92.3 %**) | 50 | **0.1 ×5** | **0.2 ×5** | **2× slower** |
| | | | 1000 | **1.3, 1.3, 1.3, 1.3, 1.3** | **2.1, 2.2, 2.2, 2.1, 2.1** | **~1.65× slower** |
| matches-late | `source=doe` | 1,103,221 | 50 | 83.8–88.1 ms | **0.1–0.2 ms** | ~500× |
| | | | 1000 | 87.4–88.7 ms | **2.2–3.2 ms** | ~33× |
| matches-nothing | `source=zz` | 0 | 50 | 3,893–7,573 ms | **0.0 ms** | >100,000× |
| | | | 1000 | 3,896–3,975 ms | **0.0–0.1 ms** | ~40,000× |

Plan after: `SEARCH notices USING INDEX notices_source_id (source=? AND id>?)` — parent 0, single line,
**no sorter** (checked by parent column, not by grepping for the word).

**The regression is real, not noise.** The same drop-remeasure-rebuild protocol that *dissolved* an
apparent 6 % on `tenders(source, id)` here *confirms* 65 %: before is 1.3 ms five times out of five,
after 2.1–2.2 ms five times out of five. Zero variance, zero overlap.

### Mechanism — and it generalises

`notices_source_id(source, id)` is **not covering**: the read selects eleven columns, so every row costs
an index seek *plus* a rowid fetch. The plain cursor walked the table sequentially with no indirection.
**When a filter matches 92.3 % of rows it buys almost no filtering, and the indirection is pure cost.**

The general statement: **a `(filter, id)` index helps in inverse proportion to the filter's selectivity
for the value being queried.** It is transformative for absent and late values and can be negative for a
dominant one. That is the mirror of the row-value trap recorded above — there a plan that looked
*better* was 4,209× worse; here an index that genuinely helps two classes costs the third, and the third
carries the traffic.

### The amended criterion (lead decision, 2026-08-03)

Dense-no-slower **remains the default**. An exception requires **all four** of:

1. sub-millisecond absolute regression;
2. the dense case stays fast in absolute terms;
3. a >100× win that closes a real DoS;
4. explicit lead sign-off with the numbers on record.

`notices(source, id)` meets all four: +0.8 ms to 2.1 ms absolute, closing a **≥226 s unauthenticated**
`?source=zz` walk. **This is a named exception for this index, not a global loosening** — the gate stays
strong precisely because it caught this rather than rounding it away.

### The clever fix that was considered and REJECTED

A partial index `WHERE source != 'ted'` would avoid the regression entirely by not indexing the dominant
value. **Rejected**: it hardcodes today's distribution into the schema. If source-dominance ever shifts,
it silently regresses whichever value becomes dominant — the same stale-judgement failure this issue
already documents elsewhere. Saving 0.8 ms on a 2.1 ms query is not worth a schema that rots.

### Build cost — and a correction to this issue's own memory constant

**1:05.12, peak RSS 674,524 KB ≈ 659 MiB at 14.24 M rows = ~48 B/row.**

The series across four indexes on four tables is now **41 / 45 / 45 / 48 B/row**. So the 45 B/row figure
used for the size guard is **central, not conservative**, and the widest key measured **exceeds** it.
An earlier note in this issue called 45 conservative on the strength of two points that happened to sit
at or below it; that was wrong, and the guard's constant should be the measured **maximum**, not the
middle. At 48 B/row a 44 M-row table projects to 2.11 GB, which puts `organization_mentions` (40.9 M)
at the boundary rather than comfortably inside it.

## CORRECTION: this issue's audit was incomplete — `/v1/tenders?kind=` is a fifth member

The four fixes deployed in `5c197e7` are real and verified. **The claim that they close the class is
not.** A fifth walk-capable read was missed by this issue's audit and found later, while deriving task
5's isolation routing from the code rather than from this issue's list.

`/v1/tenders?kind=` filters `t.kind`, which **no index covers** — `tenders_procedure_key`,
`tenders_island`, `tenders_current_published` and `tenders_source_id` are the whole set. Measured
locally rather than inferred from the schema:

| request | limit | time | rows |
|---|---|---|---|
| `?kind=procedure` (matches) | 50 | 0.0009 s | 50 |
| `?kind=zzz` (matches nothing) | 50 | **0.0437 s** | 0 |

48× — a full walk, the same shape as the filters fixed here. Its **magnitude at prod scale is
deliberately not extrapolated**: the members fixed here ran 10–1000× worse in production than naive
scaling from equivalent synthetics predicted, so 400k rows establishes *that* it walks and nothing about
*how long*. team-lead has authorised a prod measurement.

~~**Resolution: isolation, not an index.** A `(kind, id)` index over a two-value column is barely a
filter, carries the dense-regression risk of a non-covering shape, and would only make the
nothing-matching value cheap. `kind` also has no matches-late case — both real values are dense and
return under `LIMIT` immediately — so the only slow case is the adversarial nothing-match, which task 5's
isolated pool confines. No index, no new exception.~~

> **SUPERSEDED — "both real values are dense" was an unstated assumption about the corpus, and it is
> false. The correction is immediately below, so the claim cannot be quoted without it.**

### Correction: `?kind=` was resolved on a wrong ASSUMPTION, not a wrong method (proj-fix)

*Supplied by proj-fix and folded in verbatim by this file's single editor. Their attribution note: the
`18.7 s` figure and the "one Class B member an index can fix" framing are run-driver's and team-lead's;
the assumption-vs-method distinction and the `notices_fetch_id` parallel are proj-fix's own.*

Issue 117 resolved `/v1/tenders?kind=` to isolation rather than an index, reasoning that `kind` has only
two values so both are dense and only an adversarial nothing-match is slow. run-driver then measured
`?kind=registration` at **18.7 s on prod** — the second of two *documented* values, sent by ordinary
clients with no crafted input.

So the assumption was wrong, not the reasoning from it.

> ~~`registration` is rare enough in the data to be a **matches-late** case: it exists, so the
> short-circuit cannot veto it, and it sits late enough in id order that the walk runs nearly to
> completion before `LIMIT` fills.~~
>
> **RETRACTED 2026-08-05.** The *positional* half of this is measured false. It was proj-fix's
> inference, not a measurement, and run-driver's Phase A falsifies it: `registration` is rare and
> **EARLY**, not late. Retracted explicitly rather than quietly replaced — this passage has already
> been corrected once, and a file corrected once is exactly where a second silent correction gets
> missed.

**Measured replacement (run-driver, Phase A, 2026-08-04):**

`registration` is rare enough to be a **matches-exhausted** case: 120 rows in ~8.1M, clustered in id
deciles 1–2 (first match at id 1,127,544, ~14% in). It exists, so the short-circuit cannot veto it —
but because the read walks `t.id > ? ORDER BY t.id LIMIT 50` and can only stop early when `LIMIT`
*fills*, the cost falls on the pages *after* the cluster is consumed: page 3 onward walks the
remaining ~6.2M rows to the end and returns nothing. The general property is **fewer than `LIMIT`
matching rows remain after the cursor while much of the table is still ahead** — matches-late and
matches-exhausted are both instances of it, and prod is the second.

**The 18.7 s figure is a FLOOR, not the cost.** It was measured on the *first* page (`after=0`),
which is the cheapest case: the cluster sits at ~14%, so page 1 fills after walking a fraction of the
table. The expensive pages are **page 3 onward** — not page 2, which still has cluster matches to
return — and they are **unmeasured**. Each walks to the end of ~8.1M rows and returns nothing. Any bed
built to reproduce this (issue 122) must paginate past exhaustion rather than time `after=0`, or it
will measure the floor and call it the ceiling.

That flips the resolution. `t.kind` is a column of the **driven table**, unlike `country`'s
`EXISTS`-per-row or `lots`' joined-table filters, so it is the one Class B member that an index can fix.
`tenders(kind, id)` makes the ordinary `registration` request milliseconds; isolation confines it but
leaves a legitimate user waiting 18.7 s, which reads as broken.

**The distinction worth preserving:** the audit's *method* found the member correctly and classified it
correctly given what it believed about the data. What failed was an unstated assumption about the data
itself — that a two-value column has two dense values. Density is a property of the corpus, not of the
schema, and nothing in the schema said which. It is the same shape as `notices_fetch_id`'s "tens of
seconds" comment, written when that table was 3.5 M rows and still there at 27.4 M: a claim true of the
data when written, not re-checked when the data moved.

### What this says about the audit, and about the fix that found it

The audit was the best available and it was **incomplete**. Any hand-maintained list derived from it —
including task 5's routing table, had it been written by hand — would have inherited the gap silently.
Deriving the routing predicate from the code found the member *in the course of doing something else*.

That is also the first evidence for issue 120's thesis: the isolated pool caught an expensive read
**nobody had audited**, which is what a backstop is for. The general defence earned its keep before it
shipped.


## Did `tenders_source_id` cause the `/v1/lots?source=ted` regression? NO — plan instrument

Task #18, plan-level determination (sdk-vendor), **independent of the 117 deploy and of
run-driver's plan DBs**: the schema came from `Db::open()` — the code's own — and the
counterfactual needed no DDL editing at all, because `tenders_source_id` is a **deferred**
index and a freshly-opened DB therefore does not have it. The statement was taken from the
derived checked-set fixture, not retyped.

**The hypothesis was:** adding `t.source = ?` flips the driving table from `lots` to
`tenders` via 117's index, so `ORDER BY l.id` can no longer be satisfied by the drive and
`LIMIT 50` cannot stop early.

**Measured on turso 0.7.0:**

```
WITHOUT tenders_source_id:  1 | 0 | 0 | SCAN tenders AS t
WITH    tenders_source_id:  1 | 0 | 0 | SEARCH t USING INDEX tenders_source_id (source=?)
```

**The query drives from `tenders` either way.** Without the index the planner *scans*
`tenders`; with it, it *seeks*. Both plans then reach `l` by
`SEARCH l USING INTEGER PRIMARY KEY (rowid=?)` and both end in a top-level
`USE SORTER FOR ORDER BY`.

So on this evidence **117 did not cause the driving-table change — there was no change.**
It improved the drive step (scan → seek) on a query whose shape already prevented `LIMIT`
from stopping early. The sorter, which is the actual reason 50 rows cost a full pass, is
present with and without the index.

That also means **removing the index cannot fix this endpoint** — it would return the
drive to a full `tenders` scan while re-breaking the four DoS reads now at milliseconds.
Consistent with team-lead's not-a-rollback decision, reached independently.

### What this instrument cannot establish, stated rather than glossed

The counterfactual DB is **empty**. Plan choice can depend on table size and statistics,
and prod has 12.4M notices. The gate's own stats precondition covers the usual form of
this — schema-only plans are representative *while `sqlite_stat1` is empty*, which is
prod's state — but "empty vs populated, both without stats" is a further axis this run did
not vary. A plan is also text about an execution, never a duration (112 rule 6).

**So this settles attribution, not cost, and it needs run-driver's real-scale timing on the
`tenders.db` fixture to agree before the record calls it closed.** The two instruments can
genuinely disagree — independent construction and a clock, against a plan on an empty
schema — which is what would make agreement worth something.

## POST-DEPLOY RECORD — two corrections (sdk-vendor, sole editor, 2026-08-03)

### 1. `/v1/lots?source=ted` — 117 did NOT cause it, and it is NOT a rollback

**Verdict: the regression predates 117. 117 made it ~1.7x faster while leaving it
catastrophic.** Two instruments, built independently and reported before either saw the
other's result:

| instrument | setup | result |
|---|---|---|
| plan (sdk-vendor) | empty schema from `Db::open()` | drives from `tenders` with **and** without `tenders_source_id`; top-level sorter in both |
| clock (run-driver) | populated real-scale `probe.db` | no source-leading index **4.6 ms** · `tenders_island` **160 s** · `tenders_source_id` **93 s** |

**The cause is that ANY source-leading index flips the drive**, and one already existed:
`tenders_island` is `tenders(source, island_notice_id)` and predates 117 (present at
`1830d50`). So the pre-117 state was **160 s**, and 117's index made the same read 93 s.
Removing `tenders_source_id` would return it to `tenders_island` at 160 s while
re-breaking four DoS reads now at milliseconds — which is why this is not a rollback,
reached independently by both the plan half and team-lead.

#### A limitation of MY half, recorded because the record should not credit it with more than it did

My counterfactual was a fresh `Db::open()`, and **both** `tenders_island` and
`tenders_source_id` are deferred — so it had **neither**. That models a state prod has
never been in. It answers "does `tenders_source_id` *specifically* flip the drive?" but
the load-bearing question was "did the drive already flip before 117?", and the honest
comparison there is `tenders_island` vs `tenders_source_id` — which only the clock made.

The empty schema also shows its limits directly: my plan predicted a `tenders` **scan**
in the no-index case, and a full scan of 6.9M rows cannot be run-driver's 4.6 ms, so the
populated planner evidently chooses a different drive than the empty-schema plan
predicts. That is exactly the empty-vs-populated axis flagged when the plan half was
filed, and it is why the record was held open for the clock.

**So the two instruments agree on the verdict, and the plan half under-determined the
mechanism.** Both statements belong here: an agreement is worth what the weaker
instrument could actually have established, not what the conclusion sounds like.

### 2. `?kind=` — see the correction at the claim it corrects

proj-fix's text has arrived and is folded in **beside the superseded "both real values are dense"
sentence**, ~120 lines above, rather than here. A correction that far from its claim reads as a second
opinion, and the claim could still be quoted without it.

What stood here was my paraphrase of a one-line summary of that finding — a paraphrase of a paraphrase,
in the record of a defect a paraphrase caused. It existed only because their wording had not yet
arrived. It is gone; theirs stands, with their attribution intact.