# 35 — Text profile: the `OC`/`ON` fields are unmapped (~577k real notices)

Status: ready-for-agent

Split out of issue 30's quarantine triage (2026-07-21). The
`unknown-field-code` bucket is 576,753 members and — sampled via `/v1/sql` on
the prod `quarantine` table — **~entirely the single legacy field code `OC`**
(at various `line N:` within the record). It is a real parser gap, not benign:

- Extracted a byte-exact example from the archive
  (`19950201_1995021.tar.gz/EN_19950201_1995021_ISO_ORG.zip!…#0`): a real notice
  (Swedish leasing tender, `ND: 4149-1995`) whose header carries
  `OC: 71101000` then `ON: <description>` — fields the text inventory
  (`crates/ingest/sdk/text-inventory.json` + `crates/ingest/src/text/rules.rs`)
  has no rule for, so the whole record quarantines.
- **All 576,753 are `EN_` files** (the primary parsed language, not non-EN
  siblings), across 1995–1998, all ISO-only packages (zero UTF8 twins in the
  archive), all outstanding (`reprocessed_at IS NULL`). So these are the sole
  representation of these notices — no already-parsed twin. They are lost.

Fix: add `OC` (and its paired `ON`) to the text-era inventory + `rules.rs` with
the right rule — `OC` looks like a classification code (`71101000` is
CPV-shaped), `ON` its description; confirm against a few more records and the
`ted-legacy-mapping.md` field table. Byte-exact fixture + regression test per
the issue-31 pattern. After deploy, reprocess the held `unknown-field-code`
bucket (`detail LIKE '%: OC'`).

Open question (carry, do not hand-wave): this collides with the quoted 96.7%
ted·text coverage (3.78M/3.91M ⇒ only ~129k lost), yet `OC` alone is 577k. The
counts are a mid-backfill snapshot (issue-15 re-walk) and the canonical
projection is far behind (v_tenders ~7k tenders), so neither the coverage % nor
the quarantine total is settled. Re-measure after the backfill settles before
sizing the true coverage impact; the fix is warranted regardless (a real field
on real notices must be claimed, ADR-0004).

Acceptance: `OC`/`ON` claimed into the model; the fixture parses; the bucket no
longer produced on those inputs; held entries reprocessed post-deploy.
