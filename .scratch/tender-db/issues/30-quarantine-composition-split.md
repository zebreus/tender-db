# 30 — Quarantine headline is mostly benign: split it, then triage the real gaps

Status: ready-for-agent

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
