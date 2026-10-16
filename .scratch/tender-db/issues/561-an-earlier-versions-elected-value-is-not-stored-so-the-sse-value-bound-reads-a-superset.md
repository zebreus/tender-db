# 561 — an earlier version's elected value is not stored, so the SSE value bound reads a superset and `max_value` (and at times `min_value`) hears `changed` where `added` was due

Status: ready-for-agent — filed 2026-10-16 from issue 547's build review. Next: decide whether to store the elected
Tender value per version (as 490 stores the lots'), which needs a refold, or keep the documented one-sided superset.
Kind: correctness (SSE `added`/`changed` under `min_value`/`max_value`)
Relates to:
- 547: `ValueBy::Version` reads the head column at the head and, at an earlier version, `EXISTS` an amount row of that
  version inside the bound — a superset, because the election (sentinels, scale partners over the chain; issues 366,
  375, 471, 492, 505, 537) is not transcribed into SQL.
- 490: lot values are already stored per version for exactly this reason (`Scope::At` serves non-head versions).
- 164 / 287: removal over-delivery, the accepted direction of the error.
- 560: lots hear a Tender value move only with a lot row; independent of this one.

## Verify

    grep -c 'ELSE EXISTS (SELECT 1 FROM tender_version_amounts va' crates/store/src/read.rs

- **done**: 0 (the At probe reads a stored per-version elected value), OR a `## Decision` below keeps the superset
  with its reason.
- **open (2026-10-16)**: 1 (the superset leg in `version_predicates`' `ValueBy::Version` arm).

## What was found

The earlier side matches when ANY amount row of that version lies within the bound. Under `max_value` that includes
every lot figure below the procedure total, so on a Tender with several lots an earlier version nearly always reads as
inside a `max_value` bound: a Tender whose value drops into the bound arrives as `changed`, never `added`. Reproduced
in the 547 review: CHAIN[0] with procedure EUR 900,000 / lot EUR 300,000, then procedure EUR 400,000 / lot EUR 300,000
→ `/v1/tenders?max_value=50000000` and `/v1/lots?max_value=50000000` both got `changed` (snapshot 0, page 1); with the
v1 lot at EUR 600,000 instead both got `added`. Under `min_value` the residue is any stored figure the earlier
version did not elect lying inside the bound — a refused figure (e.g. implausible or scale-refused), a
partner-only BT-709/BT-660 framework figure (stored and served at its lot, issue 506, never elected), a `Part` figure
(`elect(false)` on a version with Lots, issue 507). It gives an over-delivered idempotent `removed` when the new head
leaves the bound, or `changed` where `added` was due when the new head is inside it. Reproduced in the 547
re-review (2026-10-16): CHAIN[0] with the procedure at EUR 400,000 plus an injected BT-709 maximum of EUR 1,000,000
for LOT-0001 (stored as `result_framework_maximum`, not elected; both `min_value=50000000` snapshots empty), then
CHAIN[1] at EUR 600,000 → `/v1/tenders?min_value=50000000` and `/v1/lots?min_value=50000000` both got `changed`;
a refused figure (the lot at EUR 200 bn) in place of the BT-709 one did the same. The served docs now say to apply
`changed` as an upsert by id, which makes the client state right either way; the event is still the wrong op.

## Units

1. **Decide (S).** A: a `tender_versions.value_eur_cents` (or a sibling table) written by the fold for every version
   with `head_value_eur_cents_with` over the running `ScalePartners`, read by the At probe in place of the EXISTS;
   costs a column fill over every version (job-706 shape, 375's rule: the fold's own function, not SQL). B: keep the
   superset.
2. **Build + fill (M–L)** if A, then retire the CHANGELOG 547 caveat and the `/docs` superset clause (keep the
   upsert advice: `version: null` rows still need it).
