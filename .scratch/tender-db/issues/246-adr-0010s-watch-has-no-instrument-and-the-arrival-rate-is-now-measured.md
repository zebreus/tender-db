# 246 — ADR-0010's reopen trigger has no instrument, and the arrival rate turns out to be measurable

Status: RESOLVED 2026-08-19 — step 1 (instrument, deployed 6cb2853) and step 2 (ADR-0010 amendment,
d96c073) both done; step 3 is a no-op unless the rate climbs
Kind: instrument gap behind a policy decision (not a defect in the policy)
Blocked by: —
Relates to: ADR-0010 (sub-cent amounts stay quarantined; names its own reopen trigger), 184 (declared
the campaign's terminal state and the disclosure), 144 (diagnosed cause F), 171 (the value-domain
study ADR-0010 nominates as "the watch"), 132 (negative amounts), 53 (`/metrics`), 230/235 (the report
this belongs in)

## What ADR-0010 promised

> Revisit only if the value domain shifts (issue 171 is the watch) … If cause F ever grows past a
> nuisance, this is the alternative to reopen first [claim-and-store] — it is cheaper than (1) and
> honest, just not worth the surface today.

The decision is sound and deliberately reversible. What it does not have is a way to notice the
condition it names. **Issue 171 is a one-off study**, done once on 2026-08-09; nothing measures cause F
continuously, and the obvious proxy — the size of the `unrepresentable-value` bucket — cannot answer
the question, because relabel passes move rows INTO the bucket from other reasons and the bucket's own
age is an artifact of a July repopulation.

## The arrival rate, measured

