# 345 — standing identifiers pre-date the v2.1 normaliser folds: a re-normalise repair in the 328 shape

Status: BUILT 2026-09-04 (gate 813 green; deploying) — dry run on prod next, then wet, then a capped R2 pass; was: ready-for-agent (filed 2026-09-03 from the gate v2.1 deploy, `aa492dc`)
Kind: data quality / identity (organization layer) — the stock half of a prevention change
Relates to: 300 (gate v2.1, top-100 read), 328 (`repair-label-prefixes`, the template), 325 (`repair-minted-countries`, the ladder), 259 (refold-invariance: prevention alone leaves the stock split)

## Observed

`aa492dc` made `normalise_identifier` fold lookalike capitals (Greek `Ε` → `E`,
Cyrillic `А` → `A`, …) before the ASCII filter, and strip a Romanian `_n`
sub-unit suffix. Prevention only: the standing rows keep the value the OLD
normaliser produced, and the old value is what R2's `canonical_key` reads.

| org | stored identifier (old normaliser) | one raw mention value | new normaliser gives | collides with |
| --- | --- | --- | --- | --- |
| 1079 (GR, ΕΝΙΑΙΑ ΑΡΧΗ ΔΗΜΟΣΙΩΝ ΣΥΜΒΑΣΕΩΝ) | `1000009610001` (the Greek Ε was dropped) | `1000.Ε00961.0001` | `1000E009610001` | **311** (same authority, `1000.E00961.0001`) |
| 2439 (RO, DRDP Constanța) | `160543683` (the `_` was dropped, the digit kept) | `16054368_3` | `16054368` | **2438** (CNAIR S.A., the parent CUI) |

So the two exemplar splits the read found stay split until the stock is
re-derived. The class is larger than two rows — every identifier ever published
with a lookalike letter or an `_n` suffix — and its size is NOT readable from the
census (`org-merge-health` sees stored values, where the letter is already
gone); only the raw mention values know.

## Why `repair-label-prefixes` cannot do it

Its row filter is `label_prefix_stripped(&identifier).is_some()` on the STORED
value (canonical.rs ~12388): it selects label-prefixed rows only, and it
re-derives from the stored string. Both folds need the RAW string, which lives
in `organization_mentions.raw_identifier` (the division issue 311 relied on).

## Build: `repair-renormalised-identifiers(dry_run = true)`

The 328/325 ladder, one difference in the walk:

1. PK-watermark walk over `organizations` (`id > ? … LIMIT 50000`), rows with an
   identifier.
2. For each row, ONE mention's raw value: `SELECT raw_identifier FROM
   organization_mentions WHERE organization_id = ? AND raw_identifier IS NOT NULL
   LIMIT 1` — the `organization_mentions_org` index makes it a seek. (Rows whose
   mentions carry no raw value are skipped and counted.)
3. `normalise_identifier(raw, country)` through the injected live normaliser
   (the supervisor.rs seam 328 uses); plan the row when `(kind, country, value)`
   differs from the stored triple; count `already_clean`, `now_refused`,
   `no_raw`.
4. Dry: store the plan as a report (`put_report("renormalise-plan")`, with the
   pre-images); wet: the 328 parity guard (abort if the reviewed row count
   moved), rewrite the triple, change events per the 325 pattern.
5. Collisions ("reunions") are made VISIBLE and CORRECT, not merged — R2's
   next pass merges what its cross-walk allows (GR:afm and RO arms exist; the
   328 note's DE caveat does not apply to these two classes).

Cost: ~2.4M identifier-bearing orgs × one indexed seek — minutes, at idle, dry
first. Tests: a fixture with the two specimens (raw Greek Ε, raw `_3`) and a
clean row; dry plan names exactly the two; wet rewrites them; a second run
plans nothing (idempotent by construction — the stored value now equals the
re-derivation).

## Acceptance

* Dry plan on prod lists 1079 → `1000E009610001` and 2439 → `16054368` among
  its rows; the class size is the plan's row count (record it here).
* After wet + one R2 pass: the merge ledger shows 1079 → 311 and 2439 → 2438.
* The weekly `org-merge-health` gate block is unchanged by this (the folds
  create no new class); `phone`/`short_numeric` counts appear from Sunday.

## Built 2026-09-04 ~00:0x UTC

* `ingest::project::normalise_identifier_before_folds` — the normaliser as it
  stood before v2.1 (same code path, folds off), so the repair can tell a
  row's WITNESS mention (the one whose published string the old rules turn
  into exactly the stored triple) from mentions an R2/R3 merge brought in.
* `Db::repair_renormalised_identifiers(before, live, dry_run, expect_rows, stop)`
  — PK walk over identifier-bearing orgs, up to five `raw_identifier`s per org
  through `organization_mentions_org`, plan = rows whose witness re-parses to a
  different (kind, value) under the live rules; `unexplained` (no witness) and
  `now_refused` (the gate rejects the witness) are counted and left standing;
  reunions counted per target; wet arm = the 328 arm (guarded per-row UPDATE +
  change event, parity abort at max(2%, 5)).
* `repair-renormalised-identifiers` job (dry by default; wet reads the stored
  `renormalise-repair` dry plan's row count), STOPPABLE, `Box::pin`ned arm.
* Tests: `crates/store/tests/renormalise_repair.rs` — the Greek-letter row and
  the suffixed RO row move onto their standing twins (2 reunions), the
  merged-in-only row and the string-less row stand, the published string
  survives in the mention, a second pass plans nothing, the parity abort
  holds; the ingest test pins the before-folds twin against the v2.1 cases.
