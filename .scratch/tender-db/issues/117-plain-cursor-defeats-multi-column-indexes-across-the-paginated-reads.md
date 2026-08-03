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
