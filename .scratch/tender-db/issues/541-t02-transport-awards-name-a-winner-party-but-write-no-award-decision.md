# 541 — 2016 public-transport awards (T02) name their operator as a `winner` party but write no award decision

Status: **DONE 2026-10-19** — unit 2 deployed (923172d, live in 9529345) and drained: refold-fields 2631 (753 TED-COST_PARAMETERS carriers re-queued, 753 Tenders stamped) + project 2632 (627 verified + 126 corrected); Verify: 5782055 serves `lot_results` RES-T02 winner 1953406 (Barnimer Busgesellschaft mbH) = its `winner` party, selec-w, 2016-09-21; 12 of 12 salted corrections likewise. Was: ready-for-agent — **unit 2 BUILT 2026-10-19** (wf_a8f42aac-9fa: implement, three review lenses, a skeptic per finding, fix; focused runs GATE-EXIT=0; full gate pending): a T02 (`TED-FORM` T02) with no LotResult writes one result `RES-T02` from its Notice section — winners = `ECONOMIC_OPERATOR_NAME_ADDRESS` refs through the `dropped` rule, decided = `CONTRACT_AWARD_DATE`, no value (none confirmed as section V's). Next: gate, deploy (after 551 B5's band seek), drain by `refold-fields` on `TED-COST_PARAMETERS` (notice_texts, 753 carriers), Verify on 5782055. Was: ready-for-agent — SIZED 2026-10-18 (refold-fields 2618, `expect: 1` on `TED-COST_PARAMETERS` in notice_texts): **753** notices carry it (T01 prior-information + T02 award MOVE forms together), against 970,980 for the shared `TED-ECONOMIC_OPERATOR_NAME_ADDRESS`. So the T02 cohort is at most 753 notices, small enough for an exact-cohort drain. Next: unit 2 build behind a MOVE-form predicate (a PROCEDURE-section award pair on a notice that carries the MOVE fields), then refold-fields over `TED-COST_PARAMETERS` (notice_texts) — size known: 753.
T02 cohort and read whether its section V carries enough to write a result.
Kind: coverage (legacy results)
Relates to:
- 534 (where this was seen);
- 242 (the same shape for another legacy award container);
- 531 (`?winner=` reads result winners only).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/5782055 | jq -c '[([.parties[]|select(.role=="winner")|.organization_id]), (.lot_results|length)]'

- **done**: the operator is in `lot_results[].winners`, so `?winner=` finds it.
- **open (2026-10-14)**: `[[1953406],0]`: the `winner` party is present and `lot_results` is empty.

## What was found

A T02 award (R2.0.8 MOVE, Regulation 1370/2007 public-transport contracts) names its operator under
`ECONOMIC_OPERATOR_NAME_ADDRESS`. That folds to `winner`, so the operator shows as a `winner` party. But
`AWARD_CONTRACT_MOVE` is a group, not a result section, so no award decision is written: `lot_results` is empty and
`?winner=` misses the operator.
- **Sample.** All six T02 tenders sampled in 534's 2,427-tender transport read look like this, for example 5782055
  (382248-2016, Barnimer Busgesellschaft) and 5804664 (427137-2016).
- **Owners.** The operator's public owners are served under `PARTY_NAME_ADDRESS`, refused in 534.

## Triage read (2026-10-16)

- **Where the award sits.** T02 notice 18999280 (tender 5782055, 382248-2016), read on `/v1/notices/18999280/content`,
  stores section V in its `Notice/PROCEDURE` section: `TED-CONTRACT_AWARD_DATE 2016-09-21` and
  `TED-ECONOMIC_OPERATOR_NAME_ADDRESS ORG-2` (a ref to the `Organization` section of Barnimer Busgesellschaft mbH),
  beside `TED-COST_PARAMETERS` / `TED-CLEANLINESS_ROLLING_STOCK` texts. `AWARD_CONTRACT_MOVE` is a Group, so its
  children land in the enclosing section. Since no `LotResult` section exists, `read_legacy_results` never sees them.
- **Sizing attempt.** `refold-fields` 2532 (`expect: 1`, notice_ids): **970,980** notices carry
  `TED-ECONOMIC_OPERATOR_NAME_ADDRESS`. That is far beyond T02. The R2.0.8 standard award forms name their
  contractor with the same element, so the field cannot size the T02 cohort.
- **Next.** Size on a T02-only field first: one in the MOVE forms' section II/VI, e.g. `TED-COST_PARAMETERS` in
  notice_texts (T01 + T02), or the notice's form-type field. Then the build reads the PROCEDURE-section award pair
  only on a MOVE-form notice: a predicate on the form, so the standard forms' contractor path is not touched.

## Sizing (2026-10-18)

- `refold-fields` 2618 (`{"profiles":["TED-COST_PARAMETERS"],"tables":["notice_texts"],"expect":1}`): aborted
  with **753** carriers, nothing written; the paired `project` 2619 ran 0 notices. The notice_texts scan took 40 min
  (16:22–17:02 box time), so a wet drain on the same field costs that scan again plus a 753-notice fold.
- 753 is an upper bound on T02: T01 (prior information) also carries the cost parameters. The build's predicate
  must still require the PROCEDURE-section award pair, and the drain can use the same field.

## Proposed units

1. **Size (S).** Count the T02 cohort (`TED-AWARD_CONTRACT_MOVE` field ids in `notice_ids`, with an in-app or bounded
   read).
