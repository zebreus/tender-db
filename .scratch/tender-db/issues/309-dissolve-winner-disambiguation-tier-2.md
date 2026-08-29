# 309 — dissolve winner disambiguation (tiers 2-4): rescue the skipped orgs

FINAL (2026-08-28 ~23:5x, rev c2b9411, wet runs 1335/1337/1342, every one
preview-exact): **7,774 of 7,865 condemned orgs dissolved (98.8%)**.
The decisive last piece was ORIGIN-FIRST winner resolution: the
lot_result's origin notice (where the result was published) names the
winner — a single mention there IS the winner side (the eForms shape:
org sections are siblings, never RES descendants — measured on the NIMAT
flagships), with the RES-descent walk for multi-mention legacy origins,
and causing-notice tiers as fallback only (a carried row's causing notice
can name a DIFFERENT real entity under the same condemned platform id).
Cumulative: 168,440 mentions re-resolved; **557,130 duplicate winner rows
erased** (placeholder rows standing beside the real winner — served award
double-counting, gone); the whole NIMAT family dissolved; every allowlist
member intact; closing census (run 1343): the corpus's top distinct-name
org is now the Tribunal (700, legitimate) — no placeholder remains at the
top of the distribution. **Residual: 91 orgs** (lexicon 38 / sequence 26
/ short-vat 41 per the census), incl. flagships 15176 (DE123456789) and
15566 (bare 123456789): legacy multi-mention origins where the descent
ties — per-org diagnosis is the remaining unit.

Status: TIERS 2+3 LANDED AND RUN 2026-08-28 (12de6f3; wet run job 1337) —
**924 of the 1,375 rescued** (334,783 winner rows, 300,503 bid-party rows,
41,265 parties, 31,663 mentions, 7,580 tenders; preview-exact). The prod
run also revealed the REAL tier-1 skip mechanism: rounds ACCUMULATE, so a
carried-forward winner row's causing notice never mentions the org at all
— tier 3 (tender-chain-scoped single-(name,country) resolution) covers
that shape. **451 residual** — including the seven flagship mega-orgs
(NIMAT500-503/100, DE123456789/15176, bare-123456789/15566): within one
tender's chain they carry mentions under DIFFERENT names (buyer and winner
both placeholder-keyed), honestly ambiguous to tiers 1-3. Tier 4 below is
the designed fix.
Kind: repair-job refinement
Relates to: 300 Stage 1 (the dissolve whose skip set this is), 259 (the
guard discipline it inherits — and whose section-walk machinery tier 4
reuses).

## Tier 5 (the residual-91 design, 2026-08-29 00:1x): dissolve-then-refold

Tier 4c rescued zero: the true residual shape (measured on 15176's origin
26730615) is a multi-lot CAN where SEVERAL DIFFERENT real winners all
published the placeholder id — three Tenderer sections, one condemned org,
one lot each. The per-lot linkage (LotResult → SettledContract → LotTender
→ TenderingParty) is FLATTENED at fold time and not persisted; winner rows
are its product. So the canonical layer cannot disambiguate — but it does
not need to: winner rows are DERIVED. For the 91: rewrite mentions and
party/bid-party rows per-section (fully deterministic), DELETE the
ambiguous winner rows and the org, and stamp the affected tenders
epoch-stale (the issue-179 refold mechanism) — the next fold re-derives
every winner row correctly from the re-bound mentions via
bind_organizations. Precondition to verify in tests: an epoch-stale refold
rewrites a version's winner set wholesale. Org-atomicity keeps its
meaning: the org disappears in one transaction; the winner TRUTH arrives
with the refold, minutes later, derived rather than guessed.

### Tier 5 BUILT (2026-08-29 01:xx) — precondition verified, then pinned

Precondition verified at every link before writing a line, all from the
production code paths: (1) the delta planner expands a requeued notice to
its Tender's FULL notice set (`touched_existing_tender_ids` →
`notice_ids_for_tenders`, project.rs pass-1/expansion) so a partial
requeue never folds a truncated chain; (2) a stale stored epoch forces
`keep = 0` and `delete_version` + `write_version` rewrite EVERY version
satellite, winners included (canonical.rs `apply_tender_tx`, the issue-99
branch); (3) winner rows derive from `organization_mentions` as they
stand (`mentions_by_ids` → `bind_organizations` → `raw_results.bind`) and
the resolver's `(notice, section)` idempotency map keeps a rewritten
binding on re-projection — the raw payload's condemned identifier cannot
re-mint the dissolved org.

