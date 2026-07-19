# Real-data sizing pilot + XML parser probe

Research date: 2026-07-19. Closes SUMMARY.md gaps **A2** (real-data DB size /
75 GB disk arithmetic), **A10** (XML parser crate evaluation + parse
throughput) and **A19** (AUTOINCREMENT probe). Everything below marked
**[measured]** was run on the production VPS (`root@zebreus.click`, 4 vCPU,
7.6 GiB RAM, 75 GB disk); tool, scripts, logs and result databases live under
`/opt/tender-db/pilot/` (`tool/` Rust crate, `runloads.sh`, `runloads.log`,
`ef-*.db`, `r209-*.db`, `doe-b.db`). Binaries release-mode, rustc 1.97.1,
turso `=0.7.0`.

**Corpora** (all real archive data):

| Corpus | Files | Raw bytes | Notes |
|---|---|---|---|
| TED monthly 2026-06 (`packages/monthly/2026-6`) | **78,480** eForms notices | 4.166 GB unpacked (395 MB tar.gz → **10.5×**) | avg **53.1 KB/notice**; full production month |
| TED daily 2019-01-02 (R2.0.9) | 1,529 notices | 31.1 MB | avg 20.3 KB/notice |
| DÖE eForms sample (`samples/oeffentlichevergabe`) | 567 notices | 9.2 MB | avg 16.2 KB/notice; 346/567 use JAXB `ns2..ns9` prefixes |
| Text-era EN files (1993 / 2005 / 2008-meta) | 199 / 911 / 1,095 records | — | 2.0 / 6.8 / 4.6 KB/notice raw; 0.4–1.4 KB gzipped |

---

## 1. Part A — XML parser probe

### 1.1 Requirements recap

Namespace-aware matching **by URI + local name** (DÖE serializes with JAXB
`ns2..ns9` prefixes — prefix matching is a known bug class), exhaustive
consumption ergonomics for ADR-0004 (every element/attribute consumed or
explicitly ignore-ruled), era encodings, and enough throughput for the
rebuild-from-archive recovery path.

### 1.2 Encoding survey [measured]

- **Every XML-era file is UTF-8.** Declared encodings across 2,000 sampled
  files from 2011/2014/2019/2026 packages: only `UTF-8`/`utf-8`; byte-level
  validation of 200 files from the 2011 package: all valid UTF-8.
- **Latin-1 exists only in the pre-2008 text era**, which is *not XML*
  (`DE_20000104_001_ISO_ORG`: raw 0xFC/0xD6 bytes, not valid UTF-8, valid
  ISO-8859-1). The 2008+ `meta` variant is valid UTF-8; the 1993 EN file is
  pure ASCII.
- Consequence: **the XML parser needs no Latin-1 support at all.** Text-era
  ingestion decodes with `encoding_rs` (already in our Cargo.lock via dioxus)
  before line-oriented parsing. quick-xml's `encoding` feature is unnecessary.

### 1.3 Candidates

None of the three is currently in our Cargo.lock (dioxus pulls no XML crate);
any choice adds one dependency.

| Crate | Version (checked 2026-07-19) | Model | Namespace handling | Notes |
|---|---|---|---|---|
| quick-xml | 0.41.0 (2026-06-29, 336 M downloads) | streaming | `NsReader` resolves per event (URI + local name) | fastest; consumer must track element context/state manually |
| roxmltree | 0.21.1 (2025-10-12, 58 M downloads) | DOM (read-only, arena) | every node exposes resolved `namespace()` + local name | UTF-8 `&str` input only; parser self-contained (no deps) |
| xml-rs | 1.0.0 (2025-10-27, 129 M downloads) | streaming | resolved `OwnedName` per event | allocation-heavy; 10× slower [measured] |

All three actively maintained.

### 1.4 Measured single-core throughput [measured]

Full parse + visit of every element and attribute with namespace resolution
(the shape ADR-0004 ingestion needs), release build, warm cache:

| Corpus | roxmltree | quick-xml | xml-rs |
|---|---|---|---|
| eForms 2026 (4,000 files, 208 MB) | **3,683 notices/s/core** (191 MB/s) | 4,709 (245 MB/s) | 439 (23 MB/s) |
| R2.0.9 2019 (1,529 files, 31 MB) | 7,806 (159 MB/s) | 8,477 (172 MB/s) | 1,010 |
| DÖE ns2..ns9 (567 files, 9 MB) | 11,289 (183 MB/s) | 16,674 | 1,385 |

All 80,576 real notices parsed by roxmltree in the pilot (78,480 + 1,529 +
567) parsed **without a single error**, including all ns-prefixed DÖE files —
URI-based matching worked with zero special-casing.

### 1.5 Recommendation: **roxmltree**

