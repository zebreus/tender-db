# 100 — eForms-DE 1.x award-winner chain resolves to nothing (synthetic result-section ids vs published-id references)

Status: open — DESIGN DECIDED 2026-08-15. The blocking measurement (below, "Open question") is now
DONE and the answer is the cheap path: within a single notice the published result ids are UNIQUE, so
result sections can be keyed on the published id and the winner chain resolves without a disambiguation
scheme. Still a parse-layer change (re-parse + re-fold the DE-1.x cohort), but no longer blocked on the
unknown. Next step: implement in the eForms result-section synthesis (eforms/index.rs grafting +
eforms/parse.rs section-id-when-no-identifier) — deep sdk-vendor work with fixtures, deliberately not
rushed. Was: open, DISCOVERED 2026-08-02.

## Measurement — the blocking question, answered (2026-08-15)

The author's within-notice-duplicates query (below) previously "did not complete (starved by
contention)." Re-run on the current box (queue idle, post-rebuild), over a 200-notice window of each
cohort:

| cohort | field | values | dupes WITHIN a notice |
|---|---|---:|---:|
| de-1.2 | DE1-NoticeResult-LotResult-ID | 5,902 | **0** |
| de-1.2 | DE1-NoticeResult-LotTender-ID | 5,217 | **0** |
| de-1.2 | DE1-NoticeResult-SettledContract-ID | 4,855 | **0** |
| de-1.2 | DE1-NoticeResult-TenderingParty-ID | 4,760 | **0** |
| de-1.1 | DE1-NoticeResult-LotResult-ID | 67 | **0** |
| de-1.1 | DE1-NoticeResult-TenderingParty-ID | 47 | **0** |

**Zero within-notice duplicates in both cohorts.** So issue 75's "the ids repeat across grafted
positions" was a CROSS-notice observation; within one notice each `RES-`/`TEN-`/`CON-`/`TPA-` is unique.
Section ids only need to be unique within their notice, so keying result sections on the published id is
safe — this is the "one-line fix" arm the issue named, not the disambiguation-scheme arm. The synthesis
that gave them synthetic ids was over-cautious for the notice-scoped case.
Kind: correctness / completeness (parse-layer identity)
Blocked by: —
Relates to: 75 (the section-id synthesis decision this comes from), 78 (the DÖE grafting that forced it),
98 (the organization-reference class — a *different* defect that does NOT fix this), 85, 13 (results layer)

## Symptom

DE-1.x award notices project their results **structure** but almost never a **winner**:

- of 247 award-bearing DE-1.x versions in a de-1.2 window, **16 (2%)** carry a winner — and those 16 are
  parties carried forward from a merged TED twin, not DE data (same carry-forward that made the buyer
  rate read 35% when the true DE contribution was 0%, issue 98);
- `lot_results` 247, `bids` 216, `contracts` 217 in the same window — the graph's *nodes* land fine.

So the results layer exists and is empty of the one fact that matters commercially: who won.

## Root cause — the references and the sections speak different id vocabularies

Measured over 50 DE-1.2 notices. `resolves` = the reference value matches a `notice_sections.section_id`
in the same notice:

| reference field | values | resolves | example value |
|---|---:|---:|---|
| `DE1-NoticeResult-LotResult-TenderLot-ID` (→ lot) | 20 | **20** | `LOT-0000` |
| `DE1-NoticeResult-LotTender-TenderLot-ID` (→ lot) | 18 | **18** | `LOT-0000` |
| `DE1-NoticeResult-TenderingParty-Tenderer-ID` (→ org) | 14 | **14** | `ORG-0001` |
| `DE1-NoticeResult-LotResult-LotTender-ID` (OPT-320) | 18 | **0** | `TEN-0000` |
| `DE1-NoticeResult-LotTender-TenderingParty-ID` (OPT-310) | 18 | **0** | `TPA-0000` |
| `DE1-NoticeResult-LotResult-SettledContract-ID` (OPT-315) | 18 | **0** | `CON-0000` |
| `DE1-NoticeResult-SettledContract-LotTender-ID` (BT-3202) | 18 | **0** | `TEN-0000` |
| `DE1-NoticeResult-LotResult-ID` (own id) | 20 | 0 | `RES-0000` |
| `DE1-NoticeResult-LotTender-ID` (own id) | 18 | 0 | `TEN-0000` |

