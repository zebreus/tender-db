# 536 — `publication_id=` is an exact string match across three stored TED widths: TED's own spelling, the OJ S form and the `/docs` example find nothing, and a wrong-width pad can return an FTS notice

Status: ready-for-agent — UNITS 1 AND 2 BUILT AND REVIEWED 2026-10-14 (uncommitted in the worktree; a three-lens review's findings applied, focused tests green, see "Review round" below). The precondition read RAN (by the owner, 2026-10-14 ~20:10 UTC): 0 pairs in both directions, recorded under "Precondition read". Next: `ops/check.sh`, commit, deploy, and read both Verify lines. Was: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, the TED-scoped normaliser (S–M), starting with its bounded precondition read.
Kind: API (lookup contract on `/v1/notices` and `/v1/tenders`) + docs (`/docs` lookups table, the OpenAPI `publication_id` parameter)
Relates to:
- 217 (RESOLVED 2026-08-16/17): shipped the exact match as "the key real consumers hold". Its own consumer
  scenario, `2026/S 123-456789`, is the OJ S form, and that form still answers `[]`. Padding and form were never handled.
- 336 (CLOSED NOT-WORTH-IT 2026-09-01): "an unmatchable value is indistinguishable from no results", closed after
  counting zero real callers. It does not govern here: the failing spelling is TED's own canonical form, and the
  `/docs` example itself fails, so this is not a shrinking legacy habit. Its measure-callers step is optional below.
- 391 (DONE 2026-09-18): the docs-example guard (`crates/app/tests/api.rs:2263`) asserts only non-4xx, and its doc
  comment deliberately excludes examples naming a prod id. The dead `/docs` example is outside its scope, not a bug in it.
- 481 (ready-for-agent): `00123456-2026` is SCB's real notice, found cited as a copied placeholder
  (`docs/operations.md:891`). It is a poor `/docs` example number. 481 is cross-source dedup, not lookup.
- 394 (done 2026-09-29): DÖE placeholder ids (`00000000-1900`). DÖE identity is `<uuid>-<NN>`; no lookup normalisation.
- 342 / 477 (done): FTS ids are `NNNNNN-YYYY`, taken verbatim from the release id. Neither records a decision about
  cross-source number handling.
- ADR-0011 (`publication_ref`, `crates/ingest/src/project.rs:6328`) and `ojs_key` (`project.rs:9378`): the ingest side
  already pads OPP-090 references to 8 digits and parses all three TED forms ("The raw string is never the key — its
  shape is not stable across eras"). The API lookup uses neither.
- No open issue covers lookup normalisation, the per-era widths, the dead `/docs` example or the wrong-source answer.

## Verify

    curl -s "https://tenders.zebreus.click/v1/notices?source=ted&publication_id=83685-2026" | jq -c '[.items[].id]'

This uses TED's own spelling and does not depend on the FTS policy. It reads unit 1; unit 2's check is the last
Repro line.
- **done**: `[25553765]` (the eForms notice stored as `00083685-2026`).
- **open (2026-10-14 16:08 UTC)**: `[]`.

## Repro

Public GETs on `/v1/notices?publication_id=…`, 2026-10-14 16:08 UTC, printed as `[id, source, profile]`:

| asked | answer | note |
| --- | --- | --- |
| `79997-2017` | `[]` | TED's own spelling |
| `079997-2017` | `[19163916, ted, ted-export-r209]` | stored 6-digit form |
| `2017/S%20044-079997` | `[]` | OJ S display form |
| `%20079997-2017` | `[]` | one leading space; no trim |
| `123456-2026` | `[]` | the `/docs` example |
| `00123456-2026` | `[25539652, ted, eforms-sdk-1.13]` | |
| `83685-2026` | `[]` | TED's own spelling |
| `083685-2026` | `[47158804, fts, fts:ocds-1.1]` only | 6-digit pad of a TED eForms number |
| `00083685-2026` | `[25553765, ted, eforms-sdk-1.13]` | |
| `123-2005` / `000123-2005` | `[2463391, ted, text]` / `[]` | text era is unpadded |
| `012345-2023` | `[22873499, ted, r209]`, `[46949236, fts]` | TED/FTS collision |
| `920ee1cf-ddd8-41c9-9cb5-aad114899550` | `[]` | DÖE without `-01` → 26403505 |

`/v1/tenders?publication_id=79997-2017` gives `[]`; `079997-2017` gives `[5792500]`. A publication_id page carries no
`resolved_filters`.

Unit 2's check: `curl -s https://tenders.zebreus.click/docs | grep -c 'publication_id=123456-2026'` printed `1`
(open); done is `0`, with a replacement example that returns a row.

## What was found

The lookup is raw string equality:
- `crates/app/src/v1/mod.rs:986` passes `self.publication_id.clone()` through untouched: no trim, no padding, no form
  parsing. `kind` and `name_prefix` are normalised in the same block.
- `crates/store/src/read.rs:4612` (notices) and `read.rs:2205` (the tenders seed over `tender_versions`) compare it
  with `publication_id = ?`.

The stored spelling is whatever each format's file carries (`crates/ingest/src/profile.rs`). The width follows the
notice's format, not its year:

| format / source | stored | prod examples |
| --- | --- | --- |
| TED text era 1993–2010 | unpadded | `123-1999` 1317822, `123-2005` 2463391, `123-2008` 3287185, `123-2010` 3962982 |
| TED internal OJS 2008 | unpadded (from code only: `internal_ojs_identity`, `profile.rs:262`, builds `{number}-{year}` from the file stem) | none read; `123-2008` above is profile `text` |
| TED XML R2.0.7–R2.0.9, 2011–2024 | 6 digits | `000123-2011` 4352477, `079997-2017` 19163916, `012345-2024` (r209) |
| TED eForms, 2022– | 8 digits | `00615938-2024` 24324996, `00123456-2026` 25539652 |
| FTS (UK), 2021– | 6 digits, release id verbatim | `083685-2026` 47158804 |
| DÖE | `<uuid>-<NN>` | `920ee1cf-…-aad114899550-01` 26403505 |

R2.0.9 6-digit numbers run into 2024 (`600123-2023`, `650000-2022`, `012345-2024`) beside 8-digit eForms numbers, so
for 2022–2024 a consumer cannot tell from the year which width to try.

**How TED prints its numbers** (the refute, 2026-10-14). One POST to TED's search API
(`api.ted.europa.eu/v3/notices/search`, `publication-number IN (79997-2017 00083685-2026)`) returned
`79997-2017` and `83685-2026`. TED serves its numbers unpadded for both R2.0.9 and eForms, accepts padded input
and canonicalises it, and treats the number as one integer per year. So TED's canonical spelling never matches
here, for any era after 2010.

Consequences:
1. **A consumer must know the stored width.** TED's own form fails for every 2011+ notice.
2. **The OJ S display form is refused** (`2017/S 044-079997` → `[]`), although the OpenAPI text calls the parameter
   "the OJ S / TED notice id". Whitespace is not trimmed.
3. **The `/docs` example is dead.** `GET /v1/notices?publication_id=123456-2026` (`docs.rs:266`) returns `[]`.
4. **A wrong-width pad can return another source's notice.** FTS numbers share the 6-digit TED shape.
   - The collision on an unpaired lookup is known: `000500-2021`, `012345-2022/2023/2024` each return one TED and one
     FTS notice, and `/docs` (`docs.rs:271`) says "pair with `source=`".
   - Not documented: `083685-2026`, TED's 83685-2026 at 6 digits, returns only FTS 47158804. The row is labelled
     (`source: fts`, `profile: fts:ocds-1.1`), so it is not silent. The defect is a wrong-source row with no hint that
     TED holds the number as `00083685-2026`.
5. **A DÖE UUID needs its `-NN` version suffix.**

Neither `/docs` (`docs.rs:271`: "exact match on the number the source printed") nor the OpenAPI parameter
(`openapi.json:803`) names the widths, the FTS/DÖE shapes, or that the OJ S form is not accepted.

**Unmeasured.** Whether FTS prints an OJ S-style form (`2026/S 000-083685`): find-tender.service.gov.uk returned 403
through the proxy. That internal OJS 2008 rows are unpadded rests on the code, not a prod read.

### What the external review got wrong

- "6 digits for older TED notices" is false for 1993–2010: the text era (and, by code, internal OJS 2008) is stored
  unpadded, and `000123-2005` finds nothing. The 6-digit width starts with TED XML in 2011. There are three TED
  widths (unpadded, 6, 8), not two.
- It implied width follows age. It follows format: 6-digit R2.0.9 numbers run into 2024 beside 8-digit eForms.
- "UK numbers have the same shape" is true, but the plain collision is already documented. The undocumented part is
  the wrong-source answer to a TED number padded to 6 digits.
- It understated the problem: the `/docs` example, the OJ S form (217's own scenario), untrimmed input and the bare
  DÖE UUID all return nothing.

Do not re-file the first verification draft's errors either: a full `{n}`/`{n:06}`/`{n:08}` expansion across all
sources (it would make `83685-2026` return FTS 47158804 too); "silently" for the FTS answer; the 391 guard as buggy.

## Proposed units

1. **Normalise TED numbers at the API boundary, once (S–M).**
   - **Precondition read first.** Plan it locally, then run it bounded per `prod-box-reads.md`. In 2023, where R2.0.9
     and eForms coexist, take a window of 6-digit TED numbers (index range on `notices_publication_id_id`, `LIMIT`) and
     seek each one's 8-digit spelling, then the reverse window. Expect 0 pairs. TED's API already treats the number as
     one integer per year; this read confirms that the store agrees.
   - **Where.** In `v1/mod.rs` beside `kind` and `name_prefix`. Trim, then parse with `ojs_key`'s grammar: OJ S
     `YYYY/S III-N`, or `N-YYYY` at any padding. Share the one parser between ingest and app; do not write a second.
   - **SQL.** `publication_id IN (trimmed input, {n}-{y}, {n:06}-{y}, {n:08}-{y})`, deduplicated. The number stays the
     only SQL predicate (217-A, the comment at `read.rs:4606`); companions post-filter in Rust. The tenders seed's
     "MUST take precedence" rule (`read.rs:2203`) holds. Read both plans locally: at most 4 seeks on
     `notices_publication_id_id` / `tender_versions_publication`, no scan.
   - **Scope the expansion to TED** in the Rust post-filter. Keep a `source='ted'` row when its stored spelling is in
     the expansion set. Keep a non-TED row (FTS, DÖE) only on an exact match of the trimmed input. On `/v1/tenders`,
     `tender_versions` has no `source` column: test the matched version's notice (`caused_by_notice_id`, a PK seek),
     not `t.source`, unless it is shown a tender never holds versions of two sources (481).
   - **Acceptance:**
     - `83685-2026` → TED 25553765 only;
     - `083685-2026` → FTS 47158804 and TED 25553765 (genuinely ambiguous; `source=` narrows);
     - `00083685-2026` → TED 25553765 only;
     - `79997-2017`, `2017/S 044-079997` and ` 079997-2017` → 19163916; `/v1/tenders?publication_id=79997-2017` → 5792500;
     - `000123-2005` → 2463391.
   - **Unparsed input stays exact.** `ojs_key` rejects a DÖE `<uuid>-<NN>` (year `01` is out of range) and
     `00000000-1900` (number 0). A bare DÖE UUID by version-stem range (the `LINK_VERSIONED_TARGET_SQL` shape) is
     optional; size it separately.
   - **Decide the OJ S issue-`000` route** before shipping: if FTS prints `YYYY/S 000-NNNNNN`, that input must also
     reach the FTS row at `{n:06}-{y}`, not TED's widths only.
   - **Report the expansion** in `resolved_filters.publication_id`, `{"asked": …, "matched": […]}`, the shape the
     identifier lookup's `merged_into` uses.
   - **Tests** (`--features server`; `v1` is server-gated): pin each format's stored shape and the acceptance list on
     fixture rows.
   - Optional, not gating: count `publication_id=` requests in the nginx log by shape (unpadded, 6, 8, OJ S, UUID),
     336's rule, to size the effect.
2. **Docs (S).**
   - Replace the `/docs` example (`docs.rs:266`) with a number whose notice is stable. Not `123456-2026`, and not
     `00123456-2026` (SCB's real notice; 481, `operations.md:891`).
   - In `/docs` (`docs.rs:271`) and the OpenAPI `publication_id` description (`openapi.json:803`), state the accepted
     forms: TED unpadded (as TED prints it), 6- or 8-digit, OJ S; the FTS and DÖE shapes; and that FTS 2021+ numbers
     coincide with TED 6-digit ones, so pair with `source=`.
   - Asserting a non-empty answer for the lookups table needs a fixture row. That widens 391's guard past its
     documented exclusion; name it as a scope change if done. Gate the served sentence (the 370 class).

## Unit 1 — built (2026-10-14, uncommitted)

### Precondition read: planned, then RUN (by the owner, 2026-10-14 ~20:10 UTC)

Both directions, 2023 (R2.0.9 and eForms coexist), one bounded index range each, one index
seek per row, `+source` so `notices_source_id` cannot drive. Planned locally first
(`target/debug/plan-probe plan` on a `Db::open` scratch file plus the deferred
`notices_publication_id_id` / `tender_versions_publication` indexes, no statistics):

    -- six_to_eight: 6-digit TED 2023 numbers 600000..600399, each one's 8-digit spelling
    SELECT COUNT(*) AS six_rows, COUNT(eight) AS pairs, MIN(pid) AS first, MAX(pid) AS last,
           COUNT(DISTINCT profile) AS profiles, group_concat(DISTINCT profile) AS which
      FROM (SELECT a.publication_id AS pid, a.profile AS profile,
                   (SELECT b.id FROM notices b WHERE b.publication_id = '00' || a.publication_id
                       AND +b.source = 'ted') AS eight
              FROM notices a
             WHERE a.publication_id >= '600000-2023' AND a.publication_id < '600400'
               AND a.publication_id LIKE '______-2023' AND +a.source = 'ted' LIMIT 400)
    SCAN (subquery-0)
      SEARCH a USING INDEX notices_publication_id_id (publication_id>=? AND publication_id<?)
      CORRELATED SCALAR SUBQUERY 1
        SEARCH b USING INDEX notices_publication_id_id (publication_id=?)

    -- eight_to_six: the reverse window, 8-digit TED 2023 numbers 00600000..00600399
    SELECT COUNT(*) AS eight_rows, COUNT(six) AS pairs, MIN(pid) AS first, MAX(pid) AS last,
           group_concat(DISTINCT profile) AS which
      FROM (SELECT a.publication_id AS pid, a.profile AS profile,
                   (SELECT b.id FROM notices b WHERE b.publication_id = substr(a.publication_id, 3)
                       AND +b.source = 'ted') AS six
              FROM notices a
             WHERE a.publication_id >= '00600000-2023' AND a.publication_id < '00600400'
               AND a.publication_id LIKE '________-2023' AND +a.source = 'ted' LIMIT 400)
    (same plan shape: range SEARCH on notices_publication_id_id, one (publication_id=?) seek per row)

Bound: the range is closed on both ends (400 numbers x the years stored at them, ~6,400 index
entries at most (2011–2026)) and `LIMIT 400` caps the per-row seeks. Expect `pairs = 0` on both; both
`*_rows` > 0 is what makes the read mean anything (the sample must hold both formats — if one
side is 0, move the window, one more query). **Not run**: piping the first query to
`ssh root@zebreus.click /root/sq.sh` was refused by this session's permission classifier
("Production Reads") and was not retried or worked around. The design does not hinge on the
answer — a pair would make `83685-2026` answer both TED rows, which are the same number per
TED's own API — but the read is the issue's stated precondition.

**Results** (run by the owner on prod, 2026-10-14 ~20:10 UTC: bounded index-range reads with
`LIMIT 400`, exactly the two planned queries):

| direction | window | rows | pairs |
| --- | --- | --- | --- |
| six_to_eight | `'600000-2023'`..`'600400'`, TED | 400 six-digit rows | 0 with an 8-digit twin |
| eight_to_six | `'00600000-2023'`..`'00600400'` | 0 eight-digit rows (eForms 2023 numbers start higher) | — |
| eight_to_six | `'00700000-2023'`..`'00700400'` | 86 eight-digit rows | 0 |
| eight_to_six | `'00750000-2023'`..`'00750400'` | 199 eight-digit rows | 0 |

Conclusion: no TED number is held at two widths in the sampled windows; the served claim "a
TED number is one notice per year whichever width you send" holds. (The `'00600000'` window
was the planned one; it held no 8-digit rows, so the window moved, as planned, to two higher
ones.)

### What was built

- **One grammar** (`crates/ingest/src/project.rs`): `ojs_key`'s body became `pub fn ojs_ref(raw)
  -> Option<OjsRef { year, number, issue }>`; `ojs_key` is now `ojs_ref(raw).map(|r| (r.year,
  r.number))`, behaviour unchanged. The display form also yields its OJ S issue. The app calls
  `ingest::project::ojs_ref`; there is no second parser.
- **API boundary** (`crates/app/src/v1/mod.rs`, `publication_lookup`, beside `kind`/`name_prefix`
  in `Params::filter`): trim; an OJS number → `ted = [{n}-{y}, {n:06}-{y}, {n:08}-{y}]`; OJ S
  with issue `000` → `fts = [{n:06}-{y}]` and no TED width; anything `ojs_ref` refuses (DÖE
  `<uuid>-<NN>`, bare uuid, `00000000-1900`, free text) stays exact.
- **Store type** (`crates/store/src/read.rs`, `PublicationLookup { exact, ted, fts }`;
  `Filter.publication_id: Option<PublicationLookup>`, `From<&str>` keeps the exact-only form):
  `spellings()` (dedup, ≤ 4) is the ONLY SQL predicate on the number — `= ?` for one spelling
  (the old cursor-riding seek), `IN (?, …)` otherwise (turso's InSeek, one equality seek per
  value on the index's leading column). `admits(source, stored)`: `exact` answers any source,
  `ted` only `source = 'ted'`, `fts` only `source = 'fts'`. 217-A holds: companions are still
  post-filtered in Rust, and the tenders seed still takes precedence in `tender_from`.
- **TED scope, notices**: `read::notices` retains `admits(r.source, r.publication_id)` before
  the source/kind companions.
- **TED scope, tenders**: `retain_publication_scope` runs after `retain_publication_companions`
  in `tenders`, `tenders_ordered` and `tenders_page` (so the SSE probe too). Tenders DO mix sources
  since 481 (a DÖE notice linked to its TED twin), so `t.source` cannot decide; it reads
  `publication_hits` — `SELECT v.tender_id, v.publication_id, (SELECT n.source FROM notices n
  WHERE n.id = v.caused_by_notice_id) FROM tender_versions v WHERE v.publication_id IN (…)`: the
  same ≤ 4 seeks on `tender_versions_publication` plus one PK seek per matched version — and keeps
  a tender when one of its matched VERSIONS is admitted. Skipped when nothing can change (no rows,
  or every spelling is `exact`).
- **`resolved_filters.publication_id = {"asked": <raw value sent>, "matched": [stored spellings]}`**
  on `/v1/notices` and `/v1/tenders` (id-ordered and sorted), present only when the page matched
  a spelling other than the one sent. Notices read the rows' own spellings; tenders re-read
  `publication_hits` (a row's `publication_id` is its CURRENT version's), kept to the page's ids.

### Decisions

- **OJ S issue `000` → FTS's 6-digit spelling, FTS rows only, no TED width.** FTS stores and its
  API serves `NNNNNN-YYYY` only (the fixtures under `crates/ingest/tests/fixtures/fts/members/`
  and `fts::release_id`: the release id verbatim; no OJ S form anywhere in them). That its notice
  PAGES print `YYYY/S 000-NNNNNN` is the service's known display form, still unmeasured here
  (find-tender 403s through the proxy). TED's OJ S issues start at 001, so issue `000` has no TED
  meaning; routing it to FTS costs nothing if FTS never prints it, and a TED answer there could only
  be a different notice. FTS-only rather than any-source because the fixture's first cut admitted
  the TED R2.0.9 notice stored as the same `012345-2023` (caught by the api test).
- **TED-ness on `/v1/tenders` is the matched version's causing notice**, not `t.source`: 481's
  link ledger merges DÖE and TED notices into one tender, so "a tender never mixes sources" is false.
- **`asked` is the raw value**, so a trimmed space is reported (`" 079997-2017"` →
  `["079997-2017"]`).

### Tests (`--features tender-db/server`, gate flags, gate package set)

- `ingest project::tests::ojs_ref_reads_every_ted_width_and_the_display_form_as_one_number`
- `tender-db v1::publication_lookup_tests::a_ted_number_expands_to_every_stored_width_and_nothing_else_does`
- `store read::publication_lookup_tests::a_ted_width_answers_a_ted_row_and_the_value_sent_answers_any`
- `store read::publication_lookup_tests::the_lookup_seeks_each_spelling_and_scans_no_table` — for a
  4-spelling TED lookup, FTS's issue-000 lookup and an exact DÖE value: the notices page (first and
  cursor), the SSE one-row probe, the tenders page, `Scope::At` and all four ordered windows, and
  `publication_hits`: ≤ 4 bound spellings, a seek on `notices_publication_id_id (publication_id=?` /
  `tender_versions_publication (publication_id=?` present, no table scanned (`hits`/`w` are the
  materialised seed/window), the causing notice by PK; and the check fails on `+publication_id IN`.
  Plans read (no stats): notices `SEARCH notices USING INDEX notices_publication_id_id
  (publication_id=?)` + `USE SORTER FOR ORDER BY` (one spelling: `(publication_id=? AND id>?)`, as
  before); tenders `SCAN hits` → `SEARCH tender_versions USING INDEX tender_versions_publication
  (publication_id=?)` → `SEARCH t USING INTEGER PRIMARY KEY`; hits `SEARCH v USING INDEX
  tender_versions_publication (publication_id=?)` + `SEARCH n USING INTEGER PRIMARY KEY (rowid=?)`.
  The 457 head-pointer matrix gained a `publication_id-ted` (4-spelling) row; still green.
- `tests/api.rs a_publication_number_answers_every_ted_form_and_its_widths_stay_on_ted` — fixture
  rows TED eForms `00083685-2026`, FTS `083685-2026`, TED `079997-2017`, text `123-2005`, R2.0.9 and
  FTS `012345-2023`, DÖE `<uuid>-01` and `00000000-1900`; pins the acceptance list on notices
  (`83685-2026` → TED only; `083685-2026` → both; `00083685-2026` → TED; `79997-2017`,
  `2017/S 044-079997`, ` 079997-2017`, `00079997-2017` → the 6-digit row; `000123-2005` → the
  text row; `12345-2023` / `2023/S 012-012345` → R2.0.9 only; `2026/S 000-083685` /
  `2023/S 000-012345` → FTS only; DÖE exact, bare uuid `[]`), `source=` narrowing, the
  `resolved_filters` shape, and on tenders (`83685-2026` → the TED tender only, `083685-2026` →
  both, `79997-2017` / OJ S / spaced → the 6-digit tender, `2026/S 000-083685` → the FTS tender).
- Still green: `notices_can_be_looked_up_by_publication_id`, `tenders_can_be_looked_up_by_publication_id`,
  `store tests/tender_publication_lookup.rs`, `tests/isolation_routing.rs`,
  `filter_classification_is_exhaustive`, `honoured_params_match_the_emitted_sql`.

Aside, not this issue: every tender page plan read here (no stats) shows the 479 procedure-type
select-list subquery as `SEARCH c USING INDEX tender_version_classifications_code (scheme=?)`, a
seek on `scheme = 'procedure'` alone rather than on `tender_version_classifications_version`. It
predates this change (the select list is untouched); worth one look against prod's stats.

## Unit 2 — built (2026-10-14, uncommitted)

- `/docs` lookups table: `GET /v1/notices?publication_id=79997-2017` — TED's own spelling of
  notice 19163916 (R2.0.9, stored `079997-2017`, a closed era; tender 5792500). Checked by public
  GET on 2026-10-14: `?publication_id=079997-2017` → `[19163916, ted, 079997-2017, ted-export-r209]`,
  `/v1/tenders?publication_id=079997-2017` → `[5792500]`; `79997-2017` → `[]` until unit 1 deploys.
  Not `123456-2026`, not `00123456-2026` (SCB, 481). `/docs | grep -c 'publication_id=123456-2026'`
  read `1` on prod before the deploy.
- The `/docs` `publication_id` bullet and the OpenAPI parameter state the accepted forms (TED
  unpadded, 6/8-digit, OJ S, whitespace), that width follows format not year, that the widths
  answer TED only, the FTS (`083685-2026`, issue-000 OJ S) and DÖE (`<uuid>-<NN>`, suffix
  included) shapes, the FTS/TED 6-digit coincidence with `source=`, and `resolved_filters`. The
  `/docs` envelope paragraph and the OpenAPI `resolved_filters` schema (new `matched` property)
  name the new entry.
- Gated: `no_served_surface_repeats_a_retired_claim` retires "exact match on the number the
  source printed" and `publication_id=123456-2026` on all three served surfaces;
  `the_docs_say_which_spellings_a_publication_number_answers` pins one phrase per sentence on the
  bullet and the parameter, and the table's `79997-2017` example. 391's guard is NOT widened (still
  non-4xx only); the non-empty answer is pinned on fixture rows by unit 1's api test instead.

## Review round (2026-10-14, uncommitted)

Three reviewers (lookup correctness, query cost, served contract) read the built diff. Every
finding was applied; nothing was waived.

1. **A filtered lookup lost its cursor (medium; prod-confirmed).** The number is the only SQL
   predicate (217-A), so the TED scope and the `source=`/`kind=`/date companions filter in Rust
   AFTER the `LIMIT` probe, and the app read "another page?" off the filtered count. A full
   probe that lost a row answered `next_cursor: null` with rows still past it: prod's
   `033562-2023` is TED 22890368 plus 15 FTS notices, so `2023/S 000-033562&limit=5` (TED row
   dropped) stranded ten, and `033562-2023&source=fts&limit=5` already did before 536.
   - **Mechanism: issue 408 (b)'s examined cursor.** `read::shrunk_probe` — when the SQL came
     back full (`limit` rows) and the Rust half dropped any, the page hands back the last row the
     SQL EXAMINED. `Banded<T, C = i64>` gained a cursor type (`(key, id)` for the ordered list)
     and carries it from `read::notices_page` (new; the REST list's notice page),
     `read::tenders_page` and `read::tenders_ordered_page` (new; `tenders_ordered` is now its
     `.rows`). `read_page` already falls back to `examined_to`; the sorted handler does the same
     with `(key, id)` (`HeadOrder::key_of`, shared with the page's own cursor). One bounded read
     per page, as before; a page may be short or empty while `more` is true (the documented 408 (b)
     contract), and following the cursor reaches every row once. The SSE snapshot pages through
     `read_page`, so it inherits the fix. Rejected: re-seeking until the page fills — unbounded
     on a value many rows share (`00000000-1900&source=ted` would read every placeholder row).
   - The false "never mis-paginate" / "can only under-fill" comments are corrected
     (`read::notices_read`, `retain_publication_companions`, `retain_publication_scope`); `/docs`'
     pagination paragraph and the OpenAPI envelope + parameter say a `publication_id` page can
     be short with `more` true, its cursor the last row examined.
2. **`2017/S 044` (an OJ S issue cited alone) answered notice 44 of 2017.** `ojs_ref` now reports
   `issue_only` (a display form with no `-` in its tail); the fold's reading is unchanged (it
   still reads the figure as the number — `ojs_key` maps `(year, number)` exactly as before,
   pinned for `2017/S 044`, `2026/S 083`, `2026/s 83 `), and `v1::publication_lookup` refuses
   an `issue_only` reference, so it stays exact (`[]`). Lookup side only: the fold's inputs were
   not re-audited, so its grammar did not move.
3. **Docs.** The `/docs` filter-table row still said "exact match"; rewritten ("A TED number is
   found in any of its spellings … on TED notices only; another source's number (FTS, DÖE)
   matches as that source stores it"), pinned, and "a TED OJS number) — exact match", the old
   OpenAPI "Exact-match an official publication number" and "at most four seeks" are RETIRED.
   The DÖE shape is `<uuid or number>-<version>` (prod notice 478241 is `17846986-1`) on both
   surfaces and in the code comments. The OpenAPI cost clause says "up to four spellings of the
   number, each found by an index seek". The `Params` doc comment no longer says exact-match.
4. **`/v1/tenders` read the hits twice.** `retain_publication_scope` now returns the admitted
   hits it read; they ride `Banded::publication_hits` → `PageOut::publication_hits` (and the
   sorted page), and `publication_resolution` is a pure function over them — a request is the
   seed plus ONE hits read, on one reader (was: seed, hits, a second pool acquisition and the
   hits again). The scope skips the read only when the lookup is one source-blind spelling
   (nothing to decide; every row matched at it), which is also what `resolved_filters` then names.
5. **Tests.**
   - `tests/api.rs a_publication_number_answers_every_ted_form_and_its_widths_stay_on_ted` gained:
     the sorted list (`sort=published_at` both orders, and `published_after`, which implies it)
     for `83685-2026`, `79997-2017`, the OJ S form, `083685-2026`, `00083685-2026` and
     `2026/S 000-083685`, with `resolved_filters`; a tender of two Sources (a TED
     `00099991-2025` and an FTS `054321-2025` notice under one procedure key, `source: ted`):
     `54321-2025` answers `[]` — deciding by `t.source` would return it — `054321-2025` and
     `99991-2025` answer it; `2017/S 044` → `[]` beside `44-2017` / `2017/S 001-000044` → notice 44.
   - New `tests/api.rs a_publication_page_that_drops_a_row_still_reaches_every_row`: the
     `033562-2023` shape (one TED notice, the lowest id, then four FTS, each its own tender),
     page sizes 1–3, walking `next_cursor` to the end: notices under `2023/S 000-033562`,
     `&source=fts`, `&source=ted` and bare; tenders by id and by `published_at` (asc puts the
     dropped TED tender first) — every row exactly once, in order, and the first short page
     says `more: true`.
   - `ingest ojs_ref_reads_every_ted_width_and_the_display_form_as_one_number` and
     `tender-db v1::publication_lookup_tests` pin the issue-only refusal (and a numeric DÖE id).
   - Both api tests send more than the limiter's 50-request burst, so the harness gained
     `Server::get_paced` (a `429` waits out `Retry-After`, ≥ 100 ms, then asks again).
   - **Each new assertion was shown to bite**, by mutating the store and re-running: no examined
     cursor anywhere → the pagination test fails on the first notice walk (`[2]` of four FTS
     rows); TED-ness by `t.source` (the hits subquery reading `tenders.source`) → the main test
     fails at `/v1/tenders?publication_id=54321-2025` (`[4]`, the mixed tender); no TED scope
     on the sorted list → it fails at `83685-2026&sort=published_at` (`[1, 3]`); no examined
     cursor on the sorted list only → the pagination test fails at
     `2023/S 000-033562&sort=published_at&order=asc&limit=1` (`[2]` of `[2, 3, 4, 5]`). Source
     restored after each.
6. **Precondition read** recorded above (0 pairs).

Test runs (gate flags and package set, `--features tender-db/server`; no `ops/check.sh` yet):
`--lib --test api` unfiltered — ingest 541 passed (4 ignored), model 3, store 185, tender-db
219, api 81, GATE-EXIT=0; store `tender_publication_lookup` 2, `published_order` 3,
`fallback_band` 1, `seeded_lots_page` 6, `isolation_routing` 11, `paginated_index_probe` 0 (4
ignored), `head_pointer_equivalence` and `ordered_source_plan` 1 each — all ok, GATE-EXIT=0.

## Re-check round 2 (2026-10-14 ~21:30 UTC)

Re-check of the fix round: no blocking defects. Every fix bites under mutation (`536r/mutA`–`mutC`). Three low points
are fixed in place:
- the stale `sse.rs` main-pool comment (a `publication_id` page can be short or empty there);
- the OpenAPI parameter's pagination clause: the TED scope also shrinks a page, and the other tender filters are SQL,
  not post-read;
- the DÖE shape is now pinned (`uuid or number`).

Left as noted, not fixed:
- **No test for "the hits are read once".** A second read would cost efficiency only, not correctness.
- **An inherited, theoretical risk in `HeadOrder::key_of`.** It maps a NULL list-row deadline to 0. If
  `t.current_deadline` drifts from the list row's own deadline pick, an examined cursor on `sort=deadline` could land
  on `d0.<id>`. 0 NULL deadlines were seen in 600 rows. The robust fix carries the window's `w.wkey` on the row. File
  it only if it is observed.
