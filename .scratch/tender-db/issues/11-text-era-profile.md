# 11 — Text-era profile (1993–2010, header-only)

Status: resolved
Blocked by: 09

Goal: text-era notices land as header-mapped single-source Tenders with
bodies preserved as declared text.

Scope: per docs/research/ted-legacy-mapping.md §text-era: parse the ~20-30
coded header fields (RN back-references feed chains), body stored as text
(encoding via encoding_rs — Latin-1 era), EN files only (schema stays
multilingual); ~30-item field-code checklist; fixtures from 1993/2005/2008
samples.

Acceptance: a 2005 daily ingests; XML-era chains referencing 2009/S numbers
terminate at real text-era records.

## Answer

Delivered: `crates/ingest/src/text/` (mod + rules + parse), the vendored
`sdk/text-inventory.json` checklist, `tests/text.rs`, a real Latin-1 fixture
(`tests/fixtures/text/2000-pin-130-2000.txt`), and the package/dispatch
plumbing the profile needed (`PackageContext` pre-scan, record spans).

Verified on the VPS (`/opt/tender-db/src-era11/`, scratch db) against three
full real dailies — **zero quarantines, zero unclaimed content, zero degraded
values, idempotent re-run (0 new rows, 2205 duplicates)**:

| daily | members | ingested | skipped | records | parsed |
|---|---|---|---|---|---|
| 1993-01-02 | 1 | 1 | 0 | **199** (= issue 02's count) | 199 |
| 2005-01-01 | 38 | 1 (EN UTF8) | 37 | **911** | 911 |
| 2008-01-03 | 46 | 1 (EN UTF8) | 45 | **1095** | 1095 |

The format declares no per-file record counts (the banner is a fixed 4-line
header, no trailer); the counts above match an independent Python sweep of
the same tarballs exactly.

Header fill rates (records with at least one value): PD/DS/CY/TI/AU/TX
**100 %** in all three years; deadline DT 100 % (1993), 61 % (2005), 49 %
(2008); CPV `PC` 100 % from 2005, pre-CPV `CC` 100 % in 1993; chain edge `RN`
105/199 (53 %), 294/911 (32 %), 400/1095 (37 %) — all consistent with the
research measurements (ted-legacy-mapping.md §7).

Design decisions, all measured against real files:

1. **Inventory is 34 codes, not ~30, and vintage-aware.** Swept the EN
   members of the 1993/2000/2005/2007/2008/2010 sample dailies: 24 codes in
   1993 (incl. the 1993-only `CC`/`CT` pre-CPV pair and `PG` OJ page),
   `OL`/`TW`/`PC`/`PN`/`OT`/`CO`/`RC`/`RG` from 2000, `IA`/`MA` from 2007.
   No official spec exists, so the vendored inventory records the sweep as
   the era authority; an unknown tag quarantines the record
   (`unknown-field-code`) — that is how the next vintage surprise surfaces.
2. **Line grammar**: `XX:` tags at column 0, indented continuations. Whether
   a continuation extends or repeats the value is per-field: prose fields
   join (`AU` wraps its name mid-sentence — measured), list fields are
   one-value-per-line (`PC`/`PN`/`RC`/`RG`/`CC`/`CT`/`RN`/`MA` — `MA`
   continuation lines occur in 59/1101 of the 2007 daily), scalars
   (dates/codes/ids) never wrap and a continuation under one quarantines.
   Bodies land as declared text: one `TXT-TX` row (EN), one `TXT-OT` row per
   block (untagged — a bilingual Belgian original publishes *two* OT blocks
   while `OL` declares one language), one `TXT-AB`. Values live in the single
   `PROCEDURE` section; codes drop their ` - label` display text (the
   XML-era CODIF_DATA convention); `RN` becomes `Id{scheme:"ojs", is_ref}` —
   the chain edge the projection's union-find joins on `(year, number)`,
   same as the r209 `REF_NOTICE` edges, so 2011 notices referencing 2009/S
   numbers terminate at these records.
3. **Mid-era dailies ship the EN delivery twice (ISO + UTF8)** — ingesting
   both would double every notice (911×2 in 2005). The UTF8 rendering is the
   richer one: the ISO twin mangles non-Latin-1 scripts (Greek OT bodies) and
   re-wraps lines. New walker-level policy: a cheap tar-name pre-scan
   (`package::entry_names` → `profile::PackageContext`) and the ISO member is
   `Skipped("text-era-iso-superseded-by-utf8")`. The `_meta_` variant stays a
   documented skip (decision recorded in the fixtures README).
4. **Encoding is declared by member name**: `_ISO_` decodes via
   `encoding_rs::WINDOWS_1252` (WHATWG's iso-8859-1; also absorbs the few
   C1 0x98 bytes real ISO dailies contain), `_UTF8_` decodes strictly and
   gets the five XML entities unescaped (`&amp;` etc. are a UTF8-variant
   serialization artifact — the ISO twin spells the same bytes unescaped).
   The Latin-1 path is regression-tested with a real 2000 record
   ("Bilbao Ría 2000", á/é/í/ó high bytes) — the fixtures-README TODO gap is
   closed.
5. **Multi-language identity holds**: a text-era Notice is
   (source, publication_id from `ND:`, content_hash of the record slice) —
   a later French ingest would be a separate notice row by hash, no model
   change needed; EN-only remains walker policy (claimed skips, never
   quarantine).

Gates: `cargo test --workspace` green, clippy zero warnings.

