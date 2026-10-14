# 536 — `publication_id=` is an exact string match across three stored TED widths: TED's own spelling, the OJ S form and the `/docs` example find nothing, and a wrong-width pad can return an FTS notice

Status: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, the TED-scoped normaliser (S–M), starting with its bounded precondition read.
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
