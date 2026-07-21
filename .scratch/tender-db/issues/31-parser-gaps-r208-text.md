# 31 — Two parser gaps quarantining real notices (r208 awards, text RP)

Status: needs-verification

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

## Fix (2026-07-21)

Both diagnosed from prod (byte-exact fixtures pulled via /v1/sql on the
quarantine table for the member paths, then extracted from the VPS archive
at /data/archive).

1. **r208 award `@REASON`** (`ted-export-r208`). The ANNEX_D justification for
   a negotiated procedure without competition publishes the reason as an
   attribute on the choice element:
   `<PURCHASE_SUPPLIES_ADVANTAGEOUS_TERMS REASON="SUPPLIER_WINDING_UP_BUSINESS"/>`.
   `@REASON` was in no rule's consumed set and not a captured attribute, so it
   quarantined the whole award. Added `REASON` to `CAPTURED_ATTRIBUTES`
   (r209/parse.rs) — exactly how the sibling annex-D qualifier `@PROCEDURE` is
   handled — so it is claimed as the code row
   `TED-PURCHASE_SUPPLIES_ADVANTAGEOUS_TERMS.REASON = SUPPLIER_WINDING_UP_BUSINESS`
   on the PROCEDURE section. One generic capture covers the CONTRACT_AWARD,
   VEAT and utilities variants (all use the same annex-D choice grammar).
   Fixture: tests/fixtures/r208/f03-annexd-neg-022211-2011.xml (2011 daily).

2. **Text `RP` continuation** (`text`). `RP` (authority/regulation code) was
   `Scalar`, but code `2` (international financing) publishes the lead
   institution plus one continuation line per co-financier
   (`European Bank for Reconstruction and Development`, …) — a continuation
   under a scalar field, which quarantined the record. Changed `RP` to
   `PerLine(Type::Code)` (like `MA`): the lead is the typed code, each
   co-financier a further value (a code-less line degrades to raw text per the
   era's value policy — never dropped). Single-code records (`4 - EEC`) are the
   one-line case of the same rule. Fixture:
   tests/fixtures/text/1993-rp-list-224-1993.txt (1993 ISO_ORG record).

Tests: r208 `f03_annex_d_negotiated_reason_is_claimed`, text
`rp_lists_every_co_financing_institution`; both fixture corpora's
exhaustive-consumption tests updated to include them; r209 completeness
(`every_inventory_attribute_is_claimed`) still green. Full ingest suite +
clippy green. Strictly ADR-0004 — content claimed into the model, no
skip-listing.

Post-deploy: the affected quarantine entries are held reprocessable; a
reprocess pass over the `unclaimed-content` bucket (detail LIKE the two
patterns) folds them in. Note for the post-backfill triage.
