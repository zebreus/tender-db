# 236 — eForms EU awards chain to their contract notice at 44–77 %, where eForms-DE manages 98–100 %

Status: IMPLEMENTED and deployed 2026-08-19 (`537620e`), verified on prod against a real pair — the
cross-era hypothesis below is FALSIFIED; the cause is BT-04 that is not stable across a procedure's
notices, and `OPP-090-Procedure` (ADR-0011) is the published repair for ~12 % of the cohort. Remaining:
the corpus-wide repair needs the rebuild (bundle with issues 100/232/235).
Kind: identity/chaining gap, suspected cross-era boundary
Blocked by: —
Relates to: 187 (INTERNAL_OJS 100 % unchained), 188 (sdk-0.1 98 % unchained), 58 (legacy OJS closure —
the chaining machinery for the eras on the other side of the suspected boundary), 12 (procedure keys),
230 (the measurement)

## What

Section 2 of the first complete data-quality report, award Tenders chained back to a contract notice:

    era                                awards  unchained   linked
    eforms-de-1.1                      65,207         18   100.0%
    eforms-de-1.2                      29,364         70    99.8%
    eforms-de-2.0                      33,658        502    98.5%
    eforms-de-2.1                      37,151        818    97.8%
    ---
    eforms-sdk-1.10                    66,748     17,045    74.5%
    eforms-sdk-1.11                    39,597     13,495    65.9%
    eforms-sdk-1.12                    94,251     37,382    60.3%
    eforms-sdk-1.13                   172,450     60,831    64.7%
    eforms-sdk-1.14                    15,129      8,407    44.4%
    eforms-sdk-1.7                     89,750     29,460    67.2%
    eforms-sdk-1.8                     61,732     20,289    67.1%
    eforms-sdk-1.9                     36,243     13,720    62.1%
    eforms-sdk-1.6                     18,136      4,269    76.5%
    eforms-sdk-1.3                      2,190        616    71.9%
    eforms-sdk-1.0                        713        711     0.3%

Both families are eForms. Both key on BT-04. Yet the German customization chains essentially
everything and the EU profile loses a third to a half — **roughly 246,000 unchained award Tenders**
across the sdk-1.x eras. That is not a small tail, and it is not the two known island cases (issues
187 and 188 cover internal-ojs and sdk-0.1, both visible in the same table at 3.5 % and 2.0 %).

## The leading hypothesis, and how to kill or confirm it

**A cross-era boundary rather than an eForms defect.** eForms became mandatory for EU publication in
late 2023, so a 2024–2026 eForms CAN routinely belongs to a procedure whose CONTRACT NOTICE was
published earlier as TED r2.0.9. The legacy notice carries no BT-04, so the eForms CAN has nothing to
chain to on the eForms key — the CN is reachable only through the legacy OJS closure, which keys
differently. eForms-DE would not show this because DÖE's German channel started publishing eForms-DE
CNs and CANs together, so both ends of a DE procedure are usually in the same vocabulary.

Two predictions that make this falsifiable:
1. **Unchained share should fall as the SDK version rises** — later SDKs mean later notices, and a
   later CAN is more likely to have an eForms CN behind it. The table does NOT obviously show that
   (1.14 at 44.4 % is the WORST, 1.6 at 76.5 % among the best), so either the hypothesis is wrong or
   SDK version is a poor proxy for date. Check publication dates directly before believing either.
2. **An unchained eForms CAN should have a findable r2.0.9 CN** for the same procedure — same buyer,
   same CPV, an OJS reference in its own text. Pull ten unchained CANs from sdk-1.13 and look.

If it holds, the fix is a cross-vocabulary bridge (eForms CAN → legacy CN), which is a real design
question, not a mapping tweak — and it should be an ADR, since it decides whether one procedure spanning
two publication regimes is one Tender.

If it does not hold, the cause is inside eForms and the next thing to check is whether these CANs carry
BT-04 at all (a missing key looks identical to an unmatched one in this metric).

## Note on `eforms-sdk-1.0` at 0.3 %

713 awards, 711 unchained. Tiny, and almost certainly its own story — sdk-1.0 is an early-adopter
trickle. Worth a glance while investigating the rest, not worth its own issue yet.

## Acceptance

- The publication-date distribution of unchained vs chained eForms EU award Tenders, recorded here —
  that alone confirms or kills the cross-era hypothesis.
- For a sample of ten unchained CANs: does the procedure have an earlier legacy CN in the corpus, and
  does the CAN carry a BT-04?
- A recorded decision: bridge the vocabularies (ADR), or accept the split with the reason stated in the
  report's own text so the number stops looking like an unexplained defect.