- The import pipeline is **DB-writer-bound, not parser-bound** (§3.3): the
  single turso writer sustains ~1.1–1.9 k notices/s while 4 cores parse
  ≥ 15 k notices/s. quick-xml's 28 % single-core edge buys nothing.
- ADR-0004's exhaustive consumption maps naturally onto a DOM: walk the tree,
  mark each node/attribute as consumed or ignore-ruled, and any *unvisited*
  node at the end is the quarantine reason. Doing that over streaming events
  means hand-maintaining a stack machine per profile.
- Namespace-URI + local-name matching is the default API
  (`node.tag_name().namespace()/name()`), not an opt-in.
- Memory is a non-issue: avg 53 KB, DOM of the largest observed notices is a
  few MB. Fallback if a pathological giant notice ever appears: quick-xml
  `NsReader` for that path.

xml-rs is eliminated (10× slower, no offsetting advantage).

## 2. Part B — sizing pilot

### 2.1 What the throwaway tool does

`/opt/tender-db/pilot/tool` (Rust, roxmltree + zstd + turso 0.7.0): parses
notices with 4 worker threads and one writer (400 notices/tx, FK on,
`synchronous=NORMAL`), dispatching eForms vs `TED_EXPORT` on the root
element, into a rough-but-representative normalized draft schema (STRICT):

- `notices` — metadata (pub number, era, root, SDK version, subtype,
  procedure id, uuid, date, country, language) + raw sizes + optionally the
  raw XML BLOB (zstd-3 / plain / absent);
- `lots` (id, cpv, deadline), `org_mentions` (name + full address + contact,
  from eForms `Organization/Company` and legacy `ADDRESS_*` blocks),
- `texts` — every multilingual text (eForms: every `@languageID` element;
  legacy: `TITLE`/`SHORT_DESCR`/… + the 24-language `ML_TI_DOC` block),
- `classifications` (CPV/NUTS), `amounts` (every `@currencyID`/`@CURRENCY`);
- 10 realistic secondary indexes, created after load, then `ANALYZE`.

Not complete, not correct — representative of bytes-per-notice. Rerun:
`./runloads.sh` (six variants), `pilot probes probes.db`.

### 2.2 Measured DB sizes [measured]

Satellite volume per eForms notice: **3.2 lots, 4.0 org mentions, 38.2 text
rows (15.0 KB of text!), 14.9 classifications, 7.9 amounts** (R2.0.9:
3.5 / 4.8 / 36.2 / 18.0 / 5.0 — every legacy notice carries ML titles in all
24 languages).

| Run | raw XML in DB | texts | final DB (with indexes) | **bytes/notice** | index share | end-to-end |
|---|---|---|---|---|---|---|
| eForms A | zstd-3 BLOB | yes | 2.257 GB | **28,765** | 5.2 % | 1,047 notices/s |
| eForms B | none | yes | 1.751 GB | **22,307** | 6.7 % | 1,129 notices/s |
| eForms C | none | no | 0.211 GB | **2,683** | 36.8 % | 2,270 notices/s |
| eForms D | plain BLOB | yes | 5.977 GB | **76,161** | 2.0 % | 550 notices/s |
| R2.0.9 A | zstd-3 BLOB | yes | 32.3 MB | **21,118** | 6.7 % | 1,393 notices/s |
| R2.0.9 B | none | yes | 22.7 MB | **14,873** | 9.5 % | 1,472 notices/s |
| DÖE B | none | yes | 3.0 MB | **5,281** | 10.8 % | 3,123 notices/s |

Decomposition (eForms, per notice): **multilingual texts are the size** —
B−C = **19.6 KB/notice (86 % of the no-raw DB**; `dbstat`: texts table
1,430 MB of 1,670 MB). Structured core + indexes is only 2.7 KB. Raw-zstd
in-DB costs A−B = 6.5 KB vs 5.7 KB of pure zstd bytes (~13 % BLOB paging
overhead). zstd-3 ratios: eForms 9.25×, R2.0.9 3.68×, DÖE 4.98×.

### 2.3 Raw XML: in-DB BLOB vs archive on filesystem → **filesystem**

- The fetched packages *are* already the raw archive: package-level tar.gz
  costs ~4.0 KB/notice (XML era, 36 GB/9 M) vs 5.5–5.7 KB/notice as
  per-notice zstd BLOBs (cross-notice redundancy lost) **plus** ~13 % BLOB
  overhead → in-DB raw is ~**+58 GB** at XML-era scale vs +0 for keeping the
  tar.gz we must download anyway.
- Raw-in-DB grows every backup/copy/checkpoint operation by the same amount
  (turso-scale already shows big-file ops are the pain point), and D (plain)
  is ruinous (76 KB/notice).
