# 439 — the resolver's triple preload binds to an arbitrary duplicate row

Status: RESOLVED-VERIFIED 2026-09-28 — verified on the day's new mentions. Was: DEPLOYED 2026-09-27 14:40 UTC (rev `9dedf49`, commit `94d2346`). Was: **BUILT 2026-09-27** (see the foot): the preload and both name probes bind the lowest id.
Uncommitted, not deployed. **Open decision, deliberately not taken here: what the shared Bavarian VAT DE811335517
identifies** (see Observed). Filed 2026-09-27.
Kind: defect (org layer — `Db::mention_resolver`'s identity preload and `resolve_one_mention`'s name probes in
`crates/store/src/canonical.rs`)
Relates to: 62 (the identity index is deliberately NOT unique, so a triple can stand on several rows), 300 (exemplar
40: row 1448 "caution", the shared-Land-VAT shape; exemplars 64 and 69 are the NRW and Italian twins of it), 329 (E0
merges same-triple duplicates), 234 / 351 / 432 (the `(name_norm, country)` reuse the name probes serve), 434 (a
refreshed mention re-resolves through these same lookups)

## Observed (2026-09-27, prod)

VAT `DE811335517` stands on **8** organization rows. New mentions carrying it land on whichever of the eight the
resolver's preload happened to keep. Row 1448, named `Vergabekammer Südbayern` (Regierung von Oberbayern), holds
**2,204 `Vergabekammer Nordbayern` mentions** — the Bavarian state publishes one VAT for both review chambers, so the
registration does not separate them, and which row a new Nordbayern mention joins is decided by scan order, not by
anything recorded.

## Mechanism

`mention_resolver` preloads `(country, identifier_kind, identifier) → id` with
`SELECT id, country, identifier_kind, identifier FROM organizations WHERE identifier IS NOT NULL` — no `ORDER BY` —
and fills the map with `org_of.insert(…)`. With duplicates, the LAST row the scan produced wins. A plain rowid walk
makes that the highest id today; a different plan (an index satisfying the `IS NOT NULL`, a future turso) would pick
another, silently. The two identifier-less probes (`(name_norm, country)` and the country-less `(name_norm, NULL)`)
had the same flaw in their own form: `LIMIT 1` without an order returns whichever match the chosen plan meets first.

## What to build

1. The preload orders by id and keeps the FIRST row per triple (`entry().or_insert`), so the binding row is the
   lowest id — the keep rule the merge arms already use.
2. Both name probes: `ORDER BY id LIMIT 1`.
3. Pin that the orders cost nothing: the preload's walk takes no sorter, and the probes still seek an index.

**The open decision** — what the shared VAT identifies — stays open, and this fix does not answer it. It makes today's
behaviour deterministic: new `DE811335517` mentions bind to row 1448, the lowest id, which is Südbayern, the row
already holding the 2,204 Nordbayern mentions. Separating the two chambers needs a rule that keys on the name as well
as the shared registration (the 300 exemplar 40 / 64 / 69 family), and is its own decision.

## Verify

    ssh -o BatchMode=yes root@zebreus.click 'curl -s --max-time 15 -H "Authorization: Bearer $(cat /root/tender-sql-token)" --data-binary "SELECT id, name, (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id) FROM organizations o WHERE identifier = '"'"'DE811335517'"'"' ORDER BY id" https://tenders.zebreus.click/v1/sql'

- **done**: after a fold that minted new `DE811335517` mentions, only the lowest-id row's count grew
- **open**: 8 rows, 1448 holding 2,204 Nordbayern mentions (2026-09-27)

## Built (2026-09-27)

Uncommitted, not deployed, not run on prod. Gate (`ops/check.sh`) not run; the focused suite below is green.

`crates/store/src/canonical.rs`: the preload reads `… WHERE identifier IS NOT NULL ORDER BY id` and fills `org_of`
with `entry(…).or_insert(id)`; the canonical-key poisoning beside it is order-independent and unchanged. The
`(name_norm, country)` probe and the country-less `(name_norm, NULL)` probe both read `ORDER BY id LIMIT 1`.

**Test** (`crates/store/tests/mention_refresh.rs`):
`the_triple_preload_and_the_name_probes_bind_the_lowest_id_among_duplicates` — three rows share
`(DE, vat, DE811335517)` (ids 5000, 1448, 9000): a mention binds **1448**; two identifier-less `stadt muster`/DE twins
(700, 300) bind **300**; the country-less probe's SQL returns 400 of (800, 400). With `build_organization_indexes`
run, `EXPLAIN QUERY PLAN` reads `SCAN organizations` for the preload (a rowid walk, no `TEMP B-TREE`),
`SEARCH … USING INDEX organizations_name_country (name_norm=? AND country=?)` for the scoped probe and
`SEARCH … USING INDEX organizations_name_norm_id (name_norm=?)` for the country-less one — no sorter anywhere.

**Red first**: with the unordered preload and plain `insert` restored, the test fails `left: 9000, right: 1448` —
the old code bound the LAST duplicate. The name-probe halves pass on the old code too (every plan turso chooses for
them yields rowid order); they now hold by the SQL rather than by the plan.

## Verify read 2026-09-28

all 9 `DE811335517` mentions of 2026-09-28's ingest (notice_id 46709000..46714265) bound to **1179**, the lowest-id row of the shared `(DE, vat, DE811335517)` triple. The older rows keep their historical counts (1179: 3,777; 1448: 18,552; 14988: 2,564; 23247725: 1,101), which the rule does not rewrite. **done.** What the shared VAT identifies stays open, as noted above.
