# 30 — Quarantine headline is mostly benign: split it, then triage the real gaps

Status: needs-verification

The dashboard's headline data-quality metric (quarantine total) reads
1,212,695 mid-backfill (rev bad8dda), dominated by `unparsable-xml`
(628,204) and `unknown-field-code` (576,753) — both text-era-shaped. But
ted·text notice coverage is **96.7 %** (3,784,475 / 3,913,520) at the same
time. A million quarantined members coexisting with near-complete notice
coverage means the big buckets are overwhelmingly **non-notice /
duplicate-representation members** — per-language variants, ISO renderings
superseded by UTF8, `_meta_` XML siblings — that were never destined to
become their own Tenders, not lost notices. So the headline overstates the
problem.

Two pieces of work:

1. **Split the metric benign vs actionable.** A quarantined *non-notice
   member* (no publication id of its own, or a duplicate representation of
   an already-ingested notice) is expected and benign; a quarantined *real
   notice* is an actionable parser gap that costs coverage. The dashboard's
   headline should count only the latter (or show both, clearly labelled),
   so the number tracks real data loss. This touches app/coverage +
   dashboard — coordinate with that crate's owner.

2. **Triage the actionable remainder.** Classify the two big buckets by
   sampling — needs an API token (`/v1/sql` on the `quarantine` table) or
   the box, since the public surface exposes only 50 recent samples:
   `SELECT detail, COUNT(*) FROM quarantine WHERE reason='unknown-field-code'
   GROUP BY detail ORDER BY 2 DESC LIMIT 20` (and the same for
   `unparsable-xml` bucketed by member-path shape). If a few field codes
   drive the 577 k, that is a one-line mapping fix in the relevant profile.
   Two concrete gaps are already visible in the public recent-quarantine
   samples, both quarantining *real* notices: (i) legacy r2.0.8 award
   notices — `unclaimed attribute at
   /TED_EXPORT/FORM_SECTION/CONTRACT_AWARD/FD_CONTRACT_AWARD/PROCEDURE`
   (also `VOLUNTARY_EX_ANTE_TRANSPARENCY_NOTICE`,
   `CONTRACT_AWARD_UTILITIES`), recurring across 2011 dailies → an r2.0.8
   parser gap (issue 10) that drops 2011 award coverage; (ii) text-era —
   `continuation under scalar field RP` on 2010 `UTF8_ORG` bundles → a
   text-profile gap (issue 11).

Note: best done after the issue-15 backfill settles so the buckets reflect
the full archive, not a mid-re-walk snapshot.

Acceptance: quarantine headline reflects real notice loss (benign members
split out); the two big buckets classified with a field-code top-N; each
genuine parser gap either fixed in its profile or filed with its share.

## Fix (2026-07-21) — three-class split, evidence-based

Triage (sampled via /v1/sql on prod quarantine + byte-exact archive extracts)
inverted the "big buckets are benign duplicates" premise:
- `unknown-field-code` (577k) is ~entirely the one legacy field `OC`, on **EN**
  files (the primary parsed language, not non-EN siblings), 1995–1998, ISO-only,
  all outstanding. Extracted example is a real notice (ND 4149-1995). → real gap,
  filed **issue 35** (text OC/ON).
- `unparsable-xml` (628k) is ~entirely `XML with DTD detected` — a whole
  DTD-bearing XML era refused wholesale. → suspected real, filed **issue 36**.

So neither big bucket is benign. Per the lead's decision, the metric is split
**three ways**, nothing called benign without evidence:
- **Actionable** (headline) — a member identified as a notice whose content we
  could not represent: `unclaimed-content`, `unrepresentable-value` (~5k).
- **Suspected gap** (flagged distinctly, ~1.2M) — `unknown-field-code`,
  `unparsable-xml`, plus the small uncertain `not-utf8` / `unknown-customization`
  (real-notice-shaped, untriaged).
- **Benign** (only where the reason itself proves non-notice) — `unknown-root`,
  `missing-publication-id`, corrupt-zip.

`model::dashboard::quarantine_class` is the one classifier (shared by the
server measure and the wasm renderer); the dashboard shows the headline =
actionable, the suspected total flagged with its `OC` driver, the benign
remainder, and every reason labelled by class. Store adds
`quarantine_field_code_gaps` (group unknown-field-code by code, not by
`line N:` detail) — proves it is one code.

Open question carried into issues 35/36 (not hand-waved): the ~1.2M suspected vs
the quoted 96.7% ted·text coverage (~129k lost) does not reconcile mid-backfill
— counts are an issue-15 re-walk snapshot and the projection is far behind
(v_tenders ~7k). Re-measure after the backfill settles.

Tests: `model::quarantine_reasons_class_by_evidence`;
`store::field_code_gaps_group_by_code_across_line_numbers`. Full app suite +
clippy green.

Needs verification: dashboard renders the three-class split honestly in prod;
re-measure the buckets post-backfill to size issues 35/36.
