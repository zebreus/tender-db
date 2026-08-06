# 140 — eForms SDK 1.3 is a DEFECT, not reprocess residue (#39 entry finding)

Status: **measured 2026-08-06**, bounded `/v1/sql` reads under team-lead's per-read word.
Owner: sdk-vendor (finding) → proj-fix (the defect itself)
Relates to: 137 (the sizing that surfaced it), 39 (the reprocess this gates), 74 (which vendored 1.3)

## The finding, controlled

Of the 71,707 outstanding `unknown-customization` rows, **4,827 are `eforms-sdk-1.3` — 99.9% of that
SDK's entire population** (4,834 rows; only 7 ever reclaimed). Every other SDK left ~0–12% behind.

The obvious reading is "those notices were never in the reprocess run's scope". **That is refuted.**
All 4,827 sit in **9 fetches**, and in those same 9 fetches **95,968 of 109,649 quarantine rows
(87.5%) were reclaimed** — the run reached them.

Controlling for fetch removes the last confound. Within those 9 fetches only:

| sdk | reclaimed | unreclaimed | **% reclaimed** |
|---|---|---|---|
| 1.10 | 9,284 | 16 | 99.8% |
| 1.9 | 5,787 | 54 | 99.1% |
| 1.8 | 9,792 | 237 | 97.6% |
| 1.7 | 62,669 | 6,854 | 90.1% |
| 1.6 | 8,412 | 1,686 | 83.3% |
| **1.3** | **0** | **4,827** | **0.0%** |

**Same fetches, same run, same mechanism. Every peer reclaims 83–99.8%. 1.3 reclaims nothing.**
That is not scope and not residue: it is a **1.3-specific failure**, and it is a defect finding for
proj-fix's queue rather than work for the reprocess.

## What keeps it interesting rather than simple

**1.3 is vendored and accepted** — `fields-1.3.0.json` is on disk and `eforms-sdk-1.3` is in
`ACCEPTED` (`crates/ingest/src/eforms/sdk.rs`). So "no metadata" cannot be the explanation.

**And 1.3 is not universally unparseable: the 7 that did reclaim are all in a DIFFERENT fetch (43).**
So the same SDK succeeds somewhere and fails completely in the 9. Whatever is wrong is a property of
those notices or that fetch's shape, not of 1.3 support as such. Two candidates worth separating
before any fix: a `ProfileID` variant that `sdk::resolve` maps differently, or a field the 1.3
inventory lacks that only these notices use.

**Do not start by re-running the reprocess on them.** It is the natural move and it would return
4,827 failures with no more information than we have now.

## Why this had to be asked before the #39 reprocess, not after

A blind re-run over all 71,707 works for the proportional tail (1.7/1.8/1.6/1.9, ~10% each — a
mechanical edge, still to be characterised) and **silently fails all of 1.3**. The 4,827 would then
read as ordinary residue of a mostly-successful run, and a genuine defect would be filed as
leftovers. That is the four-DTD-populations-under-one-label shape (issue 84/#29) at a smaller scale:
**a real finding hidden inside a plausible aggregate.**

Cost of asking first: four bounded queries. Cost of not asking: the defect becomes invisible at the
moment it is most cheaply findable.

## Method note

Five bounded `/v1/sql` reads, status-checked, **408 never retried** (the cap bounds the wait, not the
work — a retry stacks a second copy server-side).

**Q1 timed out at the 10 s cap and was not retried.** Its numbers (4,834 total / 7 reclaimed / 4,827
outstanding) are taken from the committed issue-137 measurement instead, so the population figures
here come from a different, earlier read than the fetch-level ones — stated because a mixed-provenance
table that does not say so is exactly the defect this file exists to prevent elsewhere.

## The generalisable shape

**A uniform-looking shortfall can contain a categorical one.** Group A's ~10% tails and 1.3's 100%
sat in a single "71,707 outstanding" number and look identical there. They separate the moment you
ask for the rate *per member of the population* rather than the total — and the control (peers in the
same fetches) is what turns "1.3 looks odd" into "1.3 is the only one at zero."


---

# Group A (the ~10% tail) — measured 2026-08-06, and it changes #39's premise

The other half of #39's characterization. **Four things established, one left open, and the open one
matters more than #39's current wording admits.**

## What it is NOT

**Not a scope gap.** 1.7's outstanding rows sit in 22 of the 33 fetches that hold 1.7 at all, and
those fetches were processed — 95,968 of 109,649 rows reclaimed in the 9 examined closely.

**Not unreachable.** `reclaim_notice` flags by `notice_id`, which is nullable, so rows without one
would be untouchable by construction. Measured: of 71,707 outstanding rows, **0 have a null
`notice_id`** (and 0 of the 1,129,367 reclaimed ones do either). **My hypothesis, and it was refuted
cleanly.** They were reachable.

**Not a stopped or resumed run.** `run_reprocess` takes `resume_after`, which skips packages a prior
run drained — so an interrupted run would leave *whole fetches* untouched. Instead the shortfall is
**uniform within every fetch**: 1.7 leaves 8.7–16.5% behind in each of its top ten fetches. A
mechanical stop cannot produce a consistent within-fetch fraction.

## What it IS, so far

**A per-notice failure whose rate tracks SDK version.** Within the same 9 fetches — same run, same
mechanism, fetch held constant:

| sdk | % still held |
|---|---|
| 1.10 | 0.2% |
| 1.9 | 0.9% |
| 1.8 | 2.4% |
| 1.7 | 9.9% |
| 1.6 | 16.7% |
| **1.3** | **100%** |

**Monotone in version across 1.6→1.10**, with 1.3 off the scale (issue 140 above). A scope or
scheduling artefact has no reason to correlate with SDK version; a *parse* failure does — older
minors carry constructs the newer inventories dropped or renamed.

## The open question, and why it is load-bearing

**Were these rows attempted and failed, or never attempted?** `run_reprocess` counts `still_held` as
a distinct outcome, so attempt-and-fail is a recorded possibility in the code — but I could not read
the historical counts: `job_log` is deliberately **outside `/v1/sql`'s queryable public surface**
(correct design; the endpoint is a public API, not an admin one), and the outcome string does not
appear in the journal.

**If attempted-and-failed — which the version gradient argues for — then #39's premise is wrong.**
The task reads "reprocess the 71,707". Re-running would reproduce 71,707 failures and return no
information. The work would be *diagnosis*, as with 1.3, not *reprocessing*.

**Cheapest way to settle it:** read `job_log`'s `counts_json` for the July reprocess runs via the
admin path (not `/v1/sql`), or run the reprocess over **one** fetch's 1.7 rows and see whether
`still_held` comes back non-zero. The second is a few hundred rows and answers it directly.

**Recorded as inference, not measurement:** the version gradient is strong evidence for
attempted-and-failed and is not proof of it. #39 should not be re-scoped on this paragraph alone —
it should be re-scoped after one of those two checks.
