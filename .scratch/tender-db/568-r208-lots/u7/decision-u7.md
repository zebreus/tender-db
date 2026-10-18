# 568 unit 7: decision. ADD four refusal guards (G3–G6) to the cross-notice match, drain with 570 (2026-10-18)

**Decision: BUILD G3 + G4 + G5 + G6.** The 568 match stays as built: rule steps 1–9, G1 and G2. Four refusal guards
run on top of it. Each one keeps a block on the Tender, as before 568, when the award block's own text or numbering
contradicts the number it prints:

- **G3, block-level.** The block's own title names two or more lots.
- **G4, notice-level.** The block's title names a lot other than its printed one, so the notice's numbers are not the
  contract notice's numbering.
- **G5, block-level.** The block's title is another lot's title.
- **G6, block-level.** The block prints two or more lot numbers.

Titles are refusal evidence only. They are never a key, and step 9 stands.

A value guard is refused, with or without the title guard. Neither "award ≥ k × lot value" nor "title names several
integers" is adopted (evidence below). The guards ship in one build (unit 8) and drain in ONE refold together with
570, over `TED-LOT_NO` + `TED-LOT_NUMBER`.

**The read-back bar changes, and this decision says why.** After the guards, the measured residual is 5 of 180
uniformly drawn Tenders with a wrong block. A fresh 30 at "30/30, no wrong block" would then pass with probability
0.43. So the bar becomes:
- zero wrong blocks of a guarded shape;
- the five recorded residual shapes in at most 2 of 30 Tenders;
- any new shape reopens the decision.

Evidence:
- `u7/measurement-u7.json`: per block, per candidate, with 95 % intervals.
- `u7/g3_port.py`: the decided rule, replayed on `u7/u7-cache.jsonl.gz`, the cached tenders and every chain notice of
  the three sets.
- `u7/lot_designators.py`: the title reader, with 35 self-test cases.
- The skeptic-final per-block tables: `u7/skeptic_u1..6/`.

## What was measured

| set | what | matched blocks | right | multi-lot | other wrong |
|---|---|---|---|---|---|
| s150 | salted 150 of 2594's 156,123 corrected (`sha256("568u7-"+id)`), skeptic-final | 972 | 904 | 6 (+1 on a 1-lot Tender) | 61 |
| f30 | the failed read-back's fresh 30 | 209 | 202 | 7 (8327340) | 0 |
| m77 | unit 1's 77 measured Tenders (control) | 656 | 656 | 0 | 0 |

- **The replay matches what is served.** The 568 replay (`../cross_notice_port.py`) on the cached parsed values
  agrees with the served lot of every matched block: s150 972/972, f30 209/209 and m77 692/692 nulls included. The 17
  s150 rows it leaves `None` while the API serves a lot are later notices with their own lot sections: 5399590's
  20595669 and 8313642's 19187804. Those take 567's same-notice keys and are out of scope.
