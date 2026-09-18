# 343 — a version with several tender-level titles: the fold's `current_title` and the read-time pick break the tie differently

Status: REOPENED 2026-09-15 — the deployed tie-break is deterministic but elects the WRONG string: award-block (RES-n) contract titles are still filed at tender scope on prod at `9e082fd`, so the ladder serves a contract title as the Tender title (incomplete fix, not a regression of `9f0bcea`).
Previous status, kept as history: FIXED 2026-09-02, DEPLOYED 2026-09-03 (`9f0bcea`). Read side verified on prod; the materialised `current_title` of heads written by the pre-fix fold (612) keeps the old tie-break until refolded — see the probe note at the end.
The mechanism was exact, not "scan order vs precedence": see "Why, exactly".
Kind: consistency (read layer vs fold-time head column)
Relates to: 115 (the set-based summary picks and their SQL oracle), 216 (the
materialised head columns), ADR-0013 D3 (`title_rank`)

## Observed

Tender 6287622, head version `seq 2` (a 2019 framework award, r209), carries
THREE tender-level titles in one language:

```
lot_id NULL  ENG  Domestic Courier Services on the Territory of Poland
lot_id NULL  ENG  Framework Contract for Courier Services for Frontex
lot_id NULL  ENG  International Export/Import Courier Services
```

(plus the same two lot titles under their lots). Two surfaces answer "the title"
differently:

| surface | title served |
| --- | --- |
| `tenders.current_title` (fold-time `head_title`, what `v_tenders` and the head sort see) | International Export/Import Courier Services |
| `/v1/tenders/6287622` and the list (read-time `title_rank` pick, `LIMIT 1`) | Domestic Courier Services on the Territory of Poland |

Both rows tie on every rank term (tender-level, ENG); each side breaks the tie by
its own scan order. The read layer's documented tie rule is "first in scan order"
(`summarise`'s doc, pinned by the SQL oracle); `head_title`'s is whatever its
precedence code does. So the same tender shows one title on `/v1/sql`'s
`v_tenders` and another on the REST detail.

## Why it is bounded

It needs a notice that publishes more than one tender-scoped title — r208/r209
framework notices that repeat `TI_TEXT` per lot group at tender scope do; eForms
publishes one BT-21 per procedure. Neither answer is wrong; they are different
members of the same tie. It does not affect matching, filters or the `?lang=`
legs — only which of two equally-ranked strings is shown.

## Fix shape

Make both sides break the tie the same way — the cheapest is to give the fold's
`head_title` the read layer's rule (first by `(tender_id, seq, rowid)` among the
top-ranked rows) or to make the read pick `ORDER BY … , rowid` explicit so scan
order is a stated rule rather than an accident of the plan. Pin with a fixture
carrying two tender-level titles and assert `current_title == list title ==
detail title`. Worth doing beside any other touch of `head_title`; not worth a
deploy of its own.

## Why, exactly (read from the code)

`Fact` derives `Ord` and a version's facts are a `BTreeSet`, so text facts are
written — and scanned — in ascending order of `(field, lang, value)`. The read
pick's `LIMIT 1` therefore returned the alphabetically FIRST of the tied titles.
`head_title` used `max_by_key(is_eng)`, and Rust's `max_by_key` returns the LAST
of equal maxima — the alphabetically last. Two rules, both accidental, opposite.

## Fixed

One stated ladder on every surface: **ENG → the version's original language →
any labelled → unlabelled, then the smallest value among equals.**

* `head_title` (fold-time `current_title`): the ladder with the original leg it
  now has on the version (issue 340), `min_by` rank-then-value.
* `title_rank` (SQL list/detail pick): `…, s.value` appended — the tie rule
  stated instead of inherited from insertion order.
* `summarise` (in-memory lots rank): equal rank → smaller value wins.