**References to lots and organizations resolve; every result-to-result reference resolves to nothing.**

Issue 75 gave the result nodes (`LotResult`, `LotTender`, `SettledContract`, `TenderingParty`)
**synthetic** section ids, because their published `RES-`/`TEN-`/`CON-`/`TPA-` ids repeat across the DÖE
serializer's grafted positions (issue 78) and would collide; lots and organizations kept their published
ids, which is why those two resolve. But the *references* still carry the published ids. So in
`read_results` (project.rs):

```rust
("OPT-320", NoticeValue::Id { value, .. }) => r.bid_refs.push(value.clone()),   // "TEN-0000"
...
let Some(b) = raw.bids.iter_mut().find(|b| b.key == owner)                      // key is "LotTender#0"
```

`r.bid_refs` never matches a bid key, `b.party_ref` never matches a tendering party, and the winner
chain **LotResult →(OPT-320)→ LotTender →(OPT-310)→ TenderingParty → Tenderer** breaks at every hop.

## Why issue 98 does not fix this

98 flags organization references so the role arm sees them. `read_results` matches
`NoticeValue::Id { value, .. }` and **never tests `is_ref`**, so the results graph was never gated on
that flag — it is gated on section-id identity, which 98 does not touch. After 98 the cohort gains
buyers and every lot role; **winners stay at 0% from DE**. Confirmed before folding rather than after,
which is the point.

## Fix direction (not costed yet)

Result sections need identity that matches what the references name: **the published id where it is
unique within the notice, disambiguated only where it genuinely collides.** The naive "just key on the
published id" is wrong — it reintroduces exactly the collisions issue 75 measured, which is why the ids
were synthesised in the first place.

This is a **parse-layer** change: `notice_sections.section_id` is written at parse time. So unlike 98 it
cannot ride a re-fold — it needs the cohort **re-parsed from the archive** (the issue-76 reprocess
mechanism), then re-folded. Materially more expensive than 98/99, hence its own issue and its own batch.

**Open question to settle first, and it decides the whole design:** are the published result ids unique
*within a single notice*, or do they collide there too? Issue 75 recorded them repeating "across grafted
positions"; whether that means *within one notice* or *across notices* is the difference between a
one-line fix and a real disambiguation scheme. **I attempted this measurement and it did not complete**
(the query was starved by contention on the box) — so it is recorded here as unmeasured, not as known.
Measure it before designing:

```sql
SELECT ni.field_id, COUNT(*) AS vals,
       COUNT(*) - COUNT(DISTINCT ni.notice_id || '/' || ni.value) AS duplicates_within_a_notice
  FROM notice_ids ni
 WHERE ni.notice_id BETWEEN <a> AND <b>
   AND ni.field_id IN ('DE1-NoticeResult-LotResult-ID','DE1-NoticeResult-LotTender-ID',
                       'DE1-NoticeResult-SettledContract-ID','DE1-NoticeResult-TenderingParty-ID')
 GROUP BY ni.field_id;   -- duplicates = 0 → published ids are safe as section ids
```

## Verification this needs

The suite gained party provenance for 98 (`C11`/`C12`/`H7`: `mention_notice_id` = the DE notice, not a
merged twin). Winners need the same shape, because a DE version can show a winner inherited from its TED
twin — that is exactly what the 16 of 247 are:

```sql
-- winners EVIDENCED BY the DE notice itself; pre-fix this is 0
SELECT COUNT(*) FROM notices n
  JOIN tender_versions v ON v.caused_by_notice_id = n.id
  JOIN tender_version_result_winners w ON w.tender_id = v.tender_id AND w.seq = v.seq
  JOIN organization_mentions om ON om.organization_id = w.organization_id AND om.notice_id = n.id
 WHERE n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2');
```

## Scope decision (team-lead, 2026-08-02)

Winners are **out of scope for the 98+99 fold**. That fold lands buyers, every lot role and the 2,185
skipped shells; the DE cohort goes live materially better than today but **without award winners**. The
ledger/dashboard wording for that gap is the user's call and is deliberately not touched here.

## Note

