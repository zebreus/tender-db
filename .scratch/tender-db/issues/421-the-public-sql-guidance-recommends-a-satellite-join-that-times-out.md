# 421 — the public SQL guidance recommends a satellite join that turso plans backwards, and it times out over any range

Status: **DONE 2026-09-26** — deployed at `e7e7358` 11:0x UTC (health green, no error lines) and the `## Verify` block reads done: the live `/v1/sql/schema` names the CROSS JOIN form (15 mentions across the six view notes and the overview), and the plain-JOIN line still reads 408, which is turso's behaviour and the reason the guidance changed. The plan test pins the order in the gate. Was: ready-for-agent — **BUILT 2026-09-26** (see the foot): the six view descriptions, the refusal message they feed and the `/v1/sql` overview now recommend `tenders t CROSS JOIN tender_version_… x`, pinned by a plan test; the prod-read guide carries the fifth trap row. Gate, deploy and the live Verify follow. Was: ready-for-agent — filed 2026-09-26 10:5x UTC from issue 243's check of whether a covering
index would help the public SQL surface. Measured on prod, reproducible, cause identified; the
guidance fix and a guard hint are unbuilt.
Kind: public API correctness of guidance (a documented query shape that cannot complete)
Relates to: 239 (views are not filterable, so the descriptions send users to base-table joins),
243 (where this was found), `docs/agents/prod-box-reads.md` (the planner traps table — this is a
fifth entry)

## What

Every `v_*` view description in `/v1/sql/schema` (`crates/app/src/v1/sql.rs` table descriptions)
says the view is NOT FILTERABLE and tells the user to "join `tenders` to
`tender_version_classifications` (or `_amounts`, `_dates`, `_parties`) on `(tender_id, seq =
current_seq)`". Written exactly that way over a modest id range, the join exceeds the 10 s cap:

| query (tenders 20,001..22,000) | as the guidance says | `tenders t CROSS JOIN …` |
|---|---|---|
| current CPV codes (`c.scheme = 'cpv'`) | **>10 s (408)** | 0.04 s (capped at 10,000 rows) |
| current titles (`x.field = 'title' AND x.lot_id IS NULL`) | **>10 s (408)** | 0.08 s, 1,981 rows |
| CPV with `+c.scheme` | 0.21 s | — |
| titles with `+x.field` | >10 s (408) | — |

A correlated subquery per tender (`SELECT t.id, (SELECT x.value FROM tender_version_texts x WHERE
x.tender_id = t.id AND x.seq = t.current_seq AND x.field = 'title' AND x.lot_id IS NULL LIMIT 1)
FROM tenders t WHERE t.id BETWEEN …`) is 0.03 s.

## Why

turso picks the satellite as the outer loop. For classifications the `scheme = 'cpv'` equality
matches the leading column of `tender_version_classifications_code (scheme, code)`, so it walks every
CPV row in the corpus and probes `tenders` by rowid per row. For texts there is no usable equality on
the satellite at all, and it still scans `tender_version_texts` first. The id range on `tenders`,
which would bound the work to ~2,000 rows, is applied last. A scratch DB with prod's schema shows the
same outer choice (`SEARCH c USING INDEX tender_version_classifications_code (scheme=?)`, `SCAN
tender_version_texts AS x`), and adding `(tender_id, seq, field|scheme)` indexes does not change it.

## Done when

- The view descriptions in `sql.rs` recommend a shape that completes: `FROM tenders t CROSS JOIN
  tender_version_… x ON x.tender_id = t.id AND x.seq = t.current_seq`, with one line saying why
  (turso otherwise drives from the satellite). Same text in `docs/` wherever the join is recommended.
- A plan test (scratch DB, prod schema) pins that the recommended CROSS JOIN form drives from
  `tenders` by its rowid range and seeks the satellite's `_version` index, for classifications,
  texts, amounts, dates and parties — so a turso upgrade that stops honouring CROSS JOIN order is
  caught in the gate.
- Optionally, when a `/v1/sql` query times out and its plan drives from a `tender_version_*` table
  while filtering `tenders`, the 408 body says so and names the CROSS JOIN form (238's message
  already names causes; this would be one more).
- The 20,001..22,000 pair above re-measured live: both under 1 s.
- `docs/agents/prod-box-reads.md`'s planner-trap table gains this as its fifth row.

## 2026-09-26 11:xx — which joins fail, measured; the guidance fix built

Every tenders-to-version-table join over the same 2,000 Tenders (20,001..22,000), plain `JOIN`
against `CROSS JOIN`:

| version table | plain JOIN | CROSS JOIN |
|---|---|---|
| `tender_versions` | 0.08 s | 0.04 s |
| `tender_version_parties` (`role LIKE '%uyer%'`) | **>10 s (408)** | 0.06 s |
| `tender_version_classifications` (`scheme = 'cpv'`) | **>10 s (408)** | 0.04 s |
| `tender_version_texts` (`field = 'title'`) | **>10 s (408)** | 0.08 s |
| `tender_version_amounts` | 0.09 s | 0.03 s |
| `tender_version_dates` | 0.10 s | 0.04 s |
| `tender_version_lot_results` | 0.08 s | 0.03 s |
| `tender_version_result_winners` | 0.05 s | 0.03 s |

The failures are exactly the joins that filter a column of the version table. A point read
(`t.id = 25808`) is fine either way (0.03–0.04 s). CROSS JOIN is never slower, so the guidance
recommends it for every tenders-to-version-table join rather than listing which ones need it.

**Built:** `sql.rs`'s descriptions for `v_tenders`, `v_tender_buyers`, `v_awards`,
`v_tender_classifications`, `v_tender_amounts` and `v_tender_dates` now show the CROSS JOIN form
and say why; the refusal message quotes that text, so a refused view read names it too; the
`/v1/sql` overview gains one paragraph with the example and the measured cost. Test
`the_recommended_version_joins_drive_from_tenders_and_seek` asserts every one of those notes says
CROSS JOIN, and reads turso's `EXPLAIN QUERY PLAN` on the real schema (deferred indexes built) for
seven version tables: `SEARCH t USING INTEGER PRIMARY KEY` first, then the version table sought by
`(tender_id, seq)`, and never a `SCAN` of it or the `(scheme, code)` index. Two existing refusal
tests pinned the old wording and now assert the new. `docs/agents/prod-box-reads.md`'s planner
table has the fifth row. The optional 408 hint is not built: the guidance fix reaches the reader
before the query is written, and the refusal already carries it for views.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "echo \"SELECT t.id, c.code FROM tenders t JOIN tender_version_classifications c ON c.tender_id = t.id AND c.seq = t.current_seq AND c.scheme = 'cpv' WHERE t.id > 20000 AND t.id <= 22000\" | /root/sq.sh | head -c 160; echo; curl -s https://tenders.zebreus.click/v1/sql/schema | grep -o 'CROSS JOIN' | head -1"

- **done**: the second line prints `CROSS JOIN` — the guidance names the form that completes (the
  first line may still read 408: the plain JOIN is turso's behaviour, not ours)
- **open**: an empty second line — the descriptions still recommend only the plain JOIN (read
  2026-09-26: the first line reads `query exceeded the 10s time limit`, the second is empty)

A bounded read through the public SQL surface and a public schema read, free per `prod-box-reads.md`.
