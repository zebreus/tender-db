# Issue 491 unit 3 (b) — implementation plan (2026-10-11)

Produced by a read-only Plan agent; ACCEPTED by the owner with one decision taken now: **projection C, not A** (§0.2, §1g). The plan's §0.1 found that a `TED-VAL_TOTAL` inside a `RES-n` reaches the projection twice — as the result's `direct_cents` AND as a tender-scope `result_value` head candidate (`amount_target` maps by field id; `scope_of` gives `Tender` for a RES with no Lot ancestor) — so under A every contract figure becomes a head candidate and the MAX election would raise ~half of ~29k single-contract heads (VAT basis, estimate-vs-final) and flip ~2.6k currencies. The unit-3 decision chose A without knowing that. C follows r208's precedent (an award figure inside a result section is a lot award and a scale-rule partner only, never a head candidate): heads move only by refusal, 3427333's €35 bn head is refused against its exact ×1000 partner (head `null`, as 492's r208 cases), and the parse change is identical either way, so A stays available later by a scoped refold if a measurement ever argues for it. Consequently M2 (head prediction under A) is dropped; M0 (pre-2004 check) and M3 (the probe) stay, M1 is optional.

## Plan: issue 491 unit 3 (b): store each award's own figure in its result section

I read the code and the five issue files and ran nothing. No files were edited and cargo was not run.

### 0. Findings that change the picture (read these first)

1. **A `TED-VAL_TOTAL` inside a `RES-n` reaches the projection twice.** The decision expected one home and there are two.
   - It becomes the result's `direct_cents`: `project.rs:6111`, `read_legacy_results` → `lot_results.awarded`.
   - It is also a tender-scope `result_value` fact. `amount_target` (`project.rs:7329-7342`) maps `TED-VAL_TOTAL` by field id alone, and `scope_of` gives `Tender` for a `RES-n`, because it has no Lot ancestor (`project.rs:5758`, loop at `4608-4652`).
   - This is the r209 shape that issue 492 calls the "lot-null copy" (decision (c); `ScalePartners::is_head_award`, `canonical.rs:3202`).
   - The consequence: **the contract figures become head candidates.** `head_value_eur_cents_with` (`canonical.rs:3182-3270`) takes the MAX over every tender-scope amount, and the copies are among them.
   - r208's award `VALUE_COST` is the opposite precedent. `amount_target` returns `None` inside a result section, so those values are lot awards only and never head candidates. Issue 492 records the outcome: "7 of the 9 now have no head, because the award is not a head candidate".
2. **The parse change is identical whichever projection you choose.** Only the projection mapping differs, so the reparse is valid either way. The choice can even be revisited after the drain with a scoped refold.
   - **A (as decided, r209 shape, no projection change).** 3427333's head falls to €35,076,200.
     - Multi-contract notices with no II.2.1 gain a head: their largest contract.
     - Single-contract notices whose V.4 is larger than II.2.1 see their head rise.
     - The decision's sample had 1,444 of 7,814 single-contract bodies that differ by something other than 10ᵏ, and 132 in a different currency. That is roughly ×20 corpus-wide, so about half of ~29k notices may rise, and ~2.6k may flip currency.
     - A V.4 slip below €1 bn wins by MAX.
   - **C (r208 shape, a one-line projection switch).** Text-profile `RES-n` `TED-VAL_TOTAL` values are lot awards and scale-rule partners only. Heads move only by refusal; 3427333 falls to `null`, because nothing else is admitted.
   - **Recommendation:** build A as decided, and make A versus C an explicit gate after the measurement in §5. The exact diff for C is in §1g.
3. **No double counting.** `facts` is a `BTreeSet<Fact>`, and `Fact::Amount` = (field, cents, currency, tax_basis, quality).
   - A single-contract body whose II.2.1 equals V.4 with the same basis gives one fact. `amounts` and the head are unchanged; only `awarded` is added to the result. That is the 6,219-of-7,814 case.
   - Equal cents with a different stated basis gives two `amounts` rows. The head is unchanged.
   - The head election is a MAX, not a SUM.
   - Issue 492's lot sums and issue 505's residual tables count only awards with a lot key or Lot-kind lots. Text awards have `lot_key = None` and text versions have no lots, so `head_lots = 0`. 492's framework and sibling-lot exemptions therefore never apply to text heads, today or after.
4. **3427333 under 471's rule (A).**
   - Partner: `partner(EUR, 3_507_620_000_000)` finds `3_507_620_000` at k = 3. It is registered both from the tender-scope copy and from the lot award (`add_version_figures`, `canonical.rs:3625`).
   - Not corroborated: `head[(EUR, 3.5e12)] = Some("result_value")`, the same field.
   - Above the gate: €35 bn ≥ €1 bn. So it is refused, the €35,076,200 copy is below the gate and admitted, and the head is €35,076,200.
   - Section 16 shows the chain has no other figure ("x smallest —"), so no CN estimate can corroborate. Confirm this at the probe.
5. **The drain is not ~1.3M notices.** Text fetches 186–269 (217 is not text) hold ≈2.11M notices: 422,001 (186–198) + ~30k (199) + 1,658,553 (200–269, from `508-2010-text-vintage/text-fetch-map-*.txt`).
   - Roughly 800k of them are award notices.
   - About 450–500k notices will gain a result value. Each changes `tender_version_lot_results` and produces one `tender changed` correction (ADR-0017 D3).
6. **Side effect: the reparse also applies issue 510's newer name rules.** Issue 510 changed `partyname` after 509's drain (`a3153bc` and later; the drain ran on `0292104`).
   - Some names leave the parse layer (their mentions were already retired by `refold-void-names`).
   - Some award summaries `names_an_award` now keeps may enter it.
   - Measure this in M1, and run `sweep-orphan-orgs` dry after the drain.

### 1. Code changes

All in `crates/ingest/src/text/parse.rs` unless noted.

**1a. Shared helpers**
- `type Money = (i64, String, Option<&'static str>);` for the new code only.
- `fn flatten_lines(body: &str) -> (String, Vec<usize>)`, next to `flatten` (~1421).
  - The string is exactly `flatten(body)`.
  - For each line of `body.lines()` it records the flat offset of that line's first word: `flat.len() + (flat non-empty as 1)` before pushing it, or `flat.len()` for an empty line.
  - Use `body.lines()` so indices match `line_awarded_names`.
- `fn value_claims(flat: &str) -> Vec<(usize /*label start*/, usize /*scope*/, Money)>`.
  - Extract the `while` loop of `awarded_value` (1240-1279) verbatim, but push each successful claim instead of folding it.
  - `awarded_value` (1230) becomes: gate → `flatten` → `value_claims` → the same per-scope fold (first claim wins, a later different claim sets `conflict`), the same `awards > 1` drop, the same widest-scope pick. Its behaviour must stay byte-identical.
- `fn block_value(block: &str) -> Option<Money>`: the block's scope-0 claims from `value_claims(block)`. If all agree, take the one; if any disagree, or there are none, return `None`. This is the same "agreeing is one fact" rule `awarded_value` applies per scope.

**1b. Splitting a body into award blocks**

`fn award_block_ranges(body: &str, flat: &str, line_starts: &[usize]) -> Vec<Range<usize>>`, placed after `awarded_names`.

- Define `opens(line, marker)`: the raw line, not trimmed, starts with `marker` (case-insensitive) and the next byte is not an ASCII letter. This is `count_heading`'s rule (279), applied per line, line 0 included.
- **Gate.** `sv` = the first line with `opens(l, "SECTION V")`. That rejects `SECTION VI`. If there is none, return `[]`.
  - This is stricter than `awarded_value`'s loose `find_ascii_ci(body,"SECTION V")`.
  - The numbered form never opens a line with that heading. A mid-line "see Section V" does not count.
- **Candidate starts.** All are flat offsets, and only after line `sv`:
  - `contract`: `line_starts[i]` for lines where `opens(l, "CONTRACT NO")`;
  - `lot`: the same for `"LOT NO"`;
  - `v3`: each non-overlapping occurrence of `AWARD_BLOCK_HEADING` in `flat` at or after `line_starts[sv]`. Move each one back over a preceding `"V.3) "` (5 bytes, case-insensitive), so the previous block's figure is not followed by a stray `V.3)` token. Without this, `parse_money("7 140 EUR V.3)")` refuses, which is exactly why 4300188's first two figures are lost today.
- **Family by priority, not by count:** `contract` if non-empty, else `lot`, else `v3`, else `[]`.
  - Priority keeps a joint or framework contract (one `CONTRACT NO` with two V.3 and one V.4) whole.
  - A contract whose heading is missing degrades safely: its name falls outside every block and gets no figure.
- **Ranges.** `starts[i]..starts[i+1]`. The last block ends at the first line after it with `opens(l, "SECTION VI")`, or at `flat.len()`.

**1c. Names with positions**
- `fn awarded_names_at(body: &str, flat: &str, line_starts: &[usize]) -> Vec<(usize, String)>`. This is the body of `awarded_names` (763-832), unchanged except that each name is pushed with a position, and the gate stays in the callers.
  - A label name's position is the flat offset where its value starts after the `CONTRACT NO …:` hop, i.e. `value_at + contract_no_prefix_len(rest)`. Do not use the label start: a `Contract No` line under its own label then lands in its own block.
  - The order stays as today: label names, then line names.
- `line_awarded_names(body, authority, line_starts) -> Vec<(usize, String)>` (921). The position of each name is `line_starts[index of the name line]`, i.e. the line read after the heading; for a wrapped name it is the first line.
- `fn awarded_names(body) -> Vec<String>` keeps its signature for the ~40 test call sites: gate, `flatten_lines`, `awarded_names_at`, then drop the positions.

**1d. Attaching figures to names**

`fn awarded_names_and_values(body: &str) -> Vec<(String, Option<Money>)>`:
1. Apply the same gate as `awarded_names` (AWARD/PROVIDER), then `flatten_lines`, then `named = awarded_names_at(...)`.
2. `ranges = award_block_ranges(...)`. If it is empty, or the body contains neither `PRICE` nor `VALUE`, return every name with `None`.
3. Find each name's block by binary search over the ranges. Positions outside every block map to `None`.
4. A block gets a figure only when all three hold:
   - (a) exactly one name falls in it;
   - (b) it holds no nested award: at most one `v3` start and at most one `lot` start inside the range;
   - (c) `block_value(&flat[range])` is `Some`.
5. Return the names in `named` order, each with its block's figure or `None`.

The returned names are always exactly `awarded_names(body)`, so RES and ORG numbering and every mention section id are unchanged.

**1e. Emitting the figures**
- `struct Emit` (2119) gains `award_values: Vec<(String, Money)>`. It still derives `Default`.
- `Emit::award(&mut self, name: String) -> String` (2201) returns the `RES-n` id it minted. Nothing else changes in it.
- `fn value_in(&mut self, section: &str, cents, currency, basis)` pushes `TED-VAL_TOTAL` (`Amount`) and, when a basis is stated, `TED-VAL_TOTAL_TAX_BASIS` (`Code`) into `section`. `Emit::value` (2235) becomes `self.root(); self.value_in(SECTION, …)`.
- `flush` (1958-1962):
  ```rust
  for (name, value) in awarded_names_and_values(&text) {
      let result = emit.award(name);
      if let Some(money) = value { emit.award_values.push((result, money)); }
  }
  ```
- Add `fn is_award(emit: &Emit) -> bool` (TD = 7), and `fn claim_award_values(emit: &mut Emit)`:
  - `let values = std::mem::take(&mut emit.award_values); if !is_award(emit) { return }`
  - then `value_in(&result, …)` for each.
  - The TD gate is the same reason `claim_awarded_value` has one: 23 non-award bodies in fetch 300 state a price.
- In `parse()` (1550-1555), call it right after `claim_awarded_value`, so the notice-scope row keeps ordinal 0. Ordinals are per field across the record (`push_into`).
- `claim_award_skeleton`, `claim_award_date` and `claim_tenders_received` are untouched, and the set of `RES-n` sections is unchanged.

**1f. Doc comments to update**
- `AGGREGATE_SCOPE` (239-240): "a lot_results-scope fact this does not yet have a home for" → per-award figures now go to their own `RES-n`.
- The displaced `claim_awarded_value` paragraph (1597-1600, "Notice scope, not the LotResult …"): the notice claim is unchanged, and each award's own figure now goes inside its result.
- `Emit::value` doc.
- New docs, each with its exhibits (3403986, 3871014, 4040506, 4300188, 4101747).

**1g. Projection**
- **A:** no code change in `project.rs`.
- **C (only if the gate says so):** pass `text: bool` (`notice.profile == "text"`) from `NoticeState::read` (4428) into `amount_target` (7329), and add before the `canonical_name` return:
  ```rust
  if text && field_id == "TED-VAL_TOTAL" && enclosing(sections, section_id, RESULT_KINDS).is_some() { return None; }
  ```
  `read_legacy_results` still folds it as `direct_cents`, so the figure stays a lot award and a partner (`add_version_figures` reads awards).

**1h. Public notes**
- `CHANGELOG.md` entry "Unreleased (issue 491)":
  - text-era 2004–2010 awards serve their own `awarded` figure;
  - 3427333-type scale pairs are judged;
  - under A only: `value` may move to a contract's figure, and `amounts` lists each contract figure beside the notice total, so do not sum `amounts`.
- Optionally extend the `/docs` lot-value paragraph (`crates/app/src/v1/docs.rs:519-522`) with "and the 2004–2010 text era".

### 2. How names and figures pair, case by case

| Case | Outcome |
|---|---|
| Not sectioned (numbered form) or no block markers | Today's output exactly: one `RES` per name, no figure on any `RES` |
| One block, one name, one figure (or agreeing figures) | `RES-1` carries the figure. The root claim is unchanged, so equal figures dedupe to one fact |
| Several names in one block (`A; B`, `1: A. 2: B.`) | Today's `RES` per name, **no figure** (it is joint or unattributable) |
| Block with no plausible name (`not_a_name` VoidLot or Placeholder, `Various`, a country-only slot, an empty slot) | No `RES` (as today). Its figure is stored nowhere and never shifts onto a later award |
| Name outside every block (before the first marker) | Its own `RES`, no figure |
| Block whose figures disagree, or none parse (range, zero, 3 decimals) | `RES` with its name, no figure |
| Block containing ≥ 2 V.3 headings or ≥ 2 `LOT NO` lines (a multi-lot contract) | No figure |
| Rejection / winner-silent skeleton | Unchanged: no names, so no figures; the skeleton `RES-1` never gets one |
| 2004/05 `V.1.2` "of contract(s)" (scope 1) inside blocks | Not a block figure. **Residual:** today's root fold can still claim one contract's agreeing figure as the total. Size it in M1 and file it separately if material |

### 3. The notice-level claim

`awarded_value` and `claim_awarded_value` are textually unchanged apart from the extraction of `value_claims`. II.2.1 still wins by scope, a single-contract V.4 is still the root claim when there is no II.2.1, and `awards > 1` still drops a part.

So a single-contract notice now carries both the root total and `RES-1`'s value. That is what 3427333 needs. The projection consequences are §0.3 and §0.4.

### 4. Tests

**`parse.rs` tests**

Bodies come from `/v1/notices/{id}/content`, verbatim, as the convention requires. Where I could not read them, a modelled body is given, and the assertions depend only on the figures and the name count.

1. `a_single_contracts_figure_lands_on_its_result_beside_the_notice_total` (3427333, notice 3403986, 143891-2008). Model it on 3871013's verbatim layout (test at 3775):
   ```
   II.2)  TOTAL FINAL VALUE OF CONTRACT(S)
   II.2.1)  Total final value of contract(s): Value: 35 076 200 000 EUR.
   Excluding VAT.
   SECTION IV: PROCEDURE
   SECTION V: AWARD OF CONTRACT
   CONTRACT NO: 1
   V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN 
   AWARDED: <winner verbatim>, <address>.
   V.4)  INFORMATION ON VALUE OF CONTRACT Initial estimated total value of the contract:
   Value: 35 076 200 EUR.
   Excluding VAT.
   Total final value of the contract:
   Value: 35 076 200 EUR.
   Excluding VAT.
   SECTION VI: COMPLEMENTARY INFORMATION
   ```
   - `awarded_value == Some((3_507_620_000_000,"EUR",Some("excl")))`
   - `awarded_names_and_values == [(<winner>, Some((3_507_620_000,"EUR",Some("excl"))))]`
   - Record with `TD: 7`: `TED-VAL_TOTAL` at `PROCEDURE` ordinal 0 = 3.5e12; in `RES-1` = 3.5e9; `TED-VAL_TOTAL_TAX_BASIS` "excl" in `RES-1`.
   - With `TD: 3`, and with no TD: no `TED-VAL_TOTAL` anywhere.
2. `four_contracts_carry_their_own_figures_and_the_total_is_unchanged` (3871014). Take the `four_contracts` text (3742) and add each contract's verbatim `V.3)…AWARDED: <name>, …` line pair after `CONTRACT NO: n`.
   - The four `RES` figures are `4_008_759_600`, `1_560_500_000`, `1_470_077_200`, `1_121_203_500` SEK, all "excl".
   - Their sum equals the root; `awarded_value` is unchanged at `8_160_540_300`.
3. Extend `a_2010_body_awarding_lots_without_contract_numbers…` (4040506, 4219):
   - values `[(Entreprise ITE, 191_213_800 EUR), (Entreprise SRBG, 60_782_270), (GC Cuisines (Groupe LANEF PRO), 14_779_573)]`, each with basis `None`;
   - `awarded_value` is still `None`;
   - today only the last lot parses, so slicing at the block boundary is what reads the other two.
4. Extend `several_award_blocks_without_headings…` (4300188, 4387):
   - `[185 650, 7 140, 103 835]` EUR on three `RES`; the root stays `None`;
   - `one` gives `RES-1` = root = 10_383_500.
5. Extend the existing exhibits:
   - 4101747 (4175): `RES` = 35_905_000 and 2_820_000 PLN excl; the root is still 113_918_500.
   - 3990107 (4105) and 4200000 (4125): `RES-1` equals the root, with the "incl" basis landing in `RES-1`.
   - 4041078 (4361): `RES-1` 14_967_714 GBP excl.
6. `a_block_whose_figure_does_not_parse_stores_none`: three `CONTRACT NO` blocks A/B/C.
   - B's V.4 is 3870959's shape `Total final value of the contract: Lowest offer: 16 184 142,63 /\nhighest offer: PLN.`, which gives `[(A,Some), (B,None), (C,Some)]`.
   - A second variant gives B `Value 0 EUR` (4101508).
7. `a_name_that_is_not_a_name_does_not_shift_later_figures`: three blocks with values 100 000, 200 000 and 300 000 EUR.
   - The middle winner is `Lot déclaré infructueux`, giving names `[A, C]` with `[100 000, 300 000]`, and 200 000 stored nowhere (the root is `None` since `awards = 3`).
   - Repeat in the 2010 print, with a `GERMANY` country-only slot (4201806) and with `Véase perfil del contratante`.
8. `a_block_naming_two_winners_carries_no_figure`: one `CONTRACT NO` block, `AWARDED: Acme Ltd, Wood Street; Bolt GmbH, Ring 1.` plus a V.4 figure.
   - Two figure-less `RES` (as today); the root keeps the V.4, unchanged because `awards = 1`.
9. `a_contract_with_two_lots_carries_no_figure`: `CONTRACT NO: 1 / LOT NO: 1 / V.3 A / LOT NO: 2 / V.3 Infructueux / V.4 v2` gives `[(A, None)]`.
10. `a_name_before_the_first_contract_heading_keeps_no_figure`.
11. `the_numbered_form_is_unchanged`.
    - redcar (2686), lyon (`8. Price: 5 301 802,22 FRF TTC.`, 2739), staffanstorp (`8. Price(s): 1: 2 750 000 SEK; 2: 245 000 SEK.`), the `numbered` Lot No body (4258), and a numbered body containing "see Section V" mid-line: every figure `None`.
    - Records: no `TED-VAL_TOTAL` outside `PROCEDURE`.
12. `the_rejection_skeleton_carries_no_figure`: the `rejected` body (3258) plus a V.4 line.
    - `RES-1` holds `TED-NO_AWARDED_CONTRACT` and no `TED-VAL_TOTAL`. The 17,438 `Various` body is likewise unchanged.
13. `names_are_read_exactly_as_before` (an invariant over ~15 bodies from the existing tests): names from `awarded_names_and_values` equal `awarded_names`, and `flatten_lines(b).0 == flatten(b)`, including tabs, NBSP and blank lines.
14. `four_hundred_sectioned_blocks_stay_linear`: `SECTION V:\n` plus 400 × `CONTRACT NO: n\nV.3) … AWARDED: Acme Ltd, 1 Road.\nV.4) … Total final value of the contract: Value: 1 000 EUR.\n`. All 400 `RES` carry the figure.

**`crates/ingest/tests/text.rs`**
- In `the_2005_award_record_carries_the_chain_edge` (290), add `values(&rec,"TED-VAL_TOTAL").is_empty()`. There are two blocks with names and no value label.
- The 1993 daily's existing `prices.len() == 0` (157) already covers figures in any section.

**`project.rs` tests** (model on the `text_record` and `round` closures at 10232-10257)
- `a_text_awards_figure_is_its_results_awarded_value`: parse test 1's record and run `NoticeState::read(notice("text"))`.
  - `round.lot_results[0]`: `awarded_cents == Some(3_507_620_000)`, EUR, `lot_key: None`, `selec-w`.
  - Under A, the facts hold `result_value` 3.5e12 AND 3.5e9. Under C, only 3.5e12.
  - A single-contract body whose II.2.1 and V.4 both read `791 805 EUR. Excluding VAT.` gives exactly one `result_value` fact.

**`canonical.rs` tests** (next to `a_figure_exactly_ten_to_the_k…` at 35086; use a lot-key-`None` `LotResultState`, `lots: vec![]`)
- Facts `[result_value 3_507_620_000_000, result_value 3_507_620_000]` plus award(None, 3_507_620_000) → `Some(3_507_620_000)` (A).
- `[3.5e12]` plus the award → `None` (C).
- Adding `estimated_value 3.5e12` → `Some(3.5e12)`, which pins the corroboration gate.
- `[result_value X]` plus award(None, X) → `X` (no double count).

### 5. Measurement before the drain

There is no dry `reparse` (`docs/operations.md:1922`). So, as issue 508 did, measure with the real functions around the wet run.

- **M0: pre-2004.** Run `508-2010-text-vintage/shape.sh` with `SUM(value LIKE '%'||char(10)||'SECTION V:%')` and `… AND value LIKE '%Total final value of the contract%'`, over 5k windows every 50k in ids 1.90M–2.24M (fetches 270–288). If any are non-zero, add those fetches to the drain.
- **M1: parse classes.** Use a temporary `#[ignore]` harness that is not committed (the 508 precedent).
  - Export with ~43 bounded `/v1/sql` reads (on the team lead's word; never retry a 408):
    `SELECT notice_id, value FROM notice_texts WHERE notice_id BETWEEN a AND a+999 AND section_id='PROCEDURE' AND field_id='TXT-TX' AND ordinal=0 AND value LIKE 'CONTRACT AWARD NOTICE%'`, with `a` every 50k from 2,250,000 to 4,350,000. That is about 400 bodies and about 2 MB per read, under the cap. Keep the bodies out of git.
  - Run with the gate flags (CLAUDE.md), `-- --ignored --nocapture`. Print a TSV per notice and the totals:
    - bodies with any `RES` figure;
    - single-name blocks: equal / differs / differs by exactly 10ᵏ (both directions) / different currency / no root;
    - multi-block with no root but at least one `RES` figure (gains a head under A);
    - max `RES` figure in the same currency above the root (rise under A);
    - pairs with max ≥ 10¹¹ cents (refusal candidates);
    - dropped figures by cause (name-less, multi-name, nested, disagreeing, outside);
    - fallback bodies;
    - same cents with a different basis (extra `amounts` row);
    - names that differ from the stored `TED-OFFICIALNAME` (510's lists).
  - Regression check: the root claims must reproduce the stored `TED-VAL_TOTAL` at `PROCEDURE` exactly. Read `notice_amounts` and `notice_texts` for the same windows; this mirrors 508's 12,412-of-12,412 check.
- **M2: heads.** For the gain, rise, currency and 10ᵏ rows, read in batches of 500 ids:
  `SELECT v.caused_by_notice_id, t.id, t.current_value_eur_cents, (t.current_seq=v.seq) FROM tender_versions v JOIN tenders t ON t.id=v.tender_id WHERE v.caused_by_notice_id IN (…)`, which uses the `tender_versions_notice` index.
  - Predict under A: new head = max(current, copy) unless refused at ≥ €1 bn. Extrapolate ×50.
  - Adjudicate every refusal row at ≥ €1 bn (expected: 3403986, 3301887, 3500335, 3802477 and similar).
  - Adjudicate 30 random rises against `/content`.
  - **Gate:** keep A unless the rises read as worse figures than today's; otherwise build C (§1g plus its tests). Record the counts and the verdict on 491.
- **M3: probe, the exact in-app measurement.**
  - Before: snapshot `t.id, t.current_value_eur_cents` for heads caused in 3401288–3425872 (fetch 216, which holds 3403986), in 5k-id windows. Also read `/v1/tenders/3427333` and `/v1/notices/3403986/content`.
  - Run `{"kind":"reparse","profiles":["text"],"after":215,"packages":1,"reclaim_only":true}`, re-read `/content` (`RES-1` `TED-VAL_TOTAL` 35 076 200 EUR), then run `{"kind":"project"}`.
  - Diff the snapshot: gained / rose / fell / unchanged. Compare it to M2's prediction for that range, and compare the reparse time per notice with 509's rounds; stop if it is more than 1.5× slower.
  - The probe is valid under A and C alike. If the gate flips, deploy C and refold the probe's notices.

### 6. The drain

Repeat 508's and 509's rounds exactly; their closures stayed under the 500k line. The order is: deploy in a queue gap, the probe, then:

`{"kind":"reparse","profiles":["text"],"after":X,"packages":N,"reclaim_only":true}` followed by one `{"kind":"project"}` per round, with X×N:
- 185×7 (186–192, 234,469 notices)
- 192×6 (193–198, 187,532)
- 198×8, 206×8, 214×8 (re-parses 216, which is harmless), 223×8, 231×8, 239×8, 247×8, 255×8, 263×6
- plus 270+ only if M0 says so.

In total about 2.1M notices in 11 rounds, roughly 12–16 h of queue (509's nine rounds took one day).

**Each round:**
- the reparse names the expected fetches;
- `unmatched` ≈ 0, `re-keyed` = 0, `now failing` = 0;
- the project log lines `[project] incremental: N changed notices`, `[project] legacy closure: … → N` (< 500k), and `compare: N verified, M corrected`;
- no daily or heavy job alongside.

**After:**
- `sweep-orphan-orgs` dry (510-list side effect);
- a `data-quality` run: section 1 text `value` %, section 16 (3427333 gone, no new text rows);
- exhibits 3427333, the 3871014 tender (four `awarded`, value unchanged), 4040506 / 4300188 / 4101747;
- Verify: `/v1/tenders/3427333 | jq .value`. Open: 35,076,200,000 EUR. Done: 35,076,200 EUR (A) or `null` (C).
- `SELECT COUNT(*) FROM notice_amounts WHERE notice_id BETWEEN 3401288 AND 3425872 AND field_id='TED-VAL_TOTAL' AND section_id LIKE 'RES-%'`: 0 before, M1's prediction after.

### 7. Risks

1. **Money is user-visible.**
   - Under A, heads gain, rise or flip currency at the scale in §0.2. A V.4 slip below €1 bn wins by MAX (about 7 of every 19 10ᵏ pairs point that way).
   - `amounts` gains a lot-null row per contract figure; analysts who sum it double count.
   - Every award with a figure gains `awarded`. A pairing bug would put a wrong figure on an award, so the 1:1, no-nesting and agreement guards are deliberately conservative.
2. **Scale rules.**
   - New partners refuse uncorroborated pairs at ≥ €1 bn: k ≥ 3, and ×100 with no exemption, because `head_lots = 0`.
   - A genuine II.2.1 beside a V.4 typed in thousands would be refused. 471's recorded false-positive rate is 1 in 65, so adjudicate all of them in M2.
   - A CN estimate in the head corroborates a k ≥ 3 figure and keeps it.
3. **Pairing gaps (losses, never misattribution).**
   - The 2004/05 per-contract V.1.2 figures read as scope 1, a pre-existing residual.
   - A missing `CONTRACT NO` heading, multi-winner blocks and multi-lot contracts all store nothing.
4. **Performance.**
   - Parsing adds one flatten-with-line-starts in place of the old flatten, plus one value scan per single-name block. Linear; test 14.
   - The reparse covers ~2.1M notices in 12 folds.
   - About 450–500k `tender changed` corrections go out on the change feed; announce them in CHANGELOG.
5. **Name side effects.** 510's lists change names in the parse layer (see §0.6), so check the orphans afterwards.
6. **Rollback.** An old binary projects the new parse the same way under A. Under C, a rollback reverts to A's projection.
7. **Daily-ingest race.** None: no `publication_id` change, so issue 404's race cannot occur. Still keep the queue clear.

### Critical files for implementation
- /home/user/tender-db/crates/ingest/src/text/parse.rs
- /home/user/tender-db/crates/ingest/src/project.rs (`read_legacy_results` 6057, `amount_target` 7329, `NoticeState::read` 4415, test harness 10232)
- /home/user/tender-db/crates/store/src/canonical.rs (`head_value_eur_cents_with` 3182, `ScalePartners` 3411-3993, tests 35086)
- /home/user/tender-db/crates/ingest/tests/text.rs
- /home/user/tender-db/.scratch/tender-db/issues/491-a-text-era-min-max-range-is-read-as-one-run-together-figure.md (record the measurement, the A/C verdict and the drain)