Pinned: the three-ENG-title case from this issue in the `head_title` unit test
and in `original_lang_pick.rs` (inserted in reverse order so scan order and
smallest value disagree), on both read paths; the original-leg case for
`head_title` beside it. Red-first: removing the value tie from both read paths
turns the tie test red.

Not changed: which rows match, filters, `?lang=` semantics. A requested language
still outranks everything, as before.

## Probe after the deploy (2026-09-03 09:1x UTC, rev `9f0bcea`) — read side fixed, the materialised column catches up per refold

| surface | title for 6287622 |
| --- | --- |
| `/v1/tenders/6287622` (read-time `title_rank`, new rule) | Domestic Courier Services on the Territory of Poland |
| `tenders.current_title` (fold-time `head_title`) | **International Export/Import Courier Services** — still the OLD rule |

Not a failed fix: `current_title` is written by the fold, and this tender's head
was last written by job 612, which ran on the pre-fix binary (`d416104`,
finished 08:07 UTC; the deploy was 09:03). The fold-side rule is deployed and
applies to every tender the fold touches from now on; the 3.5M heads 612 wrote
keep their old tie-break until something re-folds them — the daily incremental
fold for touched ones, an epoch refold for all. **A refold campaign just for this
is not worth 5.8 hours**; the materialised heads catch up with the next
epoch-stamped campaign (304 stage 2, or whichever comes first), and this issue
stays FIXED-IN-CODE with that residue stated. Only notices with several
tender-level titles (framework r208/r209 shapes) can differ at all.

Both read paths agree with each other now (list, detail, lots all serve the
smallest value among equals), which is the half users touch.

## Comments

### 2026-09-15 — API/data-quality review fan-out: award-block (RES-n) contract titles are filed at tender scope and the ladder elects them — the fix here was incomplete, and this issue's stated premise was wrong

**This is not a regression of `9f0bcea` — the tie-break it deployed still holds (list, detail and `current_title` agree on the rule). What is still live on prod at rev `9e082fd` is the thing this issue declared harmless:** "Why it is bounded" says the extra tender-scope titles come from `TI_TEXT` repeated per lot group and that "neither answer is wrong". Both halves are false. `TI_TEXT` is not mapped at all (issue 368 confirms), and the extra titles are the `TED-TITLE`s of `AWARD_CONTRACT` / `RES-n` award blocks, i.e. per-contract titles filed at Tender scope. So `9f0bcea` did not resolve a tie between equal-and-both-correct strings; it made the *wrong* pick deterministic. This issue's own worked example, 6287622 (two lot titles plus the framework title), is exactly that shape.

Evidence, literal, run 2026-09-15 against prod at `9e082fd`:

```
curl -s https://tenders.zebreus.click/v1/tenders/6751050
  -> title "Acquisition d'autocars et leur entretien"
     tender-scope (lot_id NULL) title texts: FRA x3 —
       "Acquisition d'autocars et leur entretien"
       "Acquisition de minibus et leur entretien"
       "Marché public de fournitures relatif à l'acquisition d'autocars et de
        minibus à haute performance environnementale (CNG ...) et leur entretien."
     (NLD x3, the same trio translated)
     lot_details: LOT-1.title = "Acquisition d'autocars et leur entretien"
                  LOT-2.title = "Acquisition de minibus et leur entretien"

curl -s https://tenders.zebreus.click/v1/notices/21658774/content
  -> PROCEDURE  TED-TITLE = "Marché public de fournitures relatif à ..."
     RES-1 (kind LotResult, parent PROCEDURE, LOT_NO 1) TED-TITLE
               = "Acquisition d'autocars et leur entretien"
     RES-2 (LOT_NO 2) TED-TITLE
               = "Acquisition de minibus et leur entretien"
     — the contract titles of the award blocks.

curl -s "https://tenders.zebreus.click/v1/tenders?publication_id=201318-2021"
  -> lists 6751050 under the same award-block title; tenders.current_title matches.
```