Third defect traceable to the issue-75 empirical inventory, after the `id` vs `id-ref` typing and the
missing role aliases (both issue 98). The common root is that an empirical inventory derived from
observed XML can recover *structure* but not *semantics* — which ids are references, and which ids are
identities the references will name. Both need the SDK's own model, or an explicit check against it. That
lesson belongs in the DE-2.x work and any future dialect vendoring, not just here.

## Parse-layer fix LANDED & DEPLOYED (2026-08-17, owner — rev `2df2703`)

The cheap arm the measurement licensed is in: `identifierFieldId` wired on the four NoticeResult
**definition** nodes (`ND-LotResult`, `ND-LotTender`, `ND-SettledContract`, `ND-TenderingParty` at
their top-level `efac:NoticeResult` positions) → their own `DE1-…-ID` fields, in
`crates/ingest/sdk/fields-de-1.x.json`. Section ids are now the published `RES-`/`TEN-`/`CON-`/`TPA-`
ids, which is the vocabulary the references already spoke.

**A structural distinction the issue had not recorded, and it shaped the edit.** The inventory holds
SEVEN nodes across these kinds, not four: `ND-LotTender` appears three times and `ND-SettledContract`
twice, at different grafted xpaths. Only the top-level ones are definitions; the nested
`LotResult/LotTender`, `LotResult/SettledContract` and `SettledContract/LotTender` positions are
**reference holders** — in eForms `efac:LotResult/efac:LotTender/cbc:ID` IS OPT-320, a pointer whose
`cbc:ID` names its target. Wiring those would give the pointer and the definition the same section
id, the exact collision `index.rs` already documents for `ND-ContractingParty`. The test is checkable,
not a judgement call: a definition is the position whose own-ID field sits at exactly
`node + /cbc:ID`, true for these four and no others. So "just key on the published id" was right for
the definitions and would have been WRONG applied uniformly — closer to issue 75's caution than the
one-line framing suggested.

Red-checked both directions (revert the inventory → the test fails naming `ND-LotResult#0`; restore →
passes). 11/11 doe, 46 eforms, golden + data-quality + process green. The inventory's `$comment` now
records the delta and warns that `de1x_gen.py` on the box does not reproduce it — regenerating would
silently revert a data-correctness fix.

### Next unit: the cohort reprocess — sized, and its mechanism is an OPEN QUESTION

Cohort measured via `/v1/sql` today: **218,876 notices** — `eforms-de-1.0` 31, `1.1` 145,859, `1.2`
72,986. (Exactly the inventory's scan count, so the profile labels and the 2023-10..2026-07 scan
agree.)

`section_id` is written at PARSE time, so nothing above reaches the canonical layer until these are
re-parsed. **The mechanism is not settled and must not be improvised:** the `reprocess` job kind
re-attempts QUARANTINED rows by reason/detail/profile, but this cohort is already `parse_state =
'parsed'` — there is no reason bucket to name, and `refold`/`refold-fields` only clear the projected
watermark and re-fold the canonical layer from the EXISTING parse rows, which is precisely what
cannot help here. A `process` re-walk dedupes by content hash and would skip. So one of:

1. a new job kind (re-parse by profile, walking the archive for already-parsed members), or
2. an extension of `reprocess` to accept a profile-only selector against parsed rows, or
3. confirmation that some existing path already re-parses (checked the enqueue arms today and found
   none — worth a second reader before building anything).

Settle that first, then: re-parse → fold → verify. Verification shape is fixed by issue 98's
precedent (`C11`/`C12`/`H7`): a winner must carry `mention_notice_id` = the DE notice itself, never a
merged TED twin, or the 2%-inherited-winner reading repeats with a bigger number. Do NOT run the
re-parse before the daily 09:35 window is clear, and expect it to be long — 218,876 members through
dispatch+parse.

### Re-parse mechanism: SETTLED (2026-08-17, owner — groundwork deployed rev `ff9795047`)

The open question above is answered from the code, and the answer is narrower than "build a new
subsystem". Ruled out by reading, not assumption:

- `reclaim_notice`'s already-parsed arm returns `AlreadyParsed` **by design** — it is what makes a
  reclaim re-run a no-op, load-bearing for the campaigns, so it must not gain a force flag;
- `run_reprocess` walks `quarantine_reclaim_packages`, so it can only ever see HELD rows;
- `refold` / `refold-fields` re-fold from the EXISTING parse rows — exactly what cannot help;
- a `process` re-walk dedupes by content hash and skips.

