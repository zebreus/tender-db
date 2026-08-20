# 253 — data-quality cost is U-shaped across the id space, and the "11× slower" claim was measurement error

Status: CLOSED 2026-08-20 as **measured and inherent**. The run finished in 5,503 s against a 5,566 s
baseline — no regression, and the "~11x" premise was my own measurement error. The cost table and the
U-shaped profile are recorded below as the note for whoever next sizes the windows. The one
optimisation this issue proposed is ALSO withdrawn, and for a reason worth reading
Kind: was filed as a performance regression; is a cost-profile note plus a retracted claim
Blocked by: —
Relates to: 243 (which merged three award queries into this one and measured its cost), 230 (the
windowed measurement), 244 (the campaign that gave the era its awards), 235 (the numbers the query
feeds), 239/248 (the two turso planner gaps already measured on this corpus)

## What

Issue 243 measured the merged `awards` query across a full run: the three separate forms cost
1,346 s + 495 s + 813 s over 32 windows, and the merged one about 30 % less — call it **~58 s per
window**. The whole run was 5,566 s (92.8 min).

Watched live on 2026-08-20, the same query on **window 2 of 32 took ~10–12 minutes**. That is ~11× its
measured per-window cost. The queries around it are unremarkable: 11 other queries went by in the
following 6 minutes (~30 s each), so this is one query, not a box-wide slowdown. The server was at
51 % CPU throughout — working, not stalled.

## The obvious suspect, and why it is only a suspect

The text-era campaign (issue 244) is the thing that changed. Before it, legacy award notices projected
**no** result content: 0.3 % of a package's award notices carried a winner. They now carry 91.7 %, which
means `lot_results`, `notice_sections` of kind `LotResult`, and the winner organizations all grew by
millions of rows — in exactly the tender-id ranges the early windows cover.

The query does two per-row EXISTS probes and one document-type probe:

    EXISTS(SELECT 1 FROM lot_results lr WHERE lr.tender_id = tv.tender_id
                                          AND lr.notice_id = tv.caused_by_notice_id)
    NOT EXISTS(SELECT 1 FROM notice_sections s WHERE s.notice_id = tv.caused_by_notice_id
                                                 AND s.kind IN ('LotResult','TenderResult'))

`lot_results` has `UNIQUE(tender_id, notice_id, result_key)`, whose prefix should serve the first probe,
and `notice_sections` is keyed `(notice_id, section_id)` for the second. So **on paper both seek** — but
this engine has already been measured twice not using an index the shape of the query implies
(issue 248: a DELETE ignored the implicit composite-PK index and scanned 41.78M rows while the same
predicate as a SELECT seeked in 0.8 ms; issue 239: no predicate pushdown into any view). Which of the
three probes actually degraded is unknown, and guessing is how the last three wrong diagnoses on issue
247 happened.

## What to do, in order

1. **Measure, don't guess.** With the queue idle, time each of the three probes separately over one
   legacy window via `/v1/sql`, and compare against a high-numbered (eForms) window. `--print-sql` on
   the data-quality binary already emits the instantiated statements, so this needs no new code.
2. Only then decide between an index, a rewrite (e.g. a join over a pre-aggregated `lot_results` set
   instead of a per-row EXISTS), or accepting a slower run with the window count raised.
3. **A/B whatever lands**, on one window, before the next full run — the discipline issue 243
   established for this exact query.

## Why it matters beyond the report

The run holds the job queue. At the observed rate the early (legacy) windows alone would take hours,
which delays the 09:35 daily tick behind it and makes `ingest_freshness` read stale for no good reason.
Issue 252 is the other half of that problem: the run cannot currently be stopped once started.

## Acceptance

- The three probes timed separately on a legacy window and on an eForms window, recorded here.
- A named cause, or an explicit "measured and it is inherent" finding.
- If a fix lands: an A/B on one window, and the next full run's total recorded against the 5,566 s
  baseline.

---

## Groundwork done, and one suspect struck off (2026-08-20)

