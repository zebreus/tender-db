# Upstream drift audit — 2026-08-09

The first periodic re-check the research phase mandated (SUMMARY.md item D8:
"watch upstream"), covering drift since the research baseline of 2026-07-19.
Method: four parallel web sweeps (EU eForms SDK; turso; eForms-DE/KoSIT/DÖE;
TED access channels) against the claims in the July research docs, each
finding then verified against our code and production state before being
acted on. Corrections were written into the affected docs in place
(ted-access-channels.md, eforms-de-profile.md — marked with this date);
this file is the audit record and the action register.

## Verdict in one paragraph

Three weeks of drift produced: one research **error corrected** (the TED
Search API floor is a rolling `today − 10y` window, not fixed July 2016), one
**assumption superseded by reality** (German DEX statistics fields went live
in the DÖE feed ~2026-08-06 — our importer already handles them), two
**deadline-driven work items** (eForms-DE successor before 2026-12-02, filed
as issue 165; EU SDK 1.16/2.0 landing autumn 2026), one **hygiene bump**
(turso 0.7.2, exposure to its corruption fix audited as none, filed as issue
166), and no changes to fetcher URLs, licences, cadence, or the DÖE API.
Nothing is broken; nothing was being silently dropped.

## 1. EU eForms SDK

- **1.15.1 released 2026-07-20** (one day after our baseline). fields.json
  diff vs 1.15.0: version string + two `businessEntities`
  `referencedBusinessEntityId` case fixes; fields/nodes/codelists otherwise
  byte-identical. **No action**: CustomizationIDs are minor-only, so notices
  declare `eforms-sdk-1.15` and resolve to our vendored 1.15.0 checklist;
  our deserializer does not even read `businessEntities`.
- **Roadmap changed**: 1.15 was expedited (eNotices2 fixes); **1.16 — not
  1.15 — is now the last 1.x**, released in sync with SDK 2.0 (beta target
  Aug 2026, final autumn 2026). SDK 2 keeps classic fields.json for its whole
  lifetime; the metadata format break is deferred to SDK 3 (`fields/fwd/`
  previews it). Softer than the July risk assessment feared.
- TED has **not yet activated 1.15** for submission (active: 1.12 until
  2026-10-31, 1.13/1.14 until 2027-12-31; 2026-08-07 package mix: 1.13
  dominant, then 1.14, 1.12; zero 1.15). We already vendor 1.15 — ahead of
  activation.
- Watch: vendor `eforms-sdk-1.16` when final (~Sep 2026) — folded into
  issue 165's watch duty.

## 2. turso (pinned `=0.7.0`)

- Releases: 0.7.1 (2026-07-22), 0.7.2 (2026-07-30), 0.8.0-pre.1–3. None of
  our pinned pain points changed: no `interrupt()` in the Rust crate surface
  (drop-the-future stays the only cancellation), VACUUM INTO OOM unfixed,
  write-future-drop poisoning is deliberate behavior, `foreign_keys` still
  defaults OFF. Recursive CTEs and window functions landed on the 0.8.0-pre
  line only.
- **0.7.1 fixes a real corruption bug** (#6858: `UPSERT … DO UPDATE` deletes
  secondary-index entries before validating; an aborting update arm leaves
  persistent table/index inconsistency). **Our exposure: none** — the
  codebase's only `ON CONFLICT … DO UPDATE` targets `layer_presence`
  (canonical.rs), whose schema is a TEXT primary key plus three non-unique
  columns and no secondary indexes; the failure shape cannot occur.
- Action: **issue 166** — bump to `=0.7.2` via the D1 reprobe protocol on a
  quiet day. Explicitly not 0.8.0-pre.

## 3. eForms-DE / DÖE

- **DEX went live** (~2026-08-06): first
  `defext:GermanEformsExtension/…/ReportingUnitId` in the 2026-08-07 DÖE
  export; SVS (Service Vergabestatistik) operational; the extension is NOT
  stripped from the public bulk feed — resolving the July open question
  (A7's "confirm DEX stripping") the observable way. **Verified on our side**:
  the SDK-DE 1.14-base inventory carries the 4 DEX fields, decisions are
  kind-derived (they store like any field), and the 2026-08-08 pipeline run
  ingested that day with zero new quarantine. Legal backdrop: VergStatVO
  amended effective 2026-07-01 (below-threshold reporting floor €25k → €50k).
- **eForms-DE 2.1 acceptance ends 2026-12-02** (DÖE support table). The
  successor (named 2.2 or 3.0, in flux) is being assembled on EU SDK 1.15
  (BT-22 split → BT-DEX-04, BR-DE-37/38, BT-DEX-05). **Issue 165** tracks
  vendoring it; the honest failure mode until then is unknown-customization
  quarantine, which is visible.
- SDK-DE releases: none after 1.14.4. DÖE OpenData API: unchanged (verified
  live — same endpoint, formats, anonymous access; 2026-08-07 export = 1,147
  notices). `eforms-sdk-0.1` dialect: still ~40% of volume (456/1,147 on
  08-07), no retirement announced. (Austria's October-2026 below-threshold
  mandate is a different country — do not confuse.)

## 4. TED access channels

- Bulk packages: URL patterns, anonymity, 404 semantics, coverage, internals
  all unchanged (verified live). Still no ETags/checksums. One cosmetic
  retroactive change: the `Content-Disposition` filename format — we key
  blobs by our own scheme, no impact (doc example corrected in place).
- **Search API v3 floor corrected**: rolling `today − 10 years`, boundary
  verified to the day (2016-08-09 on 2026-08-09). July's "July 2016 floor"
  was the rolling edge observed in July. Gap-fill/cross-check logic must
  compute the floor, and pre-window archive data is bulk-verifiable only
  (ted-access-channels.md corrected in place). The free integrity identity
  still holds: API day-count == package file-count (3,344 on 2026-08-07).
- No v4, no deprecation, no auth/quota/licence/cadence changes (release
  calendar verified: 254 issues, Mon–Fri). One in-flight rename:
  `noticeAuthorLocale` → `noticeAuthorLang` (both accepted; we use neither).
- Unreachable through this environment's proxy, explicitly not guessed:
  per-notice XML URLs (AWS WAF challenge), `/en/help/data-reuse`,
  `/en/legal-notice`. No contrary signals anywhere reachable; re-verify from
  the VPS if these become load-bearing.

## Action register

| # | Action | Where | When |
|---|--------|-------|------|
| 1 | Vendor eForms-DE successor (2.2/3.0) + resolve() arm | issue 165 | before 2026-12-02, or first unknown-customization quarantine |
| 2 | Vendor EU SDK 1.16 when final | issue 165 (watch item) | ~Sep–autumn 2026 |
| 3 | turso `=0.7.0` → `=0.7.2` via D1 reprobe protocol | issue 166 | next quiet day |
| 4 | Treat Search-API floor as rolling in any future gap-fill/cross-check design | ted-access-channels.md correction | standing |
| 5 | Next drift re-check | this file's successor | ~2026-09 (or on SDK 1.16/2.0, eForms-DE successor, turso 0.8.0 news) |

No new user decisions required: every finding either has no action, is
already handled by existing mechanisms, or is filed as a dated board issue.
