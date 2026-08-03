# 117 — a plain `id > ?` cursor defeats every multi-column index across the paginated reads

Status: open — LIVE defect, measured on prod 2026-08-03 at rev `1830d50`. Pre-existing; not caused by
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