2. **Build (S–M).** Write a result from the T02 section V, as 242 did for its container: one lot result whose winner
   is `ECONOMIC_OPERATOR_NAME_ADDRESS`, plus the award value and date if published. Then refold the cohort.

## Unit 2 — built (2026-10-19, workflow wf_a8f42aac-9fa)

- **Where.** `read_legacy_results` used to return early when a notice had no `LotResult`; that branch now first appends
  `read_move_award(sections, parsed, dropped)`. So it runs on the legacy path only, and only without a result section.
  The legacy decision rule moved unchanged into `decide_legacy_result`, shared by both.
- **Predicate.** `first_code(parsed, "TED-FORM")`, trimmed, case-insensitively `T02` (`MOVE_AWARD_FORM`). T01 (the MOVE
  prior information, which names the operator the buyer intends to award), the F-forms and a missing code write
  nothing. An R2.0.9 T02 puts its award in `AWARD_CONTRACT`, a `LotResult`, so it never reaches the new branch (its
  owners problem is filed as **571**).
- **Result.** Key `RES-T02` (`MOVE_AWARD_RESULT_KEY`: the walkers mint `<prefix>-<n>` with a numeric n, so it names no
  real section and is the same on every refold). `lot_key` None (T02s have no lot sections). Winners: each
  `TED-ECONOMIC_OPERATOR_NAME_ADDRESS` ref directly on a `Notice` section, through the legacy `dropped` rule (void →
  `void_lot`, placeholder → `withheld_winner`, blank ignored). The buyer (`CA_CE_CONCESSIONAIRE_PROFILE`) and the owners
  (`PARTY_NAME_ADDRESS`, issue 534) are never read. Date: `CONTRACT_AWARD_DATE`. No result when section V names no
  operator.
- **Value: none.** 382248-2016 stores 2,186,477.00 and 2,186,477.47 EUR twice over, under `TED-VALUE` and
  `TED-VALUE_COST` (ordinals 0 and 1), typed by ordinal by `TED-VALUES.TYPE` `GLOBAL` / `CONTRACT`, with no `VAL_TOTAL`;
  427137-2016 publishes `UNKNOWN_VALUE`. Nothing confirms which is section V's awarded value (ted.europa.eu answered empty
  202s from the container), so no value is written. Follow-up idea, not filed: pair `VALUES.TYPE CONTRACT` with its
  ordinal's amount once a T02 original confirms the pairing.
- **data_quality.rs.** The review found T02 versions would sit in section 3's `sections_with` numerator without the
  denominator (no result section) and be subtracted twice in 3b (`with_results` and `no_award_content`). Fixed:
  `sections_with` adds the result-section probe (`sections_with_sql()` / `sections_with_template`, catalog and windowed
  forms from one template), and `no_award_content` leaves out versions that wrote a `lot_results` row.
- **Tests (project.rs):** `a_t02_award_writes_its_operator_as_the_result_winner` (18999280 shape, full read + bind:
  winners [the operator's org], winner role ORG-2, owners keep `PARTY_NAME_ADDRESS`, stable key on re-read, form code
  trimmed / case-insensitive), `a_t02_award_binds_every_operator_it_names`, `only_a_t02_writes_a_result_from_its_notice_section`
  (T01/F03/F06/3/T2/T020/empty/missing; non-legacy path), `an_award_with_a_result_section_is_read_as_before`,
  `a_t02_owner_is_never_a_winner`, `a_t02_dropped_operator_decides_as_a_legacy_block_would`.
- **Review record.** 9 findings, 7 confirmed (the blocking one: the first test read `state.roles` after
  `bind_organizations` had moved them into facts — fixed with `t02_parties`), 2 refuted: a blank, undated operator
  slot reading `clos-nw` (needs a T02 without `CONTRACT_AWARD_DATE`, which the form requires), and the issue-384
  reader-arm guard not scanning the new function (both arms have a destination today).

## RESOLVED-VERIFIED (2026-10-19)

- Deployed with 551's switch-off (rev 9529345; 541 unit 2 is 923172d, gate green 1,450 s on its tree).
- Drain: `refold-fields` 2631 (`{"profiles":["TED-COST_PARAMETERS"],"tables":["notice_texts"],"expect":753}`, 03:42Z,
  sweep ~40 min): 753 carriers, 753 notices re-queued, 753 Tenders stamped; project 2632 (04:2xZ): 820 Tenders folded,
  753 written, **627 verified + 126 corrected** (D7 complete), 128 versions, 127 correction rows. So 126 T02 Tenders gained
  their result (the other 627 carriers are T01 prior information and T02s already identical).
- Verify: `/v1/tenders/5782055` → winner parties [1953406], `lot_results` [RES-T02, winners [1953406 Barnimer
  Busgesellschaft mbH], selec-w, decided 2016-09-21] — the issue's "done" line. 12 salted corrections (`/v1/changes` after
  958809778, salt `541-`): each has one RES-T02 result whose winner is its `winner` party (Trenitalia, VR-Yhtymä, PKP
  Intercity, VGS Südharz, …).
- One exhibit carries a placeholder name as operator: 5266661 'Voir Section VI) Renseignement complémentaires' — already
  its `winner` party before 541; the placeholder rule's gap is recorded as issue 511 unit 7 item 5.