But the in-place parse write already existed for the quarantined arm (`insert_parsed` + the
`parse_state`/`projected` update). Only the reach was missing — **and one safety property**.

**The hazard, which is why this got its own commit:** `insert_parsed` is pure `INSERT`. Forcing it on
an already-parsed notice DOUBLES every section/text/code/date/amount/number/integer/id row instead of
replacing them — it would have looked like a working feature while corrupting the parse layer. So
`Db::reparse_notice` clears first (`clear_parsed` over `PARSED_TABLES`), then inserts, then sets
`parse_state='parsed', projected=0`, in ONE transaction: a crash leaves the old layer whole, and the
notice never leaves `parsed`, so no window exists where it is absent from the corpus. Red-checked by
deleting the clear — 2 rows where 1 is required. A second test reads `sqlite_master` and asserts
`PARSED_TABLES` equals the schema's `notice_*` tables, so a future table added to `insert_parsed`
fails the suite rather than orphaning rows on every re-parse.

**Remaining for this issue**, in order:
1. A job kind selecting the cohort by profile and walking its archive packages through
   `reparse_notice` — the `run_reprocess` shape (per-package member sets, checkpoint per package,
   resumable) but with the work list from `notices WHERE profile IN (…)` instead of `quarantine`.
2. The fold: re-parsed notices land `projected=0`, so a following incremental projection picks them
   up — no `refold` needed, and `reclaim_only` semantics apply if the run is bulk.
3. Verification, shape fixed by issue 98's precedent (`C11`/`C12`/`H7`): a winner must carry
   `mention_notice_id` = the DE notice itself, never a merged TED twin, or the 2%-inherited reading
   simply repeats with a bigger number. Also re-check the award-linkage ratio for the DE profiles on
   the dashboard, which is where the 2% is visible today.

### The re-parse job is BUILT but NOT USABLE yet — three structural facts found by running it

Ran `reparse` on the smallest slice of the cohort first (`eforms-de-1.0`, 31 notices) rather than all
218,876. It failed, twice, and the failures are the useful part. **Do not fire `reparse` expecting it
to work** — it errors safely (the transaction rolls back, the parsed layer is untouched, the paired
projection finds nothing) but it cannot currently complete on any notice that has been folded.

**1. The parsed layer is pinned by the canonical layer, two levels deep.** `organization_mentions`
carries `FOREIGN KEY (notice_id, section_id) REFERENCES notice_sections` — the only FK into the parsed
layer, and enough to break the first run. Clearing the notice's mentions first (committed, tested,
red-checked against prod's exact error) fixed that and exposed the next level:
`tender_version_parties` AND `tender_version_bid_parties` carry `FOREIGN KEY (mention_notice_id,
mention_section_id) REFERENCES organization_mentions(notice_id, section_id)`. So the chain is
**tender_version_parties → organization_mentions → notice_sections**, and a notice's parsed layer
cannot be replaced while any tender version cites the organizations it mentioned. That is ADR-0001
traceability working as designed — a tender party points back at the exact mention it came from — not
a bug to route around.

**2. Even with the deletes ordered correctly, the fold would silently skip the work.**
`apply_tender_tx` keys its early return on the sequence of causing notice ids, NOT on content. A
re-parse changes a notice's CONTENT while leaving its chain identical, so every affected tender hits
`keep == stored.len() == p.versions.len()` and early-returns — new parse rows sitting in the corpus,
canonical layer unchanged, and (because `delete_version` is what removes party satellites) the
citations left dangling. This is issue 99's lesson arriving from a new direction: a projection-logic
or parse-content change needs a `PROJECTION_EPOCH` bump to force rewrites. Without one, the entire
re-parse would have reported success and produced nothing — the exact "accurate about what it
measured, misleading about what it is read as" shape issue 108 catalogues.