- **"Wrong" is two shapes, not one.**
  - (1) **One award block for several lots, printing one of them.** 8327340 ×7, 5020611 ×3, 5001424 ×1 (six
    `LOT_NUMBER`s in one block) and 5267907 ×2 (the CN's two lot sections are identical "47 lots" placeholders; the
    lots appear only in the winner fields).
  - (2) **A `LOT_NUMBER` that is not the contract notice's lot number.**
    - Contract counters whose own titles name the real lot: 5051007 (16 of 19), 5267810 (31 of 32), 8364864 (2) and
      5020611 RES-2.
    - CAN-only renumbering: 4854639 ×4 and 4975428 ×1.
    - A CAN of another procedure chained onto the Tender (4861483 ×4, which is issue 364's).
    - 4968281 ×1 and 5364153 RES-4 ('3a', a lot that exists only in corrigendum prose).
  - 4514627's 17-lot contract sits on the Tender's only lot, which is the whole 182-lot procedure. That is the
    Tender's own scope, the same reading the skeptic gave 4485310, so it is not counted wrong.
- **The deployed rule is not at unit 1's bound.** **12 of 180 uniform Tenders (s150 + f30) carry a wrong block:
  6.7 %, 95 % 3.5–11.4 %.** By block it is 74 of 1,181 (6.3 %). Unit 1 bounded it at ≤ 0.57 % of blocks and ≤ 5.1 %
  of CANs on a frame that met none of these shapes: 0 of 70 CANs, which has p ≈ 0.008 at the uniform rate. Unit 1's
  frame was one OJ S day a month, 2012–2015, multi-number CANs. The uniform draw of what 568 actually changed is the
  rate this decision uses.
- **None of the 77 is a multi-lot contract.**
  - No block of the 77 names two or more lot designators, even before filtering. No block prints two numbers.
  - 16 blocks have an award ≥ 1.5 × their lot's CN value: 5284511 × 15, each titled "Grupa k" with an estimate equal
    to its lot's value, and 5086044 RES-5. Each names exactly one lot. These are value anomalies, and they are why a
    value guard fails.
  - The guards refuse **0 of the 77's 656 matched blocks**.

## Candidate guards

Catches are over the 13 multi-lot blocks and the 61 other-wrong blocks. False refusals are right blocks refused, out
of 904 + 656 + 202 = 1,762. Intervals are Clopper–Pearson 95 %. Every guard is off on a Tender whose inherited lots
print fewer than 2 distinct numbers (G1's exemption).

| guard | multi caught | other wrong caught | FR s150 | FR m77 | FR f30 | FR all |
|---|---|---|---|---|---|---|
| G3a: title names ≥ 2 distinct integers the inherited lots print | 7/13 | 22/61 | 30 | 66 | 0 | 5.45 % (4.44–6.61) |
| **G3c: title names ≥ 2 distinct lot designators (lot / part / position phrase, any language) the inherited lots print** | **10/13** | 0/61 | 0 | 0 | 0 | **0 % (0–0.21)** |
| G3d: award ≥ 1.8 × matched lot's CN value | 7/13 | 0 | 0 | 14 | 1 | 0.85 % (0.48–1.40) |
| G3d: award ≥ 2.0 × | 6/13 | 0 | 0 | 13 | 1 | 0.79 % (0.44–1.33) |
| G3d: award ≥ 2.5 × | 6/13 | 0 | 0 | 10 | 0 | 0.57 % (0.27–1.04) |
| G3b: G3a or award ≥ 2.0 × | 7/13 | 22/61 | 30 | 79 | 1 | 6.24 % (5.16–7.48) |
| G3c or award ≥ 2.0 × | 10/13 | 0 | 0 | 13 | 1 | 0.79 % (0.44–1.33) |
| G4 block: a title designator that is not the printed number and not the matched lot's own | 2/13 | 48/61 | 0 | 0 | 0 | 0 % (0–0.21) |
| **G4 notice: any matched block of the notice contradicts, so none is matched** | 3/13 | **50/61** | 4 | 0 | 0 | 0.23 % (0.06–0.58) |
| **G5 block: title = exactly one OTHER lot's title** | 0 | **9/61** | 0 | 0 | 0 | 0 % (0–0.21) |
| G5 notice | 0 | 36/61 | 13 | 0 | 0 | 0.74 % (0.39–1.26) |
| **G6: block prints ≥ 2 distinct numbers** | **1/13** | 0 | 0 | 0 | 0 | 0 % (0–0.21) |
| **FINAL = G3c ∪ G4 notice ∪ G5 block ∪ G6** | **11/13** (85 %, 55–98) | **53/61** (87 %, 76–94) | 4 | 0 | 0 | **0.23 % (0.06–0.58)** |

- **G3c is the multi-lot guard.** It catches 8327340's 7 blocks and 5020611's 3, with no false refusal in 1,762.
  Keeping only designators that the inherited lots print changes nothing on these sets, and it is kept as a safety
  margin:
  - "Część 2- 80 miejsc." reads lot 2 by the dash-spacing rule, and the filter would drop 80 anyway;
  - on a CN printing "00", "Lots n° 11132 …" is filtered out;
  - "lots 1 à 195" against a 2-lot CN keeps only lots 1–2.
- **G3a (raw integers) is refused.** It catches fewer multi-lot blocks (7) and refuses 96 right blocks. Examples:
  "Programa A1 … de 3 a 17 años" (5693088 × 13), "Pakiet I – załącznik nr 2.1" (5209588 × 19), "groupes 1+2+3"
  (5071935).
- **Value guards are refused at every k.**
  - 8327340's seven multi-lot ratios run from 1.89 (RES-6) to 25.9.
  - The right 17840150 RES-1 sits at 1.75.
  - Right blocks reach 237 × (5284511 RES-25 "Grupa 63"), 15.9 × and 12.4 ×, and 5733979 RES-2 sits at 2.02.
  - Every multi-lot block a value guard catches is one G3c already catches. The value guard adds no catch and costs
    10–19 false refusals.
  - It also needs a same-currency lot value. 5625827's PLN award against an EUR lot reads 4.36 ×.
- **G4 is notice-level, by its evidence.**
  - Every notice where it fires (5051007, 5267810, 8364864, 5020611) numbers its awards with a contract counter, so
    no number in it is evidence.
  - The notice level adds 5267810 RES-8, a bare "Pakiet", and RES-7, "Pakiet 1A", whose designator no lot prints.
  - It costs 4 blocks that are right only by coincidence: 5051007 RES-1..3 and 5267810 RES-1, where the counter equals
    the lot until the first unawarded lot.
  - G4 reads only blocks the 568 rule matched. 5218711's CAN-only 5/6/7 for sub-lots 3.B–3.D match nothing and say
    nothing about its other numbers. Counting them would cost 20 right blocks.
- **G5 is block-level.**
  - At the notice level it would add only 4854639 RES-9/10 for 13 false refusals: 4854639's Cranes, Lifts and HV, and
    4975428's other six, 5267810 RES-1.
  - One language agreeing with the matched lot is agreement. 8303947 RES-2's Maltese variant copies lot 1's title; its
    other 22 languages are lot 2's.
  - A title that is the Tender's own title never counts. 5395010 copies the procedure title into a lot title.

## The rule (unit 8 builds exactly this)

Where: `crates/ingest/src/project.rs`, `match_inherited_lots`, called from `fold` exactly where it is now. The
guards apply only to blocks the 568 rule would match:
- `lot_key None` after 567;
- a printed number;
- an award notice with no lot section of its own;
- G2 not set;
- G1 not firing.

They never touch 567's same-notice keys, eForms, FTS or 570's remap.

Let N be the normalised numbers the inherited `Lot`s print: `normalise_lot_number` of `LotState::published_lot_no`,
the set G1 already builds. **If |N| < 2, no guard fires** (G1's exemption: on a 1-lot Tender the one lot is the
Tender's scope). For each numbered block, let `p` be its normalised printed number and `l` the lot `lookup(p)`
tentatively names (568 steps 3–4).

1. **G4, notice-level, evaluated first.** Take a block with a tentative `l` whose own-title designators, kept to those
   whose head is in N (call that set Dn), are non-empty. If `p` is not the head of any designator in Dn, and no
   designator in Dn is among the designators of `l`'s own title or description facts, the block contradicts. If any
   block contradicts, **no block of the notice is matched**.
   - The agreement through `l`'s own text is what keeps 5071935 right. Its blocks say "Lot 06 désamiantage" where the
     CN's lot printing 3 is itself titled "lot 06 DESAMIANTAGE".
2. **G6, per block.** The values of the block's numbering field (its `TED-LOT_NO`, else its `TED-LOT_NUMBER`, the
   field `award_block_lot_number` reads) normalise to two or more distinct numbers. The block is not matched.
   - Issue 568 risk 7 is now measured: 5001424 prints 1–6 under one award, one winner and one value.
3. **G3, per block.** Dn holds two or more distinct designators. The block is not matched.
   - A compound such as `Pakiet 8 poz. 6` is ONE designator.
4. **G5, per block.** None of the block's title keys equals a title key of `l`. Some block title key that is not a
   title key of the Tender equals a title key of exactly one other inherited `Lot` that prints a number. The block is
   not matched.
5. Otherwise `lot_key = l`, as now. A refused block keeps `lot_key None` and its `published_lot_no`. It is **never**
   re-keyed to the lot its title names: 8364864's "Zadanie 2" printed 1 stays on the Tender, not on LOT-2. Frozen at
   the award's version, as 568 step 7.

**Text fields.** The block's own `TED-CONTRACT_TITLE` (R2.0.8), `TED-TITLE_CONTRACT` (R2.0.8 variants) and
`TED-TITLE` (R2.0.9 `AWARD_CONTRACT/TITLE`). Every value, every language and every ordinal, taken only from values
directly on the `RES-n` section, never from its Organization children. Not read:
- `TED-CONTRACT_NUMBER` / `CONTRACT_NO`. They hold file and contract numbers ("53/14/01 lote 2", "HIL_A_SYS0173
  Los 1", "13087"); adding them cost a false refusal and caught nothing.
- `TED-ADDITIONAL_INFORMATION`, which holds scope prose ("1)–5)"). It is neutral on the sets and left out.
- Winner names, and the notice-level title.

The lot side reads the lot's `title` and `description` facts (from `TED-LOT_TITLE`, r209 `TED-TITLE`,
`TED-LOT_DESCRIPTION`, `TED-SHORT_DESCR`). The Tender side reads the version's Tender-scope `title` facts.

**Designators** (`u7/lot_designators.py` is the reference; the Rust reader copies its `SELFTEST` cases):
- **Matching.** Case-insensitive after NFC. A keyword is a whole word, with no letter on either side, followed by an
  optional mark (`nr`, `no`, `n`, `num`, `nos`, `nrs`, `číslo`, `č`, each with an optional `.°º:`, or `№`, `#`, `n°`,
  `nº`), then a list.
- **Keywords:**
  - lot: `lot lots lote lotes lotto lotti lotul loturi(le) los lose losnr teillos fachlos`
  - part: `part parts partie parties parte partes partida`, `část části časť`, the unaccented `cast casti`,
    `część części` (and spellings with missing accents), `dalis dalys dalies dalių`, `delaftale(r) delkontrakt(et)
    delområde`, `osa osat`, `τμήμα(τα|τος)`, `sklop(a|i|ov)`, `grupa grupe grupy`, `skupina skupiny`,
    `perceel percelen`
  - position / package / task: `pozycja pozycje pozycji pozycjy poz pozice`, `позиция позиции`,
    `обособена/обособени позиция/позиции`, `pakiet pakiety pakietu pakietów pakietach pakietami`,
    `zadanie zadania zadań zad`, `položka položky`
  - Postfix keywords, which take the number BEFORE them: `rész része részek` ("IV. rész"), `daļa`, `dalis`, `osa`,
    `část`, `część`.
- **Deliberately not keywords:**
  - bare `del`: Swedish and Danish "Del 1" collides with ES/IT/PT "del" + a date, such as "del 31.7.2008";
  - `Teil`, `Gruppe`, `groupe(s)`: 5071935's "groupes 1+2+3" are groups inside one lot;
  - `tranche`, `phase`, `Vergabepaket`, `Rejon`, `Sektor`, `obszar`, `Programa`;
  - `item` / `position` as a lot keyword: they are accepted only as the second half of a compound.
  - A missing keyword costs recall, never precision.
- **Number:** digits, with leading zeros dropped, an optional `.digits` sub-number ("02.1" → "2.1") and an optional
  single trailing letter, lower-cased ("3a", "1A" → "1a"). Or an UPPER-CASE Roman numeral not followed by a letter,
  read as its value. A Roman token only continues a Roman list.
- **List separators:** `, ; / + & 、`; the conjunctions `i и oraz a et and und e y en és ir ja och og și si ou`; a tight
  dash `-–—`, or a dash with spaces on BOTH sides (so "2- 80 miejsc." stops at 2); the range words
  `do to à bis až au tot t/m al`.
- **Range:** exactly two numbers joined by a dash, or any two joined by a range word, with a < b ≤ a + 300, expand to
  a..b. A dash chain of three or more numbers is a list ("11132-11193-11194").
- **Compound:** a one-number lot clause directly followed by a position clause (`poz./pozycja/pozice/položka/item/
  position` + one number) is one designator (a, b) with head a.
- **Title key (G5):** NFKD, combining marks removed, lower-cased, one leading designator dropped ("5. Snagging",
  "Lot 1 : Dents"), runs of non-word characters collapsed to one space, trimmed.

## What the build needs

- **The award block's title is not in the fold today.**
  - `TED-CONTRACT_TITLE`, `TED-TITLE_CONTRACT` and `TED-TITLE` are in the parsed values of the `RES-n` section. The
    content endpoint shows them, and `read_legacy_results` iterates them.
  - But nothing keeps them. `RawLotResult` has no text field. `LotResultState` and `Round` carry none. Issue 343's
    filter in `state_of` deliberately keeps the award block's title out of the Tender's facts ("nothing is lost by
    not filing this one anywhere").
  - **The parse layer needs nothing: no re-parse, no new field id, no epoch.** It is an ADR-0017 R3 refold, like
    567, 568 and 570.
- **`read_legacy_results`** reads the three title fields (`NoticeValue::Text`, `value.section_id == owner`). It
  computes, per block:
  - `named: Vec<Designator>`;
  - `title_keys: Vec<String>`;
  - `several_numbers: bool`, from the `lot_refs` it already collects.
- **Carry, fold-input only.** The block evidence goes from `RawLotResult` into a side table on `NoticeState` keyed by
  result key, for example `award_text: BTreeMap<String, AwardText>`, and into `BucketRow`'s snapshot and
  `to_notice_state`.
  - Add the field with the existing postcard lifecycle note. The bucket directory is wiped at the start and end of
    every run, so old bytes are never read.
  - **Not** on `LotResultState` or `Round`. They are carried into every later version, and `crates/store/src/
    canonical.rs` needs no change. The evidence is read once, at the award's version, and never written.
- **`match_inherited_lots(round, inherited, tender_facts, award_text)`.** N already exists for G1. The lot side
  computes designators and title keys from `LotState.facts` (`Fact::Text` with field `title` / `description`). The
  Tender's title keys come from the version's `facts`. `LegacyLotNumbers` is unchanged.
- **`lot_designators` and `title_key`** are hand-written beside `normalise_lot_number`. ingest has no `regex`
  dependency, and the parser's look-arounds are not `regex`-crate syntax anyway.

## Residual: what the guards do not catch (recorded, not built)

After the guards, 5 of the 180 uniform Tenders carry a wrong block: **2.8 %, 95 % 0.9–6.4 %**. That is 10 of 1,181
blocks (0.85 %), against 6.7 % of Tenders (12/180) and 74 blocks today.

| shape | case | why not guarded |
|---|---|---|
| CAN-only renumbering, title paraphrased | 4854639 RES-9/10 "Painting" on LOT-5 "5. Snagging" (its lot is "6. Repair painting") | Containment matching ("painting" ⊂ "repair painting") refuses 5 right blocks (5451989, 5106209 × 2, 4866521, 4748733). The notice level costs 6 right blocks. G5 does take RES-7/8 "Snagging". |
| single-block counter, no designator in its title | 4968281 "2 Solo-Niederflubusse ca. 15 m", LN 1 = CONTRACT_NO 1 = ITEM 1, its lot is 2 | A counter guard (`LN = CONTRACT_NO`, 1..n < \|N\|) refuses 16 right blocks (0.9 %). |
| a mis-chained CAN of another procedure | 4861483 CAN 17286230 (file 40132230 against the Tender's 40132200) | It is a weld. Issue 364 owns it, and it was misattributed to this Tender before 568 too. |
| a lot that exists only in corrigendum prose | 5364153 RES-4 "3a Leki …" on LOT-3, lot 3a split off by corrigendum 18188432 | No keyword. "3a" is its own lot only in prose; the skeptic's softer reading is that LOT-3 is its parent. |
| duplicated placeholder lot sections | 5267907: the CN's two lots are both "Produits frais et surgelés / 47 lots"; the blocks name annex lots only in their winner fields | No text or value. A guard on "identical inherited lot texts" refuses 54 right blocks (4544195, 4903987, 6519230 …). No value moves, only winners. |

Expected effect on the 156,123 corrected:
- 7 of 150 Tenders are touched (4.7 %, 1.9–9.4 %), about 7.3 k (3.0–14.6 k).
- 5 of 150 revert fully to tender scope (3.3 %, 1.1–7.6 %), about 5.2 k: 5051007, 5267810, 8364864, 5020611 and
  5001424.
- About 63 k blocks go back to `lot: null`. That point estimate is dominated by two counter CANs.
- About 4 k of those blocks were right by coincidence.

## Units

8. **Build** (one commit, `crates/ingest/src/project.rs` only; `git diff` it before staging, per CLAUDE.md
   "Committing"). Named tests in `project::tests`, built from parsed values (`/v1/notices/{id}/content`, as 567, 568
   and 570 did; all are in `u7/u7-cache.jsonl.gz`):
   - `an_award_naming_several_lots_stays_on_the_tender_8327340`: CN 17694913 + CANs 17821481, 17830774, 17840150.
     - 17821481 RES-1/2/5/6/7 and 17830774 RES-1/2 give `None`, with `published_lot_no` unchanged.
     - 17821481 RES-3 → LOT-24, RES-4 → LOT-32, and 17840150 RES-1 → LOT-2.
     - `stated_totals[].lots_awarded` no longer names LOT-5/28/3/21/12/34/1 for these.
   - `a_notice_numbered_by_a_contract_counter_matches_none_5051007`: CN 17548991 + corrigendum 17556401 + CAN 17689000
     give 19 × `None`, including RES-1..3, whose titles agree.
   - `roman_pakiet_lists_and_a_contradicted_pakiet_5020611`: CN 17490851 + CAN 17604929 give 4 × `None`. G3 fires on
     RES-1/3/4, and G4 on RES-2 "Pakiet IV" printed 2.
   - `a_compound_pakiet_position_agrees_with_its_pakiet_4945774`: "Pakiet 42 poz.3" printed 42 stays on LOT-42
     (CAN 17426530). Against it, 5267810 (CN 17979642 + 17996722 + CAN 18199880) gives 32 × `None`, including the
     bare "Pakiet" (RES-8) and "Pakiet 1A" (RES-7).
   - `a_title_designator_the_matched_lot_also_carries_agrees_5071935`: CAN 17760667's "Lot 06 désamiantage" printed 3
     stays on LOT-3, and "Lot 10 sols" printed 4 stays on LOT-4, because 10 is no printed number. All 13 are
     unchanged.
   - `a_block_printing_several_lot_numbers_stays_on_the_tender_5001424`: CN 17452649 + CAN 17550359 give `None` (G6).
   - `an_award_titled_as_another_lot_stays_on_the_tender_4854639`: CAN 17286567 RES-7/8 "Snagging" give `None`, and
     RES-1..6 and 9/10 are unchanged. Also 4975428 CAN 18326140 RES-2 gives `None`, while its other 6 are unchanged.
     Controls:
     - 4889588's "Toksyna botulinowa" on lots 3 and 9 keeps both;
     - 5395010's procedure title copied into a lot title keeps;
     - 8303947 RES-2, whose Maltese copies lot 1, keeps LOT-2.
   - `a_contradicting_block_with_no_lot_does_not_refuse_the_notice_5218711`: CAN 18184190's 20 matched blocks are
     unchanged, and 5/6/7 stay `None`.
   - `the_text_guards_never_fire_on_a_one_lot_tender`: 4485310 ("Lots n° 11132-…", printed "00") and 4514627 ("lots
     no1, 2, 5, …") keep LOT-1.
   - `counts_quantities_years_and_packages_are_not_lot_designators`. Each of these is unchanged:
     - 5693088 "Programa A1 … de 3 a 17 años";
     - 4667265 "Część 2- 80 miejsc.";
     - 5476961 "с 4 обособени позиции: Обособена позиция № 1";
     - 6328307 "w latach 2019-2020";
     - 4846721 "Vergabepaket 2";
     - 8022925 "obszarze II";
     - 5106209 "IV. rész".
   - `lot_designators_reads_lists_ranges_roman_and_compounds`: the 35 `SELFTEST` cases of `u7/lot_designators.py`.
   - `the_unit7_guards_are_identical_on_the_bucketed_path`.
   - All of 568's and 570's existing tests stay green unchanged, G1 and G2 included.
   - **Port parity:** `python3 -I u7/g3_port.py` must give `FINAL` 11/13 multi, 53/61 other, and 4 false refusals,
     with the 77's 656 matched blocks unchanged.
   - **Docs:** extend 568's `docs/operations.md` runbook block, and its `/docs` caveat sentence pinned by
     `assert_says`, with: "An r2.0.8 award whose own title names several lots, names another lot than its printed
     number, or carries another lot's title, or that prints several lot numbers, stays on the Tender."
9. **Gate, review, deploy.**
   - Run `ops/check.sh` and read `GATE-EXIT=`.
   - Adversarial review: the four guards, the |N| < 2 exemption, G4 reading only matched blocks, the "never a key"
     rule, and keyword false positives.
   - Deploy in a queue gap.
10. **Drain, combined with 570.** One `refold-fields` over `["TED-LOT_NO","TED-LOT_NUMBER"]`,
    `"tables":["notice_ids"]`. Every block the guards can refuse sits on a notice carrying one of the two fields.
    - dry with `"expect":1` gives N (≈ 2.1 M, per 570). Then wet with `"expect":N`; it queues its own `project`.
    - Expect a FULL fallback, more than 4.3 h.
    - Baseline first: the ≥ €1 bn band and the 16 path-V heads (`drain/baseline-pre568u7-570.json`).
    - Run it clear of the 07:35 UTC daily chain, and before 551's B3 parity and B5 flip.
11. **Read back.**
    - (a) D7: `verified + corrected` ≥ the wet run's `stamped N`, and no `projection_epoch = 0`.
    - (b) **8327340:** its 7 blocks serve `lot: null`; 17821481 RES-3 → LOT-24, RES-4 → LOT-32, 17840150 RES-1 → LOT-2.
    - (c) **The 150's 61 refused blocks** serve `null`:
      - 5051007 × 19 and 5267810 × 32;
      - 8364864 × 2, 5020611 × 4 and 5001424 × 1;
      - 4854639 RES-7/8 and 4975428 RES-2.

      The other 911 matched blocks are unchanged, except 570's named movers.
    - (d) **The 77:** 692 of 692 unchanged. The guards refuse none, and 570's port names any 570 mover.
    - (e) **A fresh salted 30.**
      - Population: the same 2594 corrected population (`readback/population.json`), salted with
        `sha256("568u7-fresh30-"+id)`, at least 10 outside 2012–2015, read after the drain.
      - **Bar:** 0 wrong blocks of a guarded shape. Wrong blocks of the five residual shapes above in at most 2 of the
        30 Tenders. A wrong block of any other shape reopens the decision.
      - At the measured residual the bar passes with P = 0.95, and 0.70 at the interval's upper end. "30 of 30" would
        pass with P = 0.43 (0.14 at the upper end), failing a correct build more often than not on shapes already
        known.
    - (f) Band / path-V diff against the baseline. Every mover is read as 568-u7's (a lot sum shrinking back) or 570's.
    - (g) 570's own read-back items, unchanged.

## Risks

1. **Keyword false positives.** A lot word used in another sense, followed by a number, could refuse a right block.
   Measured 0 of 1,762 for G3 and G5, and 4 of 1,762 for G4, all four in counter notices. Upper bounds are 0.21 % and
   0.58 %. A refusal is today's pre-568 behaviour, never a wrong lot.
2. **Language gaps.** Languages or spellings missing from the lexicon (bare Swedish "del", Irish, Maltese) make
   G3/G4 silent, which is today's behaviour. Add words only with a measured control.
3. **G4 discards coincidental rights.** In a counter notice the blocks before the first skipped lot are right, and G4
   refuses them: 4 of 4 in the sample. That is deliberate: the notice's numbering is not the CN's.
4. **The residual.** 2.8 % of corrected Tenders (0.9–6.4 %) keep one wrong block, in the five recorded shapes. 4861483
   belongs to 364. 5267907 moves winners only.
5. **570 interplay.** G2 now compares at the remapped key. G3–G6 do not depend on 570. Both drain in one refold, so
   the read-back attributes per block: 568-u7 movers go lot → null on award notices without own lot sections, while
   570 movers remap section and result keys.
6. **Change feed.** About 7 k Tenders × a few lot rows on top of 570's burst.
7. **Value sums.** 8327340-style blocks leave their lot. `LotSums` and path W lose inflated per-lot awards: 709,431.63
   BGN off LOT-5, for example. Path V's read (f) covers it.

## What the issue records

The status line of issue 568 should read:

> Status: ready-for-agent — UNIT 7 DECIDED 2026-10-18: BUILD G3–G6 on the cross-notice match (G3 title names ≥ 2
> lots; G4 a title names another lot than its printed number → the notice matches none; G5 title = another lot's
> title; G6 ≥ 2 printed numbers; none on a Tender whose lots print < 2 numbers; titles refuse, never key). Measured on
> the salted 150 + fresh 30 + the 77: catches 11/13 multi-lot and 53/61 other-wrong blocks, false refusals 4/1,762
> (0.23 %, ≤ 0.58 %), the 77 untouched. Value guards refused at every k. Deployed rule today: 12/180 uniform Tenders
> carry a wrong block (6.7 %, 3.5–11.4 %); after G3–G6: 5/180 (2.8 %, 0.9–6.4 %), five recorded residual shapes.
> NEXT: unit 8 build → gate → deploy → ONE refold with 570 (TED-LOT_NO + TED-LOT_NUMBER) → read back (8327340's 7 on
> the Tender; the 150's 61 named; the 77 unchanged; fresh 30: 0 guarded-shape wrongs, residual ≤ 2/30).

**`done` line:** an award block printing n on a Tender whose contract notice printed lot n serves that lot. The
recorded exceptions stay `lot: null` by design: those of the 568 decision, and also an award whose own title names
several lots, names a lot other than n, or is another lot's title, or which prints several numbers. **Residual,
recorded:** the five shapes in the table above.
