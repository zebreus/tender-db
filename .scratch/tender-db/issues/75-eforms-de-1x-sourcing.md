# 75 — eForms-DE 1.0/1.1/1.2 sourcing spike (~219K quarantined, German)

Status: open (feasibility/sourcing spike — find or reconstruct the inventory, then decide)
Kind: completeness / data-quality
Blocked by: —
Relates to: 71 (parent), 12 (DÖE eforms-de-2.x + sdk-0.1 vendoring), ADR-0002, ADR-0004, CONTEXT.md (~40% German volume)
Owner: (assign)

## Finding (issue 71 spike, 2026-07-29)

Of the ~1.2M `unknown-customization` quarantines, three are the German national dialect at its
1.x line — **eforms-de-1.1 = 145,859**, **eforms-de-1.2 = 72,986**, **eforms-de-1.0 = 31**
(~219K total, almost all real German notices). These were left out of issue 71's EU-SDK cohorts
because they are **not sourced the same way**:

- The EU versions come from `OP-TED/eForms-SDK` on GitHub (`fields/fields.json` per tag). eForms-DE
  is a *fork*: SDK-DE lives at **gitlab.opencode.de** (`OC000008125155/SDK-eforms-de`), which is how
  issue 12 vendored eforms-de **2.0** (tag 1.12.6) and **2.1** (1.13.3 / 1.14.4) — see the family
  notes in `sdk.rs:23-39`.
- `eforms.rs:256` records the current assumption: *"eForms-DE 1.x has no SDK-DE artifact and stays
  out."* That needs verifying, not assuming — 219K German notices is a large slice.

## The spike (source, then decide — do NOT vendor yet)

1. **Does an SDK-DE `fields.json` exist for the 1.x line at all?** Check gitlab.opencode.de
   `OC000008125155/SDK-eforms-de` tags/branches for the versions whose national line predates
   2.0 — the EU bases these 1.x DE notices declare (likely EU SDK 1.6–1.11, cross-referenced with
   their `cbc:ProfileID`). If a `fields.json` exists per version, vendoring is the issue-71
   mechanical path (fetch → `ACCEPTED` → completeness + index-build tests → the DE→EU `resolve`
   map if a ProfileID split is needed like 2.1's).

2. **If no artifact exists** (the `eforms.rs:256` assumption holds): reconstruct the inventory
   **empirically**, exactly as `sdk-0.1` was built (`sdk.rs:33-39`) — walk every element path
   observed across the full eforms-de-1.0/1.1/1.2 sample history from the archive, commit the
   observed-path checklist in `fields.json` shape, and let ADR-0004 quarantine anything outside it.
   This is more work than a fetch but is a proven pattern in this codebase.

3. **Watch for the grammar blocker (issue 74).** SDK-DE 1.x forks EU bases (1.6–1.11) that
   themselves straddle the descendant-axis+`or` `CompanySizeCode` line. If a DE 1.x inventory
   carries that predicate, it depends on issue 74; if its EU base is ≥1.8 it does not. Determine
   the EU base per DE minor first.

## Deliverable

A short finding in this issue: (a) does an SDK-DE 1.x `fields.json` exist and where; (b) the EU
base per DE minor (→ whether issue 74 blocks it); (c) recommended path (vendor vs empirical
inventory) with effort estimate. Then a follow-up implementation issue, not this one.

## Validation (once implemented, later)

Per issue 71's pattern: a fixture DE 1.x notice parses with expected BTs incl. the OPT-002
ProfileID national delta; `unknown-customization` drops by ~the DE 1.x volume after ops reprocess.
