# 522 — the UK's shared withheld-supplier PPON binds unrelated suppliers into one org

Status: ready-for-agent — filed 2026-10-12 from issue 511 unit 4's item 5 read.
Kind: data quality (organization identity)
Relates to:
- 511 (placeholder names; unit 4 drops the s.94 sentences);
- 452 (identifier verdicts: `wrong`, the per-identifier withhold);
- 466 (related numbers withheld from grouping).

## Verify

    echo "SELECT count(*) FROM organization_mentions WHERE organization_id = 30914328" | ssh root@zebreus.click /root/sq.sh

- **done**: the count drops to the placeholder mentions only (or the org is gone), and HISCOX / DOC-Labor mentions bind
  their own orgs;
- **open**: `9` as on 2026-10-12, with real suppliers among them.

## What was found

FTS publishers put one PPON, `GB-PPON-PBXY-3232-WNHZ`, on every supplier whose identity a notice withholds under
s.94 of the Procurement Act 2023. Org 30914328 ("Withheld Section94 Supplier") holds it. Its 9 mentions on
2026-10-12, all under section `ORG-GB-PPON-PBXY-3232-WNHZ`:
- "Information withheld due to security" (2);
- "This information has been omitted from this notice in accordance with s.94 Procurement Act 2023" (3);
- "Individual" (2);
- **"HISCOX INSURANCE COMPANY LIMITED"** (notice 46800936);
- **"DOC-Labor GmbH"** (notice 46818757).

Unit 1 of issue 511 already noted the shape (decision line 188: one PPON covers different withheld suppliers on all
123 mentions). The PPON identifies the placeholder, not a supplier. Every party that carries it binds the one org
by identifier, so two real companies are filed under "Withheld Section94 Supplier", and the org gathers
whatever names come next.

Since 511 unit 4, the placeholder sentences drop as `Placeholder`. The real names still bind the PPON org.

## Proposed unit

1. Record a 452 identifier verdict, `wrong`, on (`GBPPONPBXY3232WNHZ`, national, GB) with the rationale above.
   - Per 452, the number then leaves the R2/E0/R3 groups and the resolver's canonical bind.
   - A mention binds the owner only when its names match the owner's.
   - "HISCOX INSURANCE COMPANY LIMITED" would then bind by name, or mint.
2. Re-fold the 9 notices (`refold-notices`). Then read where the real names landed, and that 30914328 keeps only
   placeholder mentions or is swept.
3. Measure first: whether other shared-placeholder PPONs exist. These are PPONs whose holders' mention names include
   both a placeholder sentence and a legal-form company name. Run it as an in-app read or over 511's `identified`
   lists, never a corpus scan from the agent.
