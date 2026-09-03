# 343 — a version with several tender-level titles: the fold's `current_title` and the read-time pick break the tie differently

Status: FIXED 2026-09-02, DEPLOYED 2026-09-03 (`9f0bcea`). Read side verified on prod; the materialised `current_title` of heads written by the pre-fix fold (612) keeps the old tie-break until refolded — see the probe note at the end.
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