| surface | title served for 6751050 | what it should be |
| --- | --- | --- |
| `/v1/tenders/6751050` (detail) | Acquisition d'autocars et leur entretien (RES-1 contract title) | Marché public de fournitures relatif à … (PROCEDURE II.1.1) |
| `/v1/tenders?publication_id=201318-2021` (list) | Acquisition d'autocars et leur entretien | same |
| `tenders.current_title` (fold-time `head_title`, seq 4, `caused_by_notice_id` 21658774, `original_lang` FRA) | Acquisition d'autocars et leur entretien | same |

With `original_lang` FRA the three FRA rows tie on every rank term, and the ladder's "smallest value among equals" elects `"Acquisition d'autocars..."` because it sorts before `"Marché public..."`. Fold and read now agree — on the wrong string.

Rate, over the window `tenders.id BETWEEN 6750000 AND 6755000` (literal SQL run via `ssh root@zebreus.click '… | /root/sq.sh'`, counting heads whose lot_id-NULL `title` values outnumber their distinct languages):

| profile | tenders in window | heads with more tender-scope titles than languages | heads with lot_results |
| --- | --- | --- | --- |
| ted-export-r209 | 4,938 | 1,674 (33.9%) | 3,993 |
| ted-export-r208 | 63 | 9 (14.3%) | 25 |