- ADR-0004's quarantine/reprocess only needs a stable reference:
  `(package, filename)` + package SHA-256. Fetch idempotency is hash-based
  anyway.

## 3. Extrapolation and the disk budget

### 3.1 Notice-layer DB, extrapolated from measured bytes/notice

Backlog counts from ted-access-channels/german-portals: TED eForms era
≈ 2.4 M (late-2022→mid-2026), DÖE ≈ 1.0 M (~22.5 k/mo × 44 mo), TED legacy
XML ≈ 6.6 M (9.0 M XML-era minus eForms), text era ≈ 3.9 M notices (EN).
(The task's working number "eForms-only ≈ 1.5 M" undercounts: the measured
era mix gives 2.4 M TED-side alone; scale linearly if scoped differently.)

| Corpus | N | bytes/notice (B-variant) | notice-layer DB |
|---|---|---|---|
| TED eForms | 2.4 M | 22,307 [measured] | 53.5 GB |
| DÖE | 1.0 M | 5,281 [measured] | 5.3 GB |
| TED legacy XML | 6.6 M | 14,873 [measured] | 98.2 GB |
| Text era (EN) | 3.9 M | ~2,000 [estimated: 2.7 KB core minus satellites + ~0.5–1.4 KB gzipped text blob] | ~8 GB |

### 3.2 Versioned-canonical overhead (stated assumption)

The canonical layer is undesigned (B3/B1), so this is a declared multiplier,
not a measurement: with the observed notice-version ratio of ~1.3–1.5
publications per logical notice, one canonical version per notice version
puts the canonical layer at **0.7–1.0× the parsed notice layer** (unique
entities ≈ notices/1.4 at full-copy versioning ≈ ×1.0; validity-ranged rows
re-versioning only changed entities, or text storage shared with the notice
layer, pushes toward ×0.7 or below). **Total DB = notice layer × 1.7–2.0.**
Replace this with real numbers as soon as a canonical draft schema exists.

### 3.3 Disk budget for the 75 GB VPS (≈ 68 GB usable)

Components: raw compressed archive on filesystem, total DB (notice layer ×
1.7–2.0), checkpoint-copy backup staging ≈ 1× total DB on the same disk
(turso-scale: VACUUM INTO is dead; checkpoint+copy is the backup path), WAL +
slack ~2 GB (WAL stayed ≤ ~36 MB during loads).

| Scenario | Raw FS | Notice layer | Total DB (×1.7–2.0) | + backup staging | **Grand total** | Fits 75 GB? |
|---|---|---|---|---|---|---|
| **eForms-only** (TED 2.4 M + DÖE 1.0 M) | ~15 GB | 59 GB | 100–118 GB | 100–118 GB | **217–253 GB** | **NO** |
| **XML era** (+ legacy 6.6 M) | ~40 GB | 157 GB | 267–314 GB | 267–314 GB | **576–670 GB** | **NO** |
| **XML era + text-EN** (+ 3.9 M) | ~48 GB | 165 GB | 280–330 GB | 280–330 GB | **595–710 GB** | **NO** |

**No scenario fits.** Even the most favorable cut — eForms-only, notice layer
only, no canonical layer, no on-box backup — is 15 + 59 = **74 GB: the disk
is full on day one**, before the ~30 GB/yr growth (TED 940 k + DÖE 300 k
notices/yr → ~23 GB/yr notice layer, ~6 GB/yr raw, ×2 with canonical +
staging). This kills turso-scale's synthetic 2.4 KB/notice → "40 GB working
assumption": real normalized notices are **6–9× fatter**, and the driver is
the multilingual text satellite, exactly as suspected.

Levers, in order of impact:

1. **Hetzner cloud volume** (attachable block storage, ~€0.05/GB/month —
   order of €25/mo for 500 GB; verify current pricing): a 300 GB volume
   covers eForms-only incl. staging; 750 GB covers the full XML era. Cheap,
   solves it outright, no schema compromise.
2. **Off-box backups instead of on-box staging** (e.g. Hetzner Storage Box,
   ~€4/mo for 1 TB): halves every scenario (eForms-only → 117–135 GB, XML
   era → 309–356 GB). Likely wanted anyway (C24 — off-box backup target).
3. **Compress the texts table** (per-row or dictionary zstd on the 86 %
   share): plausibly ÷2–3 on the dominant component at the cost of
   SQL-endpoint transparency (texts no longer LIKE-able without a UDF —
   conflicts with the plain-SQL product promise; a `texts_compressed`
   satellite + decompressed hot fields is a middle path). Unmeasured.
4. **Language policy** (C12): storing only original-language + EN for the
   24-language legacy ML blocks and multi-language eForms fields would cut
   the legacy texts substantially — a data-fidelity decision, not a technical
   one.

### 3.4 End-to-end throughput and backfill wall-clock [measured → extrapolated]

