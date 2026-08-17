# 35 — Text profile: the `OC`/`ON` fields are unmapped (~577k members, ~47k real loss)

Status: RESOLVED-VERIFIED (2026-08-17, owner — prod rev `62f0e19`). Acceptance met on every clause:
the `unknown-field-code` bucket is at ZERO outstanding (today's only non-zero quarantine buckets are
8 terminal unreadable-zips and 5,174 `unrepresentable-value`), and the 1995-98 coverage this issue
predicted would rise now reads **1.011 / 1.015 / 1.019 / 1.016** against the vendored ground truth —
inside the ±2% tolerance, all four years. (Slightly ABOVE 1.0, which the ground-truth CSV header
documents as expected: the upstream counts are approximate for the older eras.)

Split out of issue 30's quarantine triage (2026-07-21). The
`unknown-field-code` bucket is 576,753 members and — sampled via `/v1/sql` on
the prod `quarantine` table — **~entirely the single legacy field code `OC`**
(at various `line N:` within the record). `OC`/`ON` are real fields the text
inventory (`crates/ingest/sdk/text-inventory.json` +
`crates/ingest/src/text/rules.rs`) has no rule for, so the whole record
quarantines. Byte-exact example from the archive
(`19950201_1995021.tar.gz/EN_19950201_1995021_ISO_ORG.zip!…#0`): a real notice
(Swedish leasing tender, `ND: 4149-1995`) whose header carries `OC: 71101000`
then `ON: <description>`. All members are `EN_` files, 1995–1998, ISO-only, all
outstanding (`reprocessed_at IS NULL`).

**Real loss is bounded ≈47k, NOT 577k** (team lead, per-year coverage, 2026-07-21):
1995–98 held-vs-TED-ground-truth = **92.0 / 92.5 / 93.3 / 92.3 %**. If the 577k
`OC` members were lost notices these years would sit near ~10%; at ~92% they are
overwhelmingly **duplicate representations** of notices already held via another
EN member/file-class, deduped away. The genuine shortfall is the ~7–8%/yr —
roughly 47k notices across 1995–98. (The 96.7% era aggregate is corroborated by
these per-year numbers, not a re-walk artifact.)

Still goal-critical: 92–93% **fails the verify ±2% tolerance**, so the missing
~47k is a real completeness gap, and mapping `OC`/`ON` is the fix regardless (a
real field on real notices must be claimed, ADR-0004).

Fix: add `OC` (classification-code shaped — `71101000` is CPV-like) and its
paired `ON` (description) to the text-era inventory + `rules.rs` with the right
rule; confirm against a few more records and `ted-legacy-mapping.md`. Byte-exact
fixture + regression test per the issue-31 pattern. After deploy, reprocess the
held bucket (`reason='unknown-field-code'`, `detail LIKE '%: OC'`): dedup absorbs
the duplicate majority, the missing ~47k subset gets reclaimed.

Acceptance: `OC`/`ON` claimed into the model; the fixture parses; the bucket no
longer produced on those inputs; held entries reprocessed post-deploy; 1995–98
coverage rises toward the ±2% tolerance.
