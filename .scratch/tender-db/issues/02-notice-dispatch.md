# 02 — Notice extraction, profile dispatch, quarantine skeleton

Status: resolved
Blocked by: 01

Goal: packages from the archive are opened, each notice file is identified
and dispatched to a mapping profile, and Notice identity rows exist — with
quarantine as the only failure mode.

Scope:
- `store`: `notices` table (source, publication_id, content_hash, profile,
  declared_version, package ref, file path/offset, ingested_at) with the
  identity key (source, publication_id, content_hash); `quarantine` table
  (notice ref/raw ref, profile, reason, detail, first_seen, reprocessed_at).
- `ingest`: package walker (tar.gz for TED) + per-file dispatch on root
  element + CustomizationID → profile id (text / ted-export-r208 /
  ted-export-r209 / eforms:<customization>); unknown → quarantine.
- Publication-id extraction per era (OJS number; eForms BT-701+publication
  number). No field mapping yet.
- `process` CLI: walk archive → notices + quarantine; idempotent re-runs.

Acceptance: processing the fetched day from issue 01 yields one notice row
per file with correct profile split (compare counts against
docs/research/ted-access-channels.md era findings), zero silent drops
(files == notices + quarantined).

## Answer

Delivered: `store` gained STRICT `notices` + `quarantine` tables and their
accessors; `ingest` gained `package` (walker), `profile` (dispatch), `process`
(driver) and a `process` CLI. Verified on the VPS against nine real daily
packages spanning 1993–2026: **17,408 members → 18,347 notices, 0 quarantined,
0 silent drops** (45 members skipped by the text-era language/variant policy).
Re-running the eight-package set wrote 0 new rows (17,560 duplicates), and the
`notices` table held exactly 17,560 distinct identities.

Profile split (the eight-package run, cross-checked against an independent
Python scan of the same tarballs — exact match):

| profile | notices |
|---|---|
| `text` | 199 |
| `ted-export-r208` | 3157 |
| `ted-export-r209` | 5706 |
| `eforms:eforms-sdk-1.3` … `1.14` | 8498 |

Four findings that changed the design, all confirmed against real files:

1. **The text era is not one-file-per-notice.** A 1993–2010 daily tarball holds
   one ZIP *per language and encoding variant*, each containing a single
   document that concatenates the whole day's notices (1993-001: 199 notices in
   one file; 2010-001: 787). The walker therefore unwraps one level of ZIP
   nesting, and text members are split on their `1.00/067192` record markers so
   a Notice stays one publication event. Identity comes from the record's `ND:`
   line.
2. **Text-era language/variant selection needs a third disposition.** A 2010
   daily has 46 members (23 languages × `utf8`/`meta`). Per CONTEXT.md the text
   era is English-only for now, and `_meta_` is a parallel XML rendering of the
   very same notices — ingesting it would double every notice. These are
   `Skipped` with a documented reason, counted and printed, rather than
   quarantined: quarantine is the data-quality metric (ADR-0004) and must not
   absorb deliberate policy. The invariant is therefore
   `members == ingested + skipped`, asserted by the CLI and the tests.
3. **Dispatch must match namespace-URI + local name, never the prefix.** A
   single 2026 daily contains `ContractNotice`, `urn:ContractNotice`,
   `ns8:ContractNotice` and `cn:ContractNotice` for the same root.
4. **BusinessRegistrationInformationNotice is rooted in the eForms `p27`
   namespace, not UBL.** The first real-data run quarantined exactly 2 of 17,362
   files on this; since CONTEXT.md puts BRIN in scope, the eForms root test
   accepts both namespace families. Regression-tested with a BRIN fixture.

Two smaller points worth carrying forward: `DOC_ID` (`000036-2011`) is used as
the legacy publication id rather than `CODED_DATA_SECTION/NO_DOC_OJS`
(`2011/S 1-000036`) because it shares its shape with the eForms publication
number — both forms are present on 100% of files, so the OJS form stays
available for chain-linking in a later issue. And 177 of 1817 files in the
2011-001 daily carry no version marker anywhere (no root `VERSION`, none in the
namespace or `schemaLocation`); they fall back to the `ted-export-r208` profile
with a NULL `declared_version`, per docs/architecture.md's note that r208 also
covers R2.0.7.