Measured end-to-end (read + parse + extract + zstd + insert with FKs +
index build + ANALYZE, 4 parse workers, 1 writer): **1,047–1,129 eForms
notices/s**, 1,393–1,472 R2.0.9/s, 3,123 DÖE/s; ~48 MB/s DB growth,
~120 k rows/s (small rows — consistent with turso-scale's 38 MB/s on fat
rows). The pipeline is **writer-bound, not parser-bound** (EF-C, with most
insert volume removed, jumps to 2,270 n/s; parse capacity alone is ≥ 15 k
n/s across 4 cores) — turso-scale §5's "import will be parser-bound" guess
had it backwards, but the absolute numbers are comfortable:

| Scenario | Parse+load wall-clock | + download (measured ~760 Mbit/s: 395 MB/4.2 s) |
|---|---|---|
| eForms-only (3.4 M) | **~40 min** | +~5 min (15 GB) |
| XML era (9.4 M) | **~2.2 h** (allow 2–6 h: >100 GB B-tree behavior unbenchmarked; turso-scale verified only to 10 GB) | +~10 min (40 GB) |
| + text era EN (13.3 M) | +~20 min | +~2 min |

Canonical-layer projection is *not* included (not designed yet). Full
rebuild-from-archive — the stated recovery path for bad merges — is an
afternoon, not days.

## 4. Probes (turso 0.7.0) [measured]

- **`INTEGER PRIMARY KEY AUTOINCREMENT`: works and is monotonic.** STRICT
  table accepted; ids continue after deleting the max rows (4,5 deleted →
  next id 6), after `DELETE` of *all* rows (→ 7), and across close/reopen
  (→ 8); `sqlite_sequence` exists and tracks correctly. Load-bearing for the
  change cursor: **green**.
- **zstd BLOB round-trip: exact.** Compressed BLOB stored and read back
  byte-identical (SQL `length()` agrees), decompresses to the original.
  (zstd via the `zstd` crate already in our lock.)

## Implications for tender-db

1. **Buy disk before backfill.** Even the smallest scenario (eForms-only,
   no canonical, no backups) fills the 75 GB VPS on day one. A Hetzner
   volume (≈ €25/mo for 500 GB) + off-box backup target is the obvious
   combination; the XML-era backfill (recommended C1 option b) needs
   ~350–700 GB depending on where backups stage.
2. **The multilingual texts satellite is 86 % of the parsed DB** — every
   size decision routes through C12 (language policy) and the
   texts-compression lever. Decide those *before* freezing the notice-layer
   schema (B3).
3. **Raw XML lives on the filesystem** as the fetched tar.gz packages;
   the DB stores `(package, filename, sha256)` references. Do not store
   per-notice raw BLOBs (worse compression, +13 % overhead, doubles backup
   surface).
4. **Parser: roxmltree** (DOM, namespace-URI matching, natural
   exhaustive-consumption bookkeeping for ADR-0004); decode text-era files
   with `encoding_rs` (Latin-1 is confined to the non-XML text era; all XML
   eras are UTF-8). Import is writer-bound, so DOM costs nothing.
5. **Backfill wall-clock is a non-problem** (~2 h XML era, notice layer);
   re-import-from-archive is a viable routine operation, which strengthens
   ADR-0001's "archive is the source of truth" and the reprocess-after-fix
   quarantine loop.
6. **AUTOINCREMENT is safe to build the change cursor on** (monotonic across
   deletes and reopen, `sqlite_sequence` intact).

## Open questions

**Needs research**

- Per-row / dictionary zstd on the texts table: actual ratio and query cost
  (unmeasured; plausibly ÷2–3 of the dominant DB component).
- Canonical-layer real size: replace the ×1.7–2.0 assumption once a draft
  canonical schema exists (B3/B4); the multiplier dominates the budget.
- turso behavior at 100–300 GB DB size (bench stops at 10 GB): index depth,
  checkpoint cost, `CREATE INDEX` on 100 M-row tables.
- DÖE bytes/notice was measured on a 567-notice sample; spot-check a full
  DÖE month before trusting the 5.3 KB figure for capacity planning.
- Text-era parsed size (~2 KB/notice) is an estimate; a text-era profile
  pilot (conditional on C4) would pin it.

**Needs user decision**

- **Hetzner volume purchase** (size hinges on backfill depth C1 and backup
  target C24): ~300 GB for eForms-only, ~500–750 GB for the XML era;
  alternatively 500 GB + Storage Box for off-box backups.
- Language/text policy (C12) and whether the texts satellite may be stored
  compressed (SQL-endpoint transparency trade-off).
- Confirm raw-archive-on-filesystem as the raw-payload store shape
  (interacts with ADR-0004 quarantine references and D7 artifact safety).