## Diagnosis (2026-08-19) — measured against prod, and the hypothesis above is wrong

### Prediction 1 (unchained share falls as eForms matures): FALSIFIED

Award-bearing Tenders whose FIRST version is an EU eForms notice (`eforms:eforms-sdk-1.%`), by the
version's publication month — the same shape as `LINKAGE_SQL`, windowed on the indexed `published_at`:

    month      award tenders   unchained   unchained %
    2024-01          16,334       6,439       39.4 %
    2024-07          24,696       7,341       29.7 %
    2025-01          16,891       6,492       38.4 %
    2025-07          25,242       6,781       26.9 %

Flat, with a seasonal wobble (July has more awards and a lower share). A cross-era boundary would decay
sharply — 2024 notices inheriting legacy contract notices, 2025 ones mostly not. It does not decay. The
legacy-vocabulary story is not what is happening.

**Measurement trap worth recording:** `LIKE 'eforms:eforms-sdk-%'` also matches `eforms-sdk-0.1`, which
is the DÖE island (issue 188), not an EU era. My first pass did that and inflated the unchained counts by
1,688–1,872 per month — every one of them a `notice_subtype IS NULL` row, which is what exposed the
mistake. The report itself groups by exact profile and is unaffected; ad-hoc probes are not. Use
`sdk-1.%`.

### What the unchained awards actually are

Subtype mix of the unchained cohort (2025-01, `v.notice_subtype`, taxonomy from
`docs/research/eforms-data-model.md` §4):

    29 (result)            4,637     ← the block that matters
    30 (result)              715
    38 (cont-modif)          427     expected: a modification has no CN of its own
    25 (dir-awa-pre/VEAT)    203     expected: a direct award has no call for competition
    33 (result)              197
    31 (result)               92
    39 (cont-modif)           71
    36, 32, 26, 34, …        <60 each

VEAT (25–28) and modification (38–40) notices are legitimately chain-less, but together they are under
10 % of the cohort. The mass is genuine result notices.

### The mechanism, from the bytes

Notice 24506261 (`sdk-1.13`, published 2025-01, subtype 29, single-version Tender, has results):

    BT-04-notice        00a143ab-195e-4337-8d8c-bfe87c79be9e   ← its own procedure key
    OPP-090-Procedure   615938-2024                            ← "previous notice", ND-PreviousNoticeReference#0

`615938-2024` normalises to publication id `00615938-2024`, and **that notice is in the corpus**: notice
24324996, `eforms:eforms-sdk-1.7`, source `ted`. Both ends are eForms. They are two Tenders solely
because their BT-04 values differ.

So the cause is not a vocabulary boundary and not a missing key: **BT-04 is not stable across the notices
of one EU procedure.** eForms-DE chains at 98–100 % because DÖE keeps it stable; EU eSenders frequently
mint a fresh UUID for the award notice. The link is still published — as an explicit previous-notice
reference — and the projection does not read it.

### How much the published reference would repair (2025-01, unchained subtype-29)

    unchained subtype-29 award tenders                     4,641
    ...carrying OPP-090-Procedure at all                   1,347   (29 %)
    ...whose reference resolves to a notice we hold          563   (12 %)
    distinct referenced tenders                              527
    ...of which themselves single-version                    373   ← orphaned CNs, de-orphaned by the same edge

So one indexed lookup per reference would merge 563 award Tenders into 527 existing ones and simultaneously
repair 373 CN-only Tenders — both halves of the same procedure, sitting in the corpus unlinked. Roughly
12 % of the cohort per month; extrapolating the four months above, order 10⁴ Tenders corpus-wide.

The other 71 % carry no previous-notice reference at all. For those the corpus holds no published link,
and recovering them would mean matching on buyer + CPV + value + dates — inference, which does not belong
in the identity layer (the same line drawn in issue 237's synthetic-group decision and issue 234's
provisional organizations).

## The decision this now needs (ADR)

`OPP-090-Procedure` is a **publisher-declared** statement that two publications belong to one procedure.
Using it as a union-find edge is therefore not inference — it is reading a field we already parse. But it
is still an identity change with three properties worth an ADR rather than a patch:

1. **It can merge Tenders that BT-04 says are distinct.** ADR-0003 already settles cross-source merging
   on a shared key; this is a different warrant (a reference, not a key) and the precedence rules need
   stating — including what happens when A references B and B carries a different BT-04 that matches C.
2. **A wrong reference merges two unrelated procedures**, which is worse than leaving them apart. Needs a
   guard: resolve only within `source = 'ted'`, only to a notice that exists, and (proposal) only when
   the referenced notice's publication date precedes the referrer's.