Landed: `dissolve_condemned` no longer skips — a winner row no tier
resolves is DELETED (`winners_deleted`), its tender collected, and after
the per-org loop the batch applies the issue-179 pair inside the same
transaction: `stamp_tenders_stale` (`refold_tenders`) + a causing-notice
requeue UPDATE (`refold_notices`), dry-run previewing all three counts
exactly (COUNT with the identical predicate). `skipped` stays as a
tripwire — nonzero now means a NEW unhandled shape. Tests: the store test
drives org 51 through tier 5 (row deleted, epoch 0, notice requeued,
change feed rows); new ingest test
`a_stamped_refold_rederives_winners_from_rewritten_mentions` pins the
whole contract end-to-end on the Gebrüder Schneller fixture — dissolve
writes mimicked, `unmark_projected_by_ids` + `stamp_stale_for_notices`,
then `project_incremental` re-derives the winner row onto the re-bound
mention's org and the dissolved org stays dissolved.

Rollout: gate → deploy → preview (expect condemned 91, winners_deleted >
0, refold_tenders ≈ affected CANs, skipped 0) → wet run → enqueue an
incremental fold (or let the scheduled cycle take it) → closing census
(expect condemned 0) → the gate-invariant tripwire returns to 0.

Gate 71/71 green; deployed rev 93eeb29 2026-08-29 ~01:2x. PREVIEW (job
435, 1213s): 91 condemned, **91 dissolved, 0 skipped**; 25,850 mentions
re-resolved (6,554 fresh + 19,296 reused); 64,409 parties, 37,739
bid-parties, 59,581 winner rows still deterministically repointed by the
origin/causing tiers; **1,046 ambiguous winner rows → tier-5 delete**
across **393 tenders** (834 notices to requeue); 10,236 tenders touched.
The ambiguous fraction is 1.7% of the flagships' winner rows — origin-
first resolution carries the rest even inside the residual set.

WET RUN (job 436, 1128s): preview-exact on every deterministic count —
**91 dissolved, 0 skipped**, 25,850 mentions, 64,409 parties, 37,739
bid-parties, 59,581 winner repoints, 1,046 tier-5 deletes, 393 tenders
stamped. Two expected dry/wet divergences, both dry-run simulation
limits (dry cannot observe its own writes): fresh/reused split
6,554/19,296 → 1,412/24,438 (wet reuses the orgs the first mention
mints; the 25,850 sum is invariant) and refold_notices 834 → 819 (a
notice shared by two scan batches' refold sets counts once when the
first batch actually writes projected=0). Incremental fold enqueued
(job 437) to re-derive the 393 tenders' winner sets; closing census
after.

## Tier 4 (landed): lot-result origin resolution

`lot_results` rows carry their ORIGIN (tender_id, notice_id, result_key) —
the notice and RES-section where the result was published, independent of
which version's row carried the winner forward. For a winner row
(T, seq, LR): take lot_results[LR].notice_id + result_key; in THAT notice's
parse layer (notice_sections), find the Organization-kind section(s) whose
ancestor chain reaches the RES section (the 259 `nested_org_aliases`-style
walk, PARTY_KINDS families); the condemned org's mention on (notice, that
section) is the winner-side mention — per-ROW precise, no name agreement
needed. Guards: exactly one such section resolving to the condemned org,
else skip (org-atomic as ever). This is the honest per-row answer the
schema always contained; tiers 1-3 remain as cheap fast paths.

## What

The placeholder dissolve's winner repoint resolves through the version's
`caused_by_notice_id` and requires exactly ONE mention of the condemned org
on that notice; any org with an unresolvable winner row is skipped whole.
The first prod dry-run (job 1333): 7,865 condemned, 6,490 dissolvable,
**1,375 skipped** — including flagship exemplars: org 15566 (bare-123456789
DE bucket, 723 multi-mention notices) and likely 15176 and the PL823 org.
The NIMAT flagship (org 211) is clean (0 multi-mention notices) and
dissolves in tier 1.

## The tier-2 rule

`tender_version_parties` rows carry role + the exact (mention_notice_id,
mention_section_id). For a winner row on (T, seq) whose causing notice
holds ≥2 mentions of the condemned org: among those mentions, keep the ones
whose (notice, section) appears in `tender_version_parties` on the SAME
(T, seq) with a winner-family role and organization_id = the condemned org.
If exactly one survives, that mention's re-resolution target takes the
winner row; else still skip. This should rescue most of the 1,375 — a
notice typically mentions a placeholder org twice as buyer + winner, and
the party roles split them.

## Acceptance

- unit test: a two-mention notice (buyer S-1 + winner S-2, both on the
  condemned org) dissolves, the winner row following S-2's target;
- prod dry-run: skipped count falls substantially from 1,375; 15566 moves
  to the dissolved set;
- the tier-1 run's results are untouched (tier 2 only widens).
