# 31 — Two parser gaps quarantining real notices (r208 awards, text RP)

Status: ready-for-agent

Found by issue 27's quarantine sampling (2026-07-21) — both quarantine
REAL notices, so they cost completeness (unlike the benign non-notice
buckets, issue 30):

1. **r208 award notices**: `unclaimed attribute at
   .../CONTRACT_AWARD/FD_CONTRACT_AWARD/PROCEDURE` (also VEAT and
   CONTRACT_AWARD_UTILITIES variants), recurring across 2011 dailies.
   The r208 profile (issue 10) doesn't claim an attribute the era
   really publishes.
2. **Text era**: `continuation under scalar field RP` on 2010 UTF8_ORG
   bundles — the text profile (issue 11) hits a continuation-line form
   it doesn't handle for the RP field.

Fix both strictly (ADR-0004: claim the content properly, no
skip-listing): pull real quarantined payloads from prod as byte-exact
fixtures (via /v1/sql on the quarantine table or recent_quarantine),
extend the profiles, regression tests per gap. After deploy, the
affected quarantine entries reprocess (they're held reprocessable —
note the reprocess step for the post-backfill triage).

Acceptance: both fixture classes parse to notices; quarantine reasons
no longer produced on those inputs; tests green.