3. **It needs a corpus-wide re-projection to take effect**, so it should be bundled with the other
   pending re-projections (issues 100, 232, 235) rather than run on its own.

Implementation note for whoever takes it: the edge belongs where the legacy `REF_NOTICE` edges already
live (`project.rs`, `Ident`'s `ojs_edges` are the precedent — a non-BT-04 chaining edge feeding the same
union-find), and the value needs the 8-digit zero-pad normalisation shown above, since eForms writes
`615938-2024` where `notices.publication_id` holds `00615938-2024`.

## Still open from the original acceptance list

- The date distribution is recorded above (four month points, not a full histogram — enough to falsify
  the hypothesis, and cheap to extend if a fuller picture is ever wanted).
- The ten-CAN sample became one CAN traced end to end plus the 4,641-row aggregate behind it, which is
  strictly more than the sample asked for. Both carry BT-04, so "does the CAN carry BT-04 at all" is
  answered: yes, always — the earlier sample of 300 `sdk-1.13` notices found 1 island in 300.
- `eforms-sdk-1.0` at 0.3 % is still unexamined.

### Guards measured for ADR-0011 (2026-08-19)

On the 563 resolvable edges of the measured month:

    reference points at an EARLIER publication      563 / 563
    ...at the same instant or later                   0
    ...at a notice with no projected version          0
    notices carrying 1 / 2 / 3 references      1,340 / 2 / 1

So "must resolve, within TED, to an earlier notice" costs nothing on today's data, and the edge has to
accept a SET of references rather than one value. Both are written into ADR-0011, which also records why
the edge must join procedure-key components rather than pull eForms notices into the legacy OJS key space:
their publication ids parse as `(year, number)` and would fit `ojs_self`/`ojs_edges` with no new
machinery, but the legacy component is keyed by MIN OJS, so eForms Tender identity would stop being BT-04
— reissuing ids corpus-wide to fix a 12 % gap.

## Implemented and verified on prod (2026-08-19, rev `537620e`)

Three commits: `62b9021` reads `OPP-090-Procedure` into `Ident`/`PlanRow` with the publication-id
normalisation; `51c453e` adds the real fixture pair and pins the parse-layer claim; `537620e` groups on
the edge.

**Where the pass sits and why.** After the keyed and legacy passes (it unions COMPONENTS, so every
notice must already carry a `group_key`) and before `plan_notice_fold` is created (so the relabel pays no
index maintenance). Resolution goes reference → `notices` UNIQUE(source, publication_id, …) → `plan_notice`
by primary key, which is why it needs **no new index** on a 22M-row plan table — the first design that
came to mind wanted `plan_notice(publication_id)` and would have cost an extra full index build.

**The representative is the earliest-published key**, reusing `MinUnionFind` by unioning over positions in
earliest-first order instead of writing a second union-find. Same rule as the legacy closure's `MIN(ojs)`:
a procedure is identified by its first appearance.

**Verified on production data, not only in a test.** `refold-notices [24324996, 24506261]` (job 749) then
the paired projection (750) reported `2 notices → 1 tenders (0 islands), 2 versions`, and the corpus now
holds:

    tender 2832   (was the award's island, key 00a143ab-…)   GONE — absorbed and retired
    tender 138780 (the CN's, key 1d76f173-…)                 current_seq 2
      seq 1  00615938-2024  2024-10-10   the contract notice
      seq 2  00000566-2025  2025-01-01   the award, its lot_result attached

That exercises the retire path as well, which the unit test cannot: the award's old Tender id is gone, so
a consumer holding it must follow the `removed` event (issue 46's protocol).

### What is still open, precisely

- **The corpus repair.** Only this one pair is merged. The incremental plan holds just the touched
  notices, so a daily run resolves an edge only when BOTH ends are in that plan — otherwise it unions
  nothing and the award stays an island. That is deliberate (a late merge beats a partial one), and it
  means the ~10⁴ merges land with the bundled rebuild. `refold-notices` is the manual lever meanwhile,
  as used above.
- **Scope expansion for the incremental path** would make dailies merge immediately: when a new notice
  references publication P, P's Tender's notices must join the touched set. Issue 58 v2 built exactly this
  shape for legacy adjacency (`legacy_ojs_keys` + the closure walk, task #32), so the pattern exists to
  copy. Not started; it is the natural next unit and worth its own issue if it does not get picked up here.
- **The expected Tender-count drop** when the rebuild lands (one Tender per merge) wants a ledger row, or
  it reads as data loss.