**3. So the remaining work is bigger than a job kind, and it is a decision, not a task.** Completing
issue 100 needs all three: (a) `clear_parsed` extended to the citing `tender_version_parties` /
`tender_version_bid_parties` rows; (b) a `PROJECTION_EPOCH` bump — which per issue 99's own note
"declares 7.9M tenders stale to fix one era", i.e. a multi-hour full refold, not a DE-1.x-sized one;
(c) the cohort re-parse itself. The cheaper alternative is ADR-0009's shape: re-parse with
`reclaim_only`, then ONE `project --rebuild`, which clears canonical and DROPs the org tables anyway
— removing the FK obstacle wholesale and making (a) unnecessary. That trades a targeted operation for
a full rebuild, and it is the pattern every prior parse-layer cohort change used (71/72/75/78/85/139/
195).

**Recommendation for whoever picks this up:** take the ADR-0009 route (re-parse `reclaim_only` → full
rebuild) and schedule it as a planned rebuild window, not an hourly-firing task. Verify afterwards per
issue 98's `C11`/`C12`/`H7` precedent — a winner must carry `mention_notice_id` = the DE notice, never
a merged TED twin — plus the dashboard's DE award-linkage ratio, which reads 98% unchained today.

### (a) is DONE — and the board had already said what it was (2026-08-18, owner, rev `f542229`)

`clear_parsed` now clears the citing `tender_version_parties` / `tender_version_bid_parties` rows
before the mentions, with deferred indexes on `(mention_notice_id)` for both tables, a test that
reproduces the failure and a second notice in that test proving the delete stays scoped.

**The honest part first: this was already written down here, in the section directly above, on
2026-08-17** — "(a) `clear_parsed` extended to the citing `tender_version_parties` /
`tender_version_bid_parties` rows". I ran the re-parse job anyway (jobs 721, 733, 735), watched it fail
three times with `immediate foreign key constraint failed`, blamed the mentions/sections ordering,
"fixed" that, ran it again, added statement-level logging, deployed, ran it again, and read the log to
learn what this issue's own text said before I started. The commit message calls the cause a grep that
was too narrow; that is true but not the whole truth. The section titled "The re-parse job is BUILT but
NOT USABLE yet" was exactly the thing to read before running the job, and I did not.

Lesson recorded for the next person, including me: when an issue has a "not usable yet / open
question" section, that section is the pre-flight checklist. Three failed prod jobs and two deploys
bought information that was already on the board.

The statement-level logging is worth keeping regardless — it named the failing statement on its first
run, and the next FK surprise in this path will not need three attempts.

### What (a) changes about the recommendation

The section above recommends the ADR-0009 route — re-parse `reclaim_only`, then ONE `project
--rebuild` — precisely BECAUSE a rebuild DROPs the org tables and so "remov[es] the FK obstacle
wholesale, making (a) unnecessary". With (a) done, that trade is no longer forced: a targeted re-parse
can now clear its own citations without a corpus-wide rebuild. That matters well beyond this issue —
the text-era buyer fix (issue 232) faces the same choice over 3.79M notices, where a full rebuild is
far dearer than for DE-1.x.

