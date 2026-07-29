# 75 — eForms-DE 1.0/1.1/1.2 sourcing spike (~219K quarantined, German)

Status: spiked — no SDK-DE artifact exists; empirical-inventory path required, NOT issue-74-blocked (see Findings)
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

## Findings (2026-07-29 spike, sdk-vendor)

**Verdict: no SDK-DE `fields.json` artifact exists for eForms-DE 1.0/1.1/1.2. The
`eforms.rs:256` assumption is CORRECT. The empirical-inventory fallback (sdk-0.1 pattern) is the
only viable path — and it is NOT blocked by issue 74.**

### 1. opencode.de SDK-eForms-DE (the only fork carrying a fields.json) starts at national 2.0
Project `OC000008125155/SDK-eforms-de` (GitLab id 418) is where 2.0/2.1 were vendored (issue 12).
Its entire tag + release history is **1.12.0 → 1.14.4** only (repo tags track the EU base; the
national version lives in `sdk/fields/fields.json` `sdkVersion` and `sdk/metadata.xml`):
- tag 1.12.0/1.12.6 → `customization_id eforms-de-2.0`, `version_sdk_eu 1.12.0`, dated 02.09.2024
- tag 1.13.3 / 1.14.4 → `eforms-de-2.1.0`

No tag, release, or `master`-branch history below 1.12 (earliest main commit 2024-07). So this repo
never held the 1.x national line — it begins at 2.0.

### 2. The eForms-DE 1.x line (KoSIT, projekte.kosit.org/eforms) ships NO field inventory
The 1.x generation lives on a *different* GitLab — `projekte.kosit.org/eforms/` — as four repos:
`eforms-de-specification` (id 172), `eforms-de-schematron`, `eforms-de-codelist`,
`validator-edition-eforms-de`. **None contains a `fields.json`.** Verified the spec repo at tag
`v1.1.0`: tree is `doc/`, `src/` (a SeMoX semantic model + DocBook + Schematron acceptance tests),
`eforms-de.properties`, `CHANGELOG.md` — recursive search for `fields`/`*.json` returns nothing.
eForms-DE 1.x is a *specification + business-rule* layer over the EU SDK, not an SDK fork with a
machine-readable field inventory. There is nothing to vendor.

### 3. EU base: eForms-DE 1.1 = EU SDK **1.7.0**
Spec changelog (`v1.1.0`): "EU Codelisten sind kompatibel mit TED SDK 1.7.0"; README: "Next
release is planned … after TED SDK 1.9". So DE-1.1 ≈ EU 1.7, DE-1.2 ≈ EU 1.8/1.9 era.

### 4. Issue-74 blocking: NO
DE-1.1's EU base (1.7) is itself in the issue-74 grammar-blocked cohort — BUT that only matters if
we vendored the *EU* `fields.json` for these notices, which we cannot (no SDK-DE fork, and the raw
EU 1.7 inventory carries the `//`+`or` `CompanySizeCode` predicate). The empirical inventory we
*would* build is **predicate-free** (a checklist of observed element/attribute paths, exactly like
sdk-0.1), so it never invokes the descendant-axis/`or` grammar. **Issue 75 proceeds independently
of issue 74.**

### 5. Recommended path + effort (implementation = a separate issue, sized like sdk-0.1 / issue 12)
Build the inventory empirically:
- Sample: all `eforms-de-1.0/1.1/1.2` payloads in the archive — ~218K notices (1.1=145K, 1.2=72K,
  1.0=31), a healthy sample comparable to sdk-0.1's ~250K.
- Enumerate distinct element/attribute leaf paths; emit a `fields.json`-shaped checklist with
  DE-prefixed field ids (cf. sdk-0.1's `SDK01-`), assign a `Decision` per path (ADR-0002 harness),
  fold into the index (predicate-free, so no grammar risk), quarantine anything outside it (ADR-0004).
- Wire `ACCEPTED` + `resolve()`. The DE-1.x `CustomizationID`s (`eforms-de-1.0/1.1/1.2`) map to
  their own keys; a ProfileID split like 2.1's is probably unnecessary — confirm during the build.
- Because 1.1 (EU 1.7) and 1.2 (EU 1.8/1.9) differ, decide one merged era-inventory vs one per
  minor (design call at build time; the DÖE OPT-002 ProfileID delta + DE codelists ride the normal
  channels as they do for 2.x).
- Effort: on the order of the sdk-0.1 build (issue 12) — one focused implementation issue.
