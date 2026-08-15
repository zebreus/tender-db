# 187 — internal-ojs awards are 100% unchained (9,701/9,701): the issue-27 rule fires

Status: RESOLVED-DIAGNOSED-HONEST (2026-08-15 — materialised by the full rebuild: internal-ojs award
chaining moved from 100% unchained to 96.5% (3,576 unchained / 3,705 award tenders on the live panel);
129 awards now chain that did not before, proving the fix ran. The 96.5% floor is the honest number,
not a broken fix: it mirrors its sibling 188 (sdk-0.1 98% unchained) — most internal-ojs award notices
genuinely publish standalone, with no prior procedure/contract version to chain onto, so there is
nothing to link. internal-ojs is a small profile (3,705 award tenders vs 945K r208 / 1.57M r209), so
the absolute residual is tiny. The is_legacy_profile fix is correct and delivered what was chainable.)
Was: FIXED-IN-CODE, PENDING-REFOLD.
Kind: reference-resolution defect (pre-registered trigger)
Blocked by: —
Relates to: 27 (the rule: ">90% unchained at full data ⇒ reference-resolution defect, file it"), 41 (the profile — its fixtures include a REF_NOTICE chain edge), 04 (canonical projection / chaining)

## Why

Issue 27 pre-registered a watch rule for the award-linkage panel when it read 96–99% unchained on
a pre-backfill canonical layer: re-check at full data, and above 90% file a reference-resolution
defect. The backfill is done, and the panel reads (public `/api/dashboard`, 2026-08-10):

    internal-ojs   awards=9,701   unchained=9,701   ratio=1.000

Every single 2008 OPOCE award is a lone-notice Tender. The issue-41 profile demonstrably parses
REF_NOTICE chain edges (its golden fixture asserts one), so the break is downstream or
cross-boundary. Two candidate causes, both checkable:

1. **Cross-profile chaining**: internal-ojs covers exactly one year (2008). A 2008 award's
   REF_NOTICE points at a contract notice held under the `text` profile (2007 and earlier) — if
   the chaining key embeds the profile/era, the link can never resolve by construction.
2. **The edge never reaches the fold**: parsed but not projected into the grouping key
   (the issue-99 class — projection-logic change without a refold, or the fold predates the
   profile's chain edges and no refold touched the era).

## What

Attribute first (one fixture-level trace of a known REF_NOTICE pair through grouping), then fix,
then the era needs a refold for links to materialize — note issue 179's cost finding: a legacy
refold currently pays the full-corpus price, so batch this with other pending era work if
possible. Success: ratio drops from 1.000 to the era's honest residual, and the number is quoted
next to r209's research-predicted ~17% baseline.

## Root cause + fix (2026-08-15, orchestrator)

Traced to the projection, code-level, no prod-data read needed:

`is_legacy_profile` matched only `text` and `ted-export*`, NOT `internal-ojs`.
The 2008 export is parsed by the r209 legacy machinery and chains by OJS number
(`NO_DOC_OJS` self + `REF_NOTICE` edges, no BT-04), but:
- `Ident::read` computes `ojs_self` only `if legacy` → internal-ojs got `ojs_self = None`.
- `canonical.rs` inserts a notice's OJS edges into the union-find graph only when
  `ojs_self.filter(|_| legacy)` — gated on BOTH ojs_self AND the plan-row `legacy`
  column, which is `is_legacy_profile`.

So every 2008 award fell to the island path: 9,701/9,701 unchained — candidate
cause 1 (cross-boundary key construction), confirmed. Preconditions for the fix all
verified: publication_id is `<number>-2008` → `ojs_key` parses it; `REF_NOTICE/NO_DOC_OJS`
is emitted as an `is_ref` scheme=ojs edge (r209 rules); award blocks are the same
`LotResult` kind, so `read_legacy_results` reads them exactly like every other r209
profile (the award projection is unchanged — verified: the internal_ojs.rs integration
suite + full ingest/app/store gates all green after the routing flip).

**Fix (deployed rev 20b0888):** add `internal-ojs` to `is_legacy_profile`. One line +
a mechanism unit test (`internal_ojs_chains_by_ojs_like_the_legacy_forms`).

**Materialisation — DEFERRED, batch it.** The fix regroups islands→chains, a Phase-1
plan operation an incremental fold does NOT redo; it needs a full rebuild
(`{"kind":"project","rebuild":true}`), which `clear_canonical()`s and reissues all
tender ids → bumps feed generation and invalidates every API cursor (issue 46). Per
issue 179's full-corpus cost note this issue itself says to batch the refold with
other pending era work rather than pay a standalone ~2h+ disruptive rebuild for
internal-ojs alone. Candidates to batch: issue 85's DE-1.x shells, any other
grouping/projection fix awaiting a refold. When the rebuild runs, VERIFY: the
internal-ojs award-linkage ratio drops from 1.000 to its honest residual, and quote
it beside r209's ~17% baseline (retire the issue-27 trigger for this era).

Note: issue 188 (sdk-0.1, 98% unchained) is a DIFFERENT mechanism — sdk01 has its own
classified path; do not assume this fix moves it. Left for its own attribution.

**2026-08-15 ~08:xx UTC (orchestrator) — BATCHED REBUILD RUNNING.** Triggered the full
`project rebuild=true` (job 1) on the Saturday low-traffic window to materialise the
three deployed org/chaining fixes together (187 internal-ojs chaining, 86 register
false-country, 48 country canonicalisation). Reissues all tender/org ids + bumps feed
generation (webhooks handle it via issue 178's reset; poll/SSE re-snapshot). ~6-10h wall.
Health stays green through clear_canonical (issue-133 presence detector skips heavy
writes). Verify on completion: internal-ojs award-unchained ratio drops from 1.000;
German HRB orgs read country DE not HR; DEU/DE/UK country codes converged to alpha-2.