**But (b) is still open, and it is what stops a targeted re-parse from landing.** `reparse_notice`
sets `projected = 0`, so the notice enters the incremental change-set — and per point 2 above the fold
then EARLY-RETURNS on an unchanged chain with a current epoch, producing nothing while reporting
success. The global `PROJECTION_EPOCH` bump is one answer and a multi-hour one (issue 99: "declares
7.9M tenders stale to fix one era").

The cheaper answer already exists in the codebase and simply is not wired to `reparse`:
`stamp_stale_for_notices` — the by-ids twin the `refold` jobs use for exactly this reason (issues
85/99/179). A `reparse` that stamped its own cohort's tenders epoch-stale would need no global bump
and no rebuild. That is the next unit.

### (b) DONE and the mechanism VERIFIED END TO END on prod (2026-08-18, rev `689feb6`)

`run_reparse` now stamps its cohort's tenders epoch-stale (`stamp_stale_for_profiles`), so the fold no
longer early-returns on an unchanged chain. Both structural blockers are closed and both were verified
on the smallest cohort rather than argued:

    737 reparse -> ok: re-parsed 31 notices across 5 packages (92839 members walked, 0 unmatched,
                       0 now failing and left untouched); stamped 22 tender(s) epoch-stale
    738 project -> ok: 31 notices → 22 tenders (1 islands), 106 versions; 22 tenders written,
                       0 verified unchanged

Both predictions recorded in the commit held: a non-zero stamp, and a projection writing MORE than the
0 tenders job 734 wrote. `22 written, 0 verified unchanged` means every one actually rewrote — which is
precisely what the missing epoch stamp had been preventing. **This is the first successful targeted
re-parse in the project**, and it needed no full rebuild, so the ADR-0009 route recorded above is now
optional rather than forced.

### A correction, before anyone reads the winner numbers as success

Checking the re-folded cohort I found 22 rows in `tender_version_result_winners` across 14 of the 22
tenders, where the data-quality report reads `eforms-de-1.0: winner 0.0%`. I nearly recorded that as
"the re-parse resolved winners". **It is not.** Tracing each winner to its origin notice's profile:

    eforms:eforms-sdk-1.7    18
    eforms:eforms-de-2.0      2
    eforms:eforms-sdk-1.10    1
    eforms:eforms-sdk-1.12    1

**Not one winner cites a DE-1.0 notice.** They come from the EU/TED twin notices merged into the same
procedure, and they were almost certainly there before this re-parse. So these 22 tenders have winners
*despite* DE-1.0, not *because of* it — the opposite of the C11/C12 criterion issue 98 sets ("a winner
must carry the DE notice, never a merged TED twin"). The number was measured on the Tenders rather than
on the notices, which is the wrong denominator, and it flattered the result.

What this does and does not establish:
- **Established:** the re-parse MECHANISM works — parse layer replaced, cohort aged, fold rewrote all 22.
- **NOT established:** that the DE-1.x parse-layer fix produces winners. DE-1.0 is 31 versions of an
  early dialect whose awards were mostly published later in other profiles, so it may be the wrong
  cohort to answer that at all.

**Next, and in this order:** count `lot_results` and winners whose ORIGIN notice is DE-1.x (not whose
Tender happens to contain one) — the query for that is what tripped the /v1/sql reader stall filed as
issue 238, so it wants retrying on an idle box. If DE-1.0 genuinely publishes no awards, move to
`eforms-de-1.1` (145,720 versions, 65,207 awards per the data-quality report) and size the package walk
first: DE-1.0's 31 notices spanned 5 packages and 92,839 members in ~35 minutes, so the 1.1 cohort is
hours and belongs in a planned window with the daily's 09:35 slot kept clear.

#### Partial evidence on whether DE-1.0 publishes awards at all (2026-08-18)

Section kinds across the 31 DE-1.0 notices' parse layer, top 12 by count:

    ContractExecutionRequirement 142   TendererQualificationRequest 123
    SelectionCriteria            135   AdditionalCommodityClassification 95
    SpecificTendererRequirement  130   SubordinateAwardingCriterion  89
    RealizedLocation             124   AwardCriterionParameter       89
    ContractingSystem             88   PartyName / PartyLegalEntity / Organization 64

**No results-layer kind appears** — no `LotResult`, `LotTender`, `TenderingParty` or `SettledContract`
— and the list runs down to 64, well below where a CAN's handful of result sections would sit. Strongly
suggestive that DE-1.0 is a contract-notice-only cohort, which would make its `winner 0.0%`
**correct-by-source** and confirm it is the wrong cohort to prove the DE-1.x winner fix with.

Not yet conclusive: the confirming query (`… AND kind IN ('LotResult',…)`) could not be completed.
Two separate obstacles, both worth knowing:
- Adding the `kind` predicate flips the planner off the `notice_sections` PK seek onto
  `notice_sections_kind` (millions of rows), so it times out on cost — an index-choice inversion, and
  the same shape as the `lot_results` `notice_id IN (…)` subquery, which cannot use its
  `(tender_id, notice_id, result_key)` index at all.
- The endpoint then began failing the very query that had just succeeded, which is issue **238**
  (a reader-acquisition stall reported as a query timeout), not a property of the query.

**Issue 238 is therefore blocking this verification**, which raises its priority: a misleading
diagnostic is now costing a data investigation, not just an operator's patience. Retry this on an idle
box once 238's acquisition timing is separated, and drive the confirming count from `notice_sections`
by notice id WITHOUT a `kind` predicate (filter in the client) so the planner cannot invert.

#### SETTLED: DE-1.0 cannot answer the winner question, because it publishes no winner graph

The count that issue 238 was blocking now runs (in 3–5 ms, on the fixed endpoint). Across all 31
DE-1.0 notices, the complete section-kind census:

    LotResult                      1     ReceivedSubmissionsStatistics    3
    LotTender                      0     TenderingParty                   0
    SettledContract                0     Change                          13
    ContractExecutionRequirement 142     SelectionCriteria              135
    (…21 further contract-notice kinds…)

**One LotResult section in the entire era, and not a single `LotTender`, `TenderingParty` or
`SettledContract`.** The winner chain is `LotResult → SettledContract → LotTender → TenderingParty →
Organization`, so with zero of the middle three links, **no winner can be resolved from a DE-1.0 notice
by any parser.** `winner 0.0%` for this era is correct-by-source, not a defect.

This also reconciles exactly with section 3 of the data-quality report, which reads
`eforms-de-1.0: 1 award-notice, 1 with lot_results, 100.0%` — one award notice, its result
materialised.

So the earlier reading ("suggestive that DE-1.0 is contract-notice-only") is now settled, and the
caveat attached to it was right to hold: the top-12-by-count view had hidden `LotResult` at 1, below
its cutoff. A census beats a top-N whenever the interesting value is a small one.

**Consequence for this issue:** DE-1.0 was a bad choice of verification cohort — it proved the
re-parse MECHANISM (which was the point, and worth it) but is structurally incapable of proving the
winner fix. The cohort that can is `eforms-de-1.1`: 145,720 versions and 65,207 awards per the report.
Before running it, size the walk — DE-1.0's 31 notices spanned 5 packages and 92,839 members in ~35
minutes, so 1.1 is hours, and it belongs in a planned window with the 09:35 daily kept clear. Run the
same census on a DE-1.1 sample FIRST: if 1.1 also lacks `TenderingParty`/`LotTender` sections, the
whole premise of this issue needs re-examining before another cohort is re-parsed.

#### DE-1.1 census + baseline: this issue's premise needs re-examining before any cohort re-parse

Sampled both ends of the era (300 notices each, `ORDER BY id` ASC and DESC over 145,859 DE-1.1
notices), which is what the note above asked for.

**Parse layer — the full winner chain is present, unlike DE-1.0:**

    sections            oldest 300      newest 300
    LotResult                  106             297
    LotTender                  146             837
    TenderingParty              73             194
    SettledContract            126             472

Every link of `LotResult → SettledContract → LotTender → TenderingParty → Organization` exists at both
ends. So DE-1.1 CAN answer the winner question, where DE-1.0 structurally cannot.

**Canonical layer — winners are already resolved for most of both samples:**

    oldest 300 notices → 299 tenders: 309 winner rows across 186 tenders (62 %)
    newest 300 notices → 288 tenders: 9,032 winner rows across 220 tenders (76 %)

**That is 62–76 %, against the data-quality report's `eforms-de-1.1: winner 1.4%`.** Both numbers are
correct; they measure different things. The report's `winner` FieldSpec counts VERSIONS carrying a
winner row, and DE-1.1 tenders have many versions — contract notices and corrigenda (the same samples
carry `Change` sections) legitimately have no winner. So per-version presence is ~1 %, while per-tender
presence is ~70 %.

**This is the third time this exact denominator has misled on this board** (see the caveats added to
issues 231 and 232, and the winner correction earlier in this issue), and it is precisely what issue
235 exists to fix. The report's `winner` column cannot distinguish "winners are lost" from "most
versions are not awards", and should not be cited as evidence of either until 235 lands.

**Consequence — do NOT run the 218,876-notice re-parse yet.** Its justification was "DE-1.x award
winners unresolved", and the evidence for that was the 1.4 %, which measures something else. What is
established: the chain is present in the parse layer, and most sampled tenders already carry winners.
What is NOT established: how many award RESULTS lack a winner they should have. That is the number
that justifies or cancels hours of re-parsing, and it needs the per-`lot_result` denominator:

    of lot_results whose origin notice is DE-1.x, what share have ≥1 tender_version_result_winners row?

Measuring it is not free — `lot_results` has no index leading with `notice_id`
(`UNIQUE(tender_id, notice_id, result_key)`), so it must be driven from a bounded tender-id list. My
attempt at it is what saturated the SQL runtime and took `/v1/sql` down for every user (issue 238), so
run it in small batches (≤50 tender ids) and check the shape's cost on one batch before scaling.