**Version inflation is NOT the mechanism.** The first thing that occurred to me was that each re-parse
plus fold might APPEND a `tender_versions` row, so 135 packages of campaign would have multiplied the
rows this query scans. It does not: a version is one row per *Notice* of the Tender, in publication
order, and a re-fold rewrites the same rows (the fold logs say "13,028 tenders written, 0 verified
unchanged"). Struck off before it could become a wrong diagnosis.

That also means the query's **row count did not grow**: its denominator is `TXT-TD = '7'`, which the
text era always had — 4,153 per package. So whatever got slower is per-row cost or page access, not
population.

**Two candidates, and they are distinguishable by measurement:**

1. **A per-row probe that degraded.** `lot_results` went from holding nothing for legacy tenders to
   millions of rows in exactly the low `tender_id` ranges the early windows cover, and text-era notices
   went from ~1 section to 1 + 2 per winner in `notice_sections`. Both probes should still seek, but
   this engine has twice been measured not using an index the query implies (239, 248).
2. **Cache and page pressure, with no plan change at all.** The same growth evicts the pages these
   probes used to hit warm. The box sat at **51 % CPU** while the slow query ran, which is more
   consistent with waiting on I/O than with a scan burning CPU — so this candidate deserves testing
   FIRST, and it needs no code change if true, only a window-size or ordering decision.

**The statement is captured.** `cargo run -p ingest --bin data-quality -- --print-sql` emits it
instantiated (window `tender_id 0..250000` shown); no prod access needed to get it. It is one
`tender_versions ⋈ notices` scan with three EXISTS families:

- the award predicate: six profile-gated `EXISTS` over `notice_codes(notice_id, section_id, field_id)`,
  OR'd — a text row therefore evaluates up to five arms that cannot match before reaching its own;
- `EXISTS lot_results (tender_id, notice_id)`;
- `NOT EXISTS notice_sections (notice_id, kind)`.

Worth noting for the rewrite option: the six OR'd arms are ordered eForms-first, so **every legacy row
pays five useless probes**. Ordering the arms by expected population, or dispatching on `n.profile`
before probing at all, is a cheap change that needs no index — but like everything else here it gets
timed first.


---

## CORRECTION (2026-08-20): the premise was measurement error, and it was mine

This issue was filed on one observation — the phase counter sitting on `window 2 … query awards`
across two readings about eight minutes apart — and turned into "~11× its measured per-window cost".
**That was wrong, and wrong in the exact way I criticised on issue 243 four firings ago:** a projection
filed as if it were a measurement.

Two errors compounded:

1. **The box was not idle.** Between those two readings a full `cargo test -p ingest --tests` run was
   saturating the CPU. Later readings taken while the box was quiet put the same query at 2–4 minutes
   per legacy window, not 10–12.
2. **One query is not the run.** The job already journals per-window elapsed — the design note in
   `run_data_quality` says so in as many words, "so the next sizing decision is a measurement rather
   than a third extrapolation" — and reading that journal answers the whole question. I filed before
   reading it.

### What the journal actually says

    window  1/32 (0..250000]         483.8s        window 17/32 (4.00M..4.25M]     62.0s
    window  2/32 (250000..500000]    429.4s        window 18/32 (4.25M..4.50M]     83.5s
    window  3/32 (500000..750000]    551.5s        window 19/32 (4.50M..4.75M]     97.3s
    window  4/32 (750000..1.00M]     590.8s        window 20/32 (4.75M..5.00M]    113.6s
    window  5/32 (1.00M..1.25M]      276.1s        window 21/32 (5.00M..5.25M]    116.5s
    windows 6-16 (1.25M..4.00M]    34-55s each     window 24/32 (5.75M..6.00M]    147.4s

At window 24 the run stood at **3,688.8 s elapsed**, on course for roughly the **5,566 s baseline**.
There is no regression. The corpus grew and the run did not get slower.

### What IS real, and it is a different shape

The cost profile is **U-shaped**, and the fixed `DQ_WINDOW = 250_000` fits neither end:

- **windows 1–5 cost 2,332 s — 66 % of the run so far, over 16 % of the id space.** The legacy eras
  are dense in low tender ids, and now carry result content they did not before.
- windows 6–16 cost 34–55 s each: the sparse middle.
- windows 17 onward climb steadily to 147 s: the modern eForms eras, dense per-version satellites.

Resizing windows would **not** reduce the total — every version is measured exactly once either way,
and more windows means marginally more statements. What it would improve is progress granularity and
worst-case cancellability, which issue 252 has now made matter.

### The one optimisation the measurement does support

The award predicate is six profile-gated `EXISTS` over `notice_codes`, OR'd, ordered **eForms-first**.
A legacy row therefore evaluates up to five arms that cannot match before reaching its own — and the
legacy windows are two thirds of the run. Dispatching on `n.profile` before probing, or simply ordering
the arms by expected population, is cheap and needs no index.

That is worth doing **only if the end-of-run `cost by query` line shows `awards` dominating the legacy
windows**. That line is emitted when the run finishes and has not been read yet. No code changes until
it is — which is the point this issue should have started from.

### Acceptance, revised

- The `cost by query` line for this run recorded here.
- If `awards` dominates: reorder the arms, A/B one legacy window, record both numbers.
- If it does not: close this as "measured, inherent", and keep the U-shaped profile as the note for
  whoever next thinks about window sizing.


---

## CLOSED: the run finished, and both of this issue's ideas were wrong

    data quality measured: 23 eras over 32 windows in 5503s; 0 label(s) unmeasured

**5,503 s against the 5,566 s baseline.** Not slower. The regression this issue was filed about does not
exist.

### The cost table, which is the durable output

    awards        2,063s        buyer          162s        winner          56s
    sections_can  1,093s        cpv            123s        value           38s
    doc_types       958s        deadline        89s        sections_with   27s
    title           772s        merge           65s        linkage         27s
                                                          versions        26s

`awards` is 37.5 % of the run, and `awards` + `sections_can` + `doc_types` + `title` are 89 % of it. For
scale, issue 243 measured the three pre-merge award queries at 1,346 + 495 + 813 = 2,654 s and predicted
~1,858 s merged; the actual 2,063 s is ~11 % above that prediction, which is ordinary corpus growth over
several weeks, not a step change.

### The window profile, U-shaped

    windows  1-5   (id ≤ 1.25M)     2,332s   —  66% of the run, 16% of the id space
    windows  6-16  (1.25M-4.0M)       477s   —  34-55s each, the sparse middle
    windows 17-24  (4.0M-6.0M)        733s   —  62s climbing to 147s
    windows 25-31  (6.0M-7.75M)     1,703s   —  183s to 274s, the dense modern eras
    window  32     (tail)             109s

The fixed `DQ_WINDOW = 250_000` fits neither end. Resizing would **not** reduce the total — every version
is measured exactly once either way — but it would even out progress reporting and shorten the
worst-case wait before a cancel takes effect, which issue 252 has just made matter.

### Withdrawing the optimisation too

This issue proposed reordering the award predicate's six profile-gated arms, on the reasoning that a
legacy row "pays five useless probes". **That reasoning is wrong.** Each arm is
`(n.profile LIKE '…') AND EXISTS(…)` — the cheap string test stands *before* its subquery, so a
non-matching arm costs a `LIKE`, not an index probe. Reordering would save four string comparisons per
row, which is nothing.

It is possible that this engine does not short-circuit `AND` the way the shape implies — it has
surprised us twice before (239, 248) — but that is a question to *test*, not a change to make. Recorded
here rather than acted on. Anyone picking it up: the A/B is cheap, since a narrow window
(`tender_id 0..25000`) brings the awards query under the `/v1/sql` 10 s cap.

### What this issue is worth keeping for

The cost table and the window profile. Everything else in it was me reasoning ahead of the measurement,
twice in the same issue.