Honest bounds on that number: 33.9% counts tenders carrying *any* extra tender-scope title, so it is an **upper bound** on mis-served titles, not the mis-served rate. The direct corpus rate ("served title != PROCEDURE title") could not be measured — one 408 on a mis-planned join, then the SQL backend answered 503 "SQL runtime saturated … query was never run" for >10 minutes across 10 polls. The wrong-title outcome itself was confirmed end-to-end on 2 of 2 worked examples (6751050 FRA; 6751051 POL — 25 tender-scope titles in one language, 24 lot_results, served title "Balony nacinające" while notice 20973483's PROCEDURE title is "Zakup materiałów dla Zakładu Hemodynamiki I"), plus one boundary case that behaves correctly: 6751052 has 15 award blocks whose titles all repeat the procedure title, so they collapse to one and the right title is served. The defect fires only when an award-block title differs from the procedure title *and* sorts before it.

Judge's reasoning for why this is ours, not the publisher's (adversarial panel, verbatim):

> Verified, and it is this system's doing, not the publisher's. (1) Mechanism confirmed in code: `crates/ingest/src/r209/rules.rs:338` maps `AWARD_CONTRACT`/`AWARD_OF_CONTRACT_DEFENCE`/`RESULTS` to `Kind::LotResult` (RES-n), and the block's `TITLE` is emitted as `TED-TITLE`; `TEXTS` (`crates/ingest/src/project.rs:110`) maps `TED-TITLE` to canonical `title` with no regard to section kind; `scope_of` (`project.rs:3851`) climbs only to `LOT_KINDS = ["Lot","LotsGroup","Part"]`, and a RES-n section's parent is `PROCEDURE`, so the contract title lands at Tender scope (lot_id NULL) — even though `read_legacy_results` (`project.rs:4118-4165`) binds that same block to its lot by `LOT_NO`. (2) Live check re-run: `/v1/notices/21658774/content` shows `PROCEDURE` TED-TITLE = "Marché public de fournitures relatif à …", `RES-1` (LotResult, parent PROCEDURE, LOT_NO 1) TED-TITLE = "Acquisition d'autocars et leur entretien", `RES-2` (LOT_NO 2) = "Acquisition de minibus …"; `/v1/tenders/6751050` serves title "Acquisition d'autocars et leur entretien" with three FRA and three NLD lot-NULL `title` texts (the two contract titles plus the procedure title) and LOT-1/LOT-2 carrying the same two strings at lot scope. The issue-343 ladder ("smallest value among equals") then elects the contract title whenever it sorts before the procedure title — exactly this case. (3) Cross-era inconsistency: the eForms analogue BT-721 (contract title) is deliberately NOT in `TEXTS` (only BT-21/BT-24 are), so an eForms CAN gets its procedure title while an r209 CAN gets a lot's contract title. (4) Internal invariant violated: issue 368 (open) and the `TEXTS`/`OJ_HEADING_FIELD` comments state the maintainers' rule that a mapping must add "no second, competing title"; this path adds one per award block. (5) Board: issue 343 is FIXED/DEPLOYED but covered only the tie-break between equally-ranked titles and explicitly called the multi-title shape source-published ("neither answer is wrong"), attributing it to `TI_TEXT` repetition — yet `TI_TEXT` is not mapped (368 confirmed), and 343's own example 6287622 (two lot titles + the framework title) is this very award-block shape. So 343 made the wrong pick deterministic rather than resolving the scoping; it does not cover this. No open issue covers award-block titles at tender scope (grepped AWARD_CONTRACT, CONTRACT_TITLE, RES-n, contract title, title election across the board). Caveat: I could not re-run the reviewer's 5k-window rate (1,674/4,938 r209 heads with more lot-NULL title values than languages) — the SQL backend answered 503 "saturated" twice, so that number stands as the reviewer's; the mechanism is verified on the worked example and in code. Actionable fix shapes: route a text fact from a RESULT_KINDS section to the LotResult's lot via LOT_NO (mirroring `read_legacy_results`), or exclude result-section `TED-TITLE` from `title` as BT-721 already is; then refold r208/r209 award carriers. `tender_version_contracts` has no title column, so a contract-scoped home would need schema.

Note on where this belongs: the judge's own read is that 343 does not *cover* the scoping defect — it is filed here because 343 is the only issue on the board that touched this shape and its premise (the "Why it is bounded" section above) is the thing this finding falsifies. Whoever picks it up may prefer to split the scoping fix into its own issue and leave 343 closed on the tie-break alone; that is a fine outcome, but 343 must not stay FIXED while its stated reason for being harmless is known-false.

Also in scope by construction: the docs premise. `docs/research/ted-legacy-mapping.md:341` states the canonical Title is `OBJECT_CONTRACT/TITLE` (r209) / `TITLE_CONTRACT` (r208), and line 147 lists AWARD_CONTRACT's TITLE as a member of the award block (CONTRACT_NO, LOT_NO, TITLE) — so the expectation is the documented design, not a reviewer's preference. No test or fixture pins a RES-n title at tender scope (fixture `f03-000988-2019.xml` has an AWARD_CONTRACT without TITLE), so the current behaviour is an unpinned side effect.

To close: either route result-section `TED-TITLE` to the LotResult's lot via `LOT_NO` (mirroring `read_legacy_results`) or drop result-section `TED-TITLE` from the `title` mapping as BT-721 already is, pin it with a fixture whose AWARD_CONTRACT title differs from and sorts before the PROCEDURE title (asserting list == detail == `current_title` == the procedure title), then refold the r208/r209 award carriers and re-probe 6751050/6751051.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/6751050 | python3 -c "import sys,json; print(json.load(sys.stdin)['title'])"

- **done**: `Marché public de fournitures relatif à …` — the PROCEDURE-scope II.1.1 title
- **open**: `Acquisition d'autocars et leur entretien` — the RES-1 award-block contract title filed at tender scope (read 2026-09-18 at rev `c36de25`, unchanged)

## Comment — 2026-09-18: re-read live, still open

Tender 6751050 still serves `Acquisition d'autocars et leur entretien` (kind `procedure`, FRA). No
fix has been built; the two closes the reopen names (route result-section `TED-TITLE` to the
LotResult's lot via `LOT_NO`, or drop it from the `title` mapping as BT-721 already is) both end in
an r209-wide refold, which is the same permission class the classifier refused for issue 393
unit 2's run today — so the code half can be built any firing, and the refold waits with the others.