Today's daily ingest quarantined 3 notices, all `BT-720-Tender: amount has more than two fraction
digits` — `358168.44102` (sdk-1.12), `268.527` and `68.53280` (de-2.1). Following that thread:

`first_reason IS NULL` is the discriminator that makes this measurable at all — issue 87's relabel
machinery preserves the first-ingest reason there, so a row that has never been relabelled is a genuine
first-time hold:

| provenance | rows | first_seen span |
|-----------|------|-----------------|
| never relabelled | 3,122 | 2026-07-19 → 2026-08-19 |
| relabelled from `unknown-customization` | 1,899 | 2026-07-22/23 |
| relabelled from `unrepresentable-value` (failed again) | 164 | 2026-07-23 |

The 1,899 are issue 184's step-2 drain landing exactly where it predicted (it forecast 1,907). And the
never-relabelled rows are NOT a steady stream — one day dominates:

    2026-07-23  2884      <- the DE-1.x reprocess campaign meeting sub-cent amounts at scale
    2026-07-24    48
    2026-07-29    20
    2026-08-05    17
    …others       14–20

Steady-state, against the day's own ingest:

| day | notices ingested | held sub-cent | share |
|-----|------------------|---------------|-------|
| 2026-08-19 | 4,046 |  3 | 0.07 % |
| 2026-08-18 | 3,038 |  5 | 0.16 % |
| 2026-08-17 | 3,782 |  5 | 0.13 % |
| 2026-08-13 | 5,810 |  8 | 0.14 % |
| 2026-08-12 | 4,220 |  8 | 0.19 % |
| 2026-08-11 | 3,258 |  8 | 0.25 % |
| 2026-08-10 | 4,051 | 13 | 0.32 % |

**~5–13 notices a day, ~0.15 % of arrivals, ~2,000–4,700 a year.** Each one is a whole notice absent
from the corpus — buyer, title, CPV, dates, lots and all — over an amount with three or more decimal
places. Verified on one of today's: notice 28,456,452 has 0 versions, 0 texts, 0 codes.

## What this does and does not say

It does **not** say the policy is wrong. ADR-0010's reasoning survives this measurement intact: a finer
integer unit is an epoch bump over every amount in the corpus, rounding invents values the source never
published, and claim-and-store splits the amounts surface. 0.15 % of arrivals is not obviously past
"nuisance".

It does say two things worth acting on:

1. **The framing in ADR-0010 flatters the cost.** "~0.02 % of notices" was cause F measured against the
   7.9M historical corpus. Measured against what is arriving now it is ~0.15 %, about seven times that,
   because sub-cent precision is a live eForms practice rather than a historical artifact — sdk-1.12 and
   de-2.1 in today's three. The denominator, not the policy, is what changed.
2. **The trigger needs an instrument**, or the next person to ask "has cause F grown?" repeats today's
   half-hour of forensics — and will get it wrong if they use the bucket size, which is dominated by one
   relabel day and one campaign day.

## Steps

1. Put the arrival rate in the data-quality report: fresh holds by reason over the last 30 days, with
   the day's ingest as the denominator, keyed on `first_reason IS NULL` so relabels cannot inflate it.
   `unwindowed_labels()` exists for exactly this shape — a query that cannot be windowed by
   `tender_id` — and is currently empty, its comment already anticipating the first such query.
2. State the arrival rate in ADR-0010 as an amendment (not a reversal): the share of ARRIVALS, next to
   the historical share, so the trigger is stated in the units it will be observed in.
3. Only then, if the rate climbs: reopen alternative (3) claim-and-store, as ADR-0010 directs.

## Acceptance

- "Has cause F grown past a nuisance?" is answerable from the report, without ad-hoc SQL and without
  the relabel confound.
- ADR-0010 carries the arrival-rate framing alongside its corpus-share framing.


---

## Step 1 landed 2026-08-19 — section 5 of the data-quality report

    == 5. Quarantine arrivals (first-time holds in the last 30 days, per reason) ==
      reason                                                arrivals  newest
      unrepresentable-value                                    3,103  2026-08-19
      not-utf8                                                 1,492  2026-07-21
      unknown-root                                               753  2026-07-21
      …

Arrivals keyed on `first_reason IS NULL`, with the newest arrival as a date so a live
bucket is distinguishable from settled residue at a glance — `unrepresentable-value` fed today,
`not-utf8` and `unknown-root` untouched for a month. The rendered table carries the caveat that this is
arrivals and not bucket size, because the two differ by a factor of two in this very bucket.

No denominator: counting notices ingested in the same window is a full `notices` scan, measured at over
10 s against the public endpoint's limit, and an arrival rate stands on its own.

**A new query category.** This is the first query that cannot be windowed by `tender_id` — quarantine
rows have no tender — but can still be measured, so it runs once through a new `whole_corpus_queries()`.
`unwindowed_labels()` keeps its meaning: a query that cannot be measured at all. The catalog invariant
is now a three-way partition and also asserts disjointness, since a label in two lists would be counted
twice and an arrival count would silently double.

Verified on the box by dry run (job 759): *"480 statements, plus 1 whole-corpus statement(s) run once
(fresh_holds); every label is windowed or whole-corpus"*. The table itself renders on the next full run
(weekly schedule); the query was run against prod by hand first, which is where the numbers above come
from.

### Note on a date in this issue

An earlier revision of the text above said `not-utf8` was last fed 2026-07-19. It is 2026-07-21 — the
unit test's date assertion caught the slip when it was written against the same timestamps. The
substance (dormant for a month) is unchanged.

### Still open

- **Step 2 done** (`d96c073`): ADR-0010 carries an amendment stating the cost as a rate (~0.15 % of
  arrivals, 5–13 notices a day) beside its corpus share (~0.02 %), notes that the decision is unchanged
  because 0.15 % does not clear "past a nuisance", and replaces the nomination of issue 171 (a one-off
  study) with section 5 as the continuous watch. It also records why the bucket total is the wrong thing
  to watch, with the 1,899-renamed / 2,884-in-one-day numbers.
- **Step 3** is a no-op unless the rate climbs, at which point ADR-0010 names claim-and-store as the
  alternative to reopen first. Nothing to schedule: the report now carries the number that would say so.
