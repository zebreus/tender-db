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
