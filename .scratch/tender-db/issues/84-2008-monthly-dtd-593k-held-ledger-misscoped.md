# 84 — ~593K held 2008 monthly-TED DTD notices; ledger reports the category "resolved" (mis-scoped to the ~28K opoce subset)

Status: open — **the "largest remaining reclaim" framing below is RETRACTED by the author; see the Correction.** The held rows are almost certainly the non-English duplicate siblings, i.e. a counting defect, not lost data. Awaiting one read-only falsifier query against a prod snapshot. DISCOVERED 2026-08-01, CORRECTED 2026-08-04 (both proj-fix).
Kind: completeness / data-quality + ledger correctness
Blocked by: —
Relates to: 36 (XXE-safe DTD strip), 41 (internal-ojs parser, the opoce subset), 73 (unparsable-xml sizing — this REVISES it), ADR-0004, ADR-0009, [[resolved-categories-ledger]]

## Finding (snapshot 531, 2026-08-01)

Post-recovery, `unparsable-xml` still holds **601,256** notices. By `detail`:

| detail | held |
|---|---|
| `XML with DTD detected` | **594,915** |
| `unknown token at 1:1` | 4,441 |
| `unknown token at 1:3` | 1,900 |

Joined to `fetches`, the DTD rows are **ted/monthly/2008 = 593,017 held** vs **26,948 reclaimed**
(same source/kind/year), plus 1,898 from 2010. So the 2008 DTD bucket is really **~620K**, not the
~28K that issue 41 scoped.

## Why it's still held (NOT a regression)

Issue 36 makes DTD-bearing XML parseable (strip the DOCTYPE, resolve no external entities). Issue 41's
`internal-ojs` parser then reclaimed the **opoce-only S-series subset** — "the whole 2008 coverage gap"
as it was understood — which is exactly the **26,948** now reclaimed. The other **~593K** 2008 DTD notices
are a different vintage that was never in the internal-ojs parser's scope and remain quarantined. My earlier
"~116K genuine unparseable remainder" estimate was wrong: the genuinely-unparseable-looking residual is only
**6,341** (the `unknown token` rows). Everything else here is a tractable, large reclaim target.

## Two actions

1. **Investigate + reclaim (the big lever):** determine what the ~593K 2008 (+~1.9K 2010) DTD notices ARE —
   likely the older standard TED_EXPORT XML (R2.0.x) that legitimately declares a DTD, distinct from the
   opoce INTERNAL_OJS backbone. Confirm whether, after the issue-36 DTD strip, they parse under an existing
   TED-XML profile or need a mapping profile. If reclaimable, run the reprocess (ADR-0009,
   `reason=unparsable-xml detail_like='XML with DTD detected'` scoped to the non-opoce set) + fold. This is
   the single largest remaining reclaim (~4.8% of the corpus), bigger than everything left in the SDK/DE buckets.

2. **Correct the ledger scope (correctness):** the dashboard "Resolved categories" entry
   "2008 OPOCE INTERNAL_OJS (DTD)" (issue 36) presents the 2008-DTD category as resolved while 593K notices
   of that exact `XML with DTD detected` signature are still held. `coverage.rs` does show its live
   outstanding count, but the "resolved" framing is misleading given the bucket's true size. Re-scope the
   ledger entry to the opoce subset it actually covers, and track the broader 2008-DTD reclaim as this open
   issue — so the user-facing record matches reality (a core project goal).

## Note

This corrects issue 73's sizing (which sized unparsable-xml at 628K and assumed issues 36+41 covered the
recoverable share). The recoverable share is far larger than the 28K reclaimed.

---

# Correction 2026-08-04 (proj-fix) — this is a counting defect, not a reclaim opportunity

Everything above is the count as the database presents it. The count is real; **my reading of it was
wrong, and wrong in the flattering direction** — I sized a backlog of recoverable notices where the
evidence says duplicates already ingested. Four findings, all from the code, none needing prod:

1. **No live code path can write that reason.** `XML with DTD detected` appears nowhere in the ingest
   crate — only in a store test helper (`store/src/lib.rs:2638`) and in `quarantine-ledger.json`. Since
   issue 36 the DOCTYPE is stripped before parsing (`profile.rs:117`). So all 594,915 rows are **stale
   pre-fix rows**: a record of how ingest behaved in the past, not of what the payload is.

2. **Today those members are skipped, not quarantined.** The 2008 OPOCE export ships one file per
   language. `dispatch_internal_ojs` ingests the English file and returns
   `Disposition::Skipped("internal-ojs-non-english")` for the rest (`profile.rs:182`) — the documented
   duplicate policy, and `internal_ojs.rs` states the era is "~28k real S-series notices" across ~22
   languages. The siblings are not notices we are missing; they are the same notices.

3. **A skipped member is invisible to the reclaim.** `spawn_record_producer` emits no record for
   `Skipped` (`process.rs:240`), and `reclaim_package` only writes on `Record::Notice`
   (`process.rs:391`). A skipped held member is therefore never reclaimed, never counted `still_held`,
   never flagged `reprocessed_at` — it stays in the work list of every future reprocess, forever, and
   the pass reports **nothing** about it. That is why job [5] reported 26,948 reclaimed against a
   ~620K bucket and nobody could see where the rest went.

4. **The arithmetic fits exactly.** 593,017 held ÷ 26,948 reclaimed = **22.0** — one English file plus
   21 siblings, which is the language count `internal_ojs.rs` documents for the era.

So the reclaim opportunity is ~0, and the two real defects are the mirror image of what I filed:

- **The outstanding count overstates the gap by ~593K.** The user-facing dashboard shows work
  remaining that does not exist. My action (2) below is still right, but for the opposite reason: the
  ledger's problem is not that it calls the category resolved, it is that ~593K duplicate-skips are
  counted as outstanding *against* it.
- **`reclaim_package` had an unrepresented outcome.** Fixed here: `ReclaimReport.skipped_by_policy`,
  surfaced in the job summary, so the four outcomes sum to the held set and a bucket that cannot move
  says so. Test `reclaim_accounts_for_held_members_a_dispatch_policy_skips` (ingest/tests/process.rs)
  drives the real `115165_2008.en`/`.fr` fixture pair through a reclaim: EN reclaims, FR is reported
  as skipped, and the row is confirmed unflagged. Verified to fail without the fix (probe: force
  `skipped_by_policy = 0` → `left: 0, right: 1`).

## What is NOT settled — the falsifier

Findings 1–3 are mechanism, proven against the artifact. Finding 4 is a **ratio**, and a ratio is not
a population. The claim "the 593,017 held rows ARE the non-English siblings" is untested. It fails if
a material share of those rows are `.en` members, or if their notices never ingested. One read-only
query against an existing prod snapshot settles it:

- the held 2008 `XML with DTD detected` rows grouped by their `member_path` language extension
  (expect: ~0 `.en`, the rest spread over ~21 languages), and
- for a sample of held non-`.en` rows, whether the sibling `<doc>-<year>` notice exists and is
  `parsed` (expect: yes — the notice is already in the corpus).

If those come back as expected, the outstanding count is corrected (not reclaimed) and the ledger
entry is re-scoped to say so. If they do not, this correction is wrong and the issue stands as
originally filed. **Do not write either conclusion into the ledger before the query runs** — that is
the mistake this correction is fixing, made once already.

The residual genuinely-unparseable set is unchanged either way: 6,341 `unknown token` rows.
