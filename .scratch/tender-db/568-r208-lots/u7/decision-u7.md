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
  the three sets. Its labels are read from the skeptic tables, so extract `u7/raw-cache.tar.gz` into `u7/` first. The
  extracted folders are gitignored locally through `.git/info/exclude`.
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

## Amendment (unit 8, 2026-10-18)

**The owner adopted the decision critic's findings** (`u8-oos/critic-and-review.json`, workflow wf_2f6dde5b-372).
**Round 2, the same day:** the port and an independent verifier agreed block for block, and the owner then adopted
refinements **R1** and **R2** and the amender's **quantity-word reader rule**. This section describes that final rule,
FINAL_AMENDED. The round-1 rule, without R1 and R2, is kept as the measured row FINAL_AMENDED_R0.

This section changes "The rule", "Designators" and the read-back bar above. Where they disagree, this section wins. The
guards stay, the drain plan stays (one refold with 570), and titles still refuse, never key.

Evidence:
- `u7/g3_port.py` replays three rules side by side on the same replay:
  - FINAL, as decided, with the frozen unit-7 reader `u7/lot_designators_u7.py`;
  - FINAL_AMENDED, R1 + R2 included (`apply_amended()`'s defaults);
  - FINAL_AMENDED_R0, the round-1 rule (`agree='designator', confirm='heads'`), plus R0 + R1 alone and R0 + R2 alone
    as attribution rows.
- Every old candidate row and the old summary are byte-identical to `measurement-u7.json`: 1,837 of 1,837 blocks,
  `amended.parity_with_measurement_u7`.
- `u7/measurement-u8.json`: the in-sample result. `measurement-u7.json` is not rewritten.
- `u8-oos/amended_oos.py` → `u8-oos/amended_oos.json`: the OOS 50 with the skeptic-final labels. FINAL replayed there
  equals the skeptic runs' `guards_fired` on all 419 blocks (`consistency_problems: []`).
- `u7/lot_designators.py`: the amended reader, with 77 SELFTEST cases (35 unchanged, 42 new) and 10 `title_key`
  cases (`TITLE_KEY_SELFTEST`). `title_key` moved there from `g3_port.py` byte for byte.
- The round-1 independent verification (`scratchpad/u8v/`) rebuilt the guards from the rule text alone. Every outcome
  matched, R1, R2 and R1 + R2 included.
- Round-2 extras in `scratchpad/u8a/`:
  - `r2_exposure.py` → `r2_exposure.json`: per-notice contradiction counts, and the blocks held only by the notice
    level;
  - `r2_synthetic.py`: the synthetic test chains below.

### What changed and why

1. **G4 is block-level, and goes notice-level only on 2 or more contradicting blocks. Title-confirmed blocks are
   kept.** (Critic finding 0.)
   - The notice level bought only 5267810 RES-7 ("Pakiet 1A") and RES-8 (a bare "Pakiet").
   - It refused every right block of a counter notice, and each of those titles names exactly its own printed
     number: 4 in-sample (5051007 RES-1..3, 5267810 RES-1) and 22 OOS (8286472 RES-1..8, 5416534 RES-1..14).
   - So FINAL's OOS false-refusal rate was 22 of 319 (6.9 %, 4.4–10.3 %), against the 4/1,762 the decision stated.
     The block-level interval treated clustered blocks as independent.
2. **The reader** (critic findings 1 and 4). Each change fixes a false-positive path that was observed or probed:
   - 4667265's own CN titles, "Część 1 – 50 miejsc", read as lots 1..50.
   - OOS 8286472's "(pakiet 8 pozycji) … - zadanie 4" read as [8, 4], so G3 hit 3 right blocks.
   - OOS 5416534's "Pakiet 16 zad.1" read as two lots.
   - The Spanish article: "de los 4 ascensores del lote 2" read as [4, 2].
   - Round 2 adopted the quantity-word rule (d), for three probes that (a)–(c) left harmful.
3. **G3 counts heads** (finding 3). "Pakiet 8 poz. 6, Pakiet 8 poz. 7" printed 8 names one lot.
4. **Titles are read per language** (finding 5). G3/G4 took the union of every language while G5 accepted one
   agreeing language. A single mistranslated language could create a multi-lot read or a contradiction.
5. **The text-field spec is clarified** (finding 6), so port parity cannot fail for reasons outside the guards.
6. **A new read-back bar** (findings 2 and 7). The old bar's "any new shape reopens" clause put P(pass) back at about
   0.5–0.65, and its population could not draw the blocks that 570's narrower G2 newly matches.
7. **A /docs caveat sentence** (finding 8).
8. **R1 and R2 (round 2).**
   - **R1:** G4 agreement compares heads. Under the round-1 rule, a correctly numbered CAN block "Pakiet 16 zad.2"
     against its CN lot "Pakiet 16 - pozycja nr 2" (read (16)) contradicted, and the notice level spread. That is
     5416534's counterfactual, where R0 refuses 90 of 105 right blocks.
   - **R2:** only a non-compound designator title-confirms. R0 kept OOS 5416534 RES-16 "Pakiet 16 zad.2" on LOT-16,
     because the compound's head 16 equals the printed counter. That was the one wrong block the amendment newly kept.
9. **The G5 title key is pinned** (the verifier's divergence): the leading designator is dropped only when a
   separator follows it. Both readings fit the old text, so the port's reading becomes the spec, with test cases.

### The rule, as amended (unit 8 builds this)

Unchanged:
- Scope (only blocks the 568 rule would match).
- N, and the |N| < 2 exemption.
- p and the tentative l.
- G6.
- G5's rule.
- Freezing at the award version.
- Never re-keying. A refused block keeps `lot_key None` and its `published_lot_no`, and is never re-keyed by its
  title.

Changed:
- **D. Per language.** Each block title value carries its language: the parsed value's `lang`, or the bucket `""`
  for a value without one.
  - Dn[lang] is the union over that language's values (every ordinal) of the designators whose head is in N.
  - The lot side is not split by language. Dlot is every designator of l's own `title` / `description` facts (not
    filtered by N), and heads(Dlot) is the set of their heads.
- **A. G4.**
  - **Contradicts (R1).** A matched block contradicts when some language has a non-empty Dn[lang], and no language
    agrees. A language agrees when its Dn[lang] has p as a head, or shares a **head** with Dlot. A compound counts by
    its head, as in C.
  - A contradicting block is refused, always (block level).
  - **Notice level:** only when 2 or more matched blocks of the notice contradict. Then every other matched block of
    the notice is refused too, except a **title-confirmed** block.
  - **Title-confirmed (R2).** Some language has a non-empty Dn[lang]. For every language with a non-empty Dn[lang],
    every designator is **non-compound** (a head with no positions) and the set of heads is exactly {p}.
  - Order: contradictions are counted over all matched blocks first, before G6, G3 and G5 run per block. A G3 block
    that contradicts still counts.
- **C. G3.** In some single language, the heads of Dn[lang] hold 2 or more distinct values. A compound counts by its
  head.
- **G5's title key, pinned.** This is NFKD with combining marks removed and the text lower-cased. One leading
  designator is dropped, but **only when one of `. : ) - – —` follows it** (after optional spaces). The designator is
  an optional `lot lots los lote lotto pakiet czesc zadanie part partie perceel cast` plus an optional `no / nr / n° /
  nº`, then 1–4 characters of `0-9ivxlc`. After that, runs of non-word characters collapse to one space, and the
  result is trimmed. `lot_designators.title_key` is the reference, and its `TITLE_KEY_SELFTEST` cases include:
  - "Pakiet 2" → "pakiet 2" (no separator, nothing dropped);
  - "Lot 06 désamiantage" → "lot 06 desamiantage";
  - "5. Snagging" → "snagging";
  - "Lot 1 : Dents" → "dents".
- G6, G5 and the |N| < 2 exemption run exactly as in "The rule". G5 still accepts any one language agreeing with l,
  and still excludes Tender title keys.

### Designators, as amended (`u7/lot_designators.py`)

- **(a) Only a tight dash joins numbers.** A tight dash has no space on either side, and covers both a list and a
  two-number range. A dash with a space on either side ends the list:
  - "Część 1 – 50 miejsc" → [1];
  - "Lot 1 - 5 ordinateurs" → [1];
  - "Część 2- 80 miejsc." → [2].
  - **Accepted recall cost, pinned:** "Lots 1 - 2 - 3" → [1]. In the corpus (53k texts), a spaced dash after a lot
    word is almost always an amount ("dla części nr 1 – 2 500,00 zł"). The real multi-lot shapes sit in notice-level
    fields, which the guards don't read.
  - `\s` includes U+00A0: "Lot 1 – 50 places" written with no-break spaces reads [1], as with plain spaces (pinned).
    The dash set is `-–—`. U+2010, U+2011 and U+2212 are not dashes, as in unit 7.
- **(b) `Los / Lose / Losnr` are keywords only when capitalised or all upper-case** (`Los Lose Losnr LOS LOSE LOSNR`).
  Lower-case `los` is the Spanish article: "de los 2 vehículos" → [].
- **(c) Compounds and position words.**
  - A compound's second half comes after `poz. / pozycja / pozice / položka / item / position`, and now also after
    `zad. / zadanie`.
  - The second half takes a list or a range, and stays ONE designator under the first half's head:
    - "Pakiet 8 poz. 6 i 7" → (8, 6, 7);
    - "Pakiet 16 zad.1" → (16, 1).
  - `pakiet N pozycji|pozycje|pozycja`, with no number after the word, is a count: "(pakiet 8 pozycji)" → [].
  - A position word (`poz pozycja pozycje pozycji pozycjy pozice položka položky позиция позиции`) is a lot keyword
    only when no lot / part / pakiet / zadanie clause has named a number before it in the same title value:
    - "Pakiet 3 (poz. 1-12)" → [3];
    - "Pozycja nr 3 – …" → [3].
    - The Bulgarian keyword `обособена позиция` is unaffected.
- **(d) A quantity word ends the list** (adopted in round 2).
  - A list's last number followed by a quantity or duration word is a count, and is dropped. The word list is
    `COUNT_WORDS`: `szt sztuk osób osoby miejsc lat dni kpl ans années mois jours years months days units pcs
    años meses días anni mesi Jahre Monate Stück Stk Personen let roky ks kusů години месеца броя бр` …
  - Examples:
    - "Lot 2, 3 ans" → [2];
    - "Pakiet 5, 2 szt." → [5];
    - "Szkolenie dla grupy 20 osób" → [].
  - **Removing a number usually withholds a refusal, but it can add one.** The verifier's synthetic case: blocks
    "Lot 2, 3 ans" printed 3 and "Lot 5, 4 ans" printed 4 turn from G3 refusals into two contradictions. Two
    contradictions fire the notice level, which refuses a third, untitled block that the unit-7 reader keeps. Rules
    (a)–(c) can do the same. Nothing like this occurs in the data.
  - Across the 8,792 distinct cached texts (in-sample + OOS: block titles, lot titles and descriptions, notice
    descriptions), it moves only three notice-level descriptions and no block or lot title.
- **Representation.** A designator is a head followed by its positions (`('8', '6', '7')`), not `(head, one
  position)`. The Rust `Designator { head, position: Option }` becomes `{ head, positions: Vec }`. R2's
  "non-compound" means `positions` is empty.
- **Case.** Keywords, marks, compound words and postfix words match case-insensitively, except `Los` in (b). List
  separators, conjunctions, range words and Roman numerals are case-sensitive. This follows the reference, as unit
  8's review fixed (option A).
- **Input** (unit 8's build review). This replaces "after NFC" under Matching above.
  - The text is read as parsed, not normalised. The reader expects NFC; every one of the 348,446 cached values is NFC.
    A decomposed title reads as its decomposed letters, in the reference and in Rust alike: NFD "Część 2" → [].
  - A digit is ASCII `0-9` only (`[0-9]`, not `\d`). Any other decimal digit, fullwidth or Arabic-Indic, is a letter:
    "Lot ３" → []. Both cases are pinned in SELFTEST. The reference used to normalise and to take Unicode `\d`, while
    Rust did neither. That disagreed on such titles, but no cached text has one. Re-running `g3_port.py` and
    `amended_oos.py` on the pinned reference gives byte-identical `measurement-u8.json` and `amended_oos.json`.
  - **Remaining class, recorded and not fixed.** The reference's letter is Python's `\w` minus `[0-9_]`, and its
    space is `\s`. Rust's are `char::is_alphanumeric` minus ASCII digits, and `char::is_whitespace`. On Unicode
    14's assigned code points, they still disagree on 1,442 letters: the `Other_Alphabetic` combining marks (895 Mn
    and 417 Mc, such as U+0345 and the Indic vowel signs) and the enclosed letters (130 So: the circled U+24B6–U+24E9,
    and the squared and negative ones in U+1F130–U+1F189). They also disagree on four spaces, U+001C–U+001F, which
    XML 1.0 cannot carry. Rust counts these marks and enclosed letters as letters, so it can block a word boundary
    the reference allows ("ⒶLot 3": Rust reads [], the reference [3]). Aligning the two would take a generated
    Unicode table on the Rust side. None of the 53,428 distinct texts in the two caches contains any of these
    characters.
- **What the reader change moves.** Of the 8,792 distinct cached texts, 96 read differently (list in
  `scratchpad/u8a/corpus_diff.txt`). By itself it moves **no outcome** of the decided rule: candidate row U8a, the
  decided FINAL on the amended reader, gives 11/13, 53/61 and 4 FR, identical to FINAL.
- **Known reader limits, recorded, not fixed.** These come from the verifier's 70 probes, and each has zero matching
  texts in the corpus.
  - Recall only:
    - lower-case German "los 3" → [];
    - "Lot 3 Personen-Aufzug" → [] (rule d);
    - "Pakiet 16 - pozycja nr 2." → (16): the position after a spaced dash is lost. R1 is what keeps that loss from
      contradicting.
  - Old false positives that stay:
    - capitalised Spanish "SUMINISTRO DE LOS 2 VEHÍCULOS" → [2], and "Lote 3 - Los 2 contenedores" → [3, 2];
    - "Pakiet 1, zad. 2" and "Pakiet nr 3, zadanie 2" → two lots, because a compound forms only with at most one
      character before its second half;
    - "Pozycja 15, zadanie 2" → [15, 2];
    - "Lot 1–50 places" → 1..50.

**SELFTEST: none of the 35 unit-7 cases changes (old → new: none).** 42 cases are new:
- 31 from round 1.
- 2 from round 2: "Lots 1 - 2 - 3" → [1], and the no-break-space case.
- 9 from the build review:
  - each one-sided half of (a) alone, with no quantity word to hide it: "Lot 1- 5 ordinateurs" → [1], "Lot 1 -5
    ordinateurs" → [1], "Lots 3- 4" → [3], "Lots 3 -4" → [3], and two cached amounts, "Pakiet nr 1- 12 000,00 PLN."
    → [1] and "Pakiet nr 2 -100,51;" → [2];
  - the count of (c) only after a pakiet keyword: "Zadanie 8 pozycji" → [8];
  - the two input preconditions above: "Lot ３" → [], and NFD "Część 2" → []. The critic's 18 probes, with their output after the fix:

| probe | unit 7 | amended | how |
|---|---|---|---|
| Część 1 – 50 miejsc | 1..50 | [1] | fixed (a) |
| Lot 1 - 5 ordinateurs portables | 1..5 | [1] | fixed (a) |
| Lot 12 – 15 sites | 12..15 | [12] | fixed (a) |
| Lot 3 - 2 véhicules | [3, 2] | [3] | fixed (a) |
| Pakiet 8 poz. 6 i 7 | [8, 6, 7] | [(8, 6, 7)] | fixed (c), head 8 |
| Pakiet 8 poz. 6-7 | [8, 6, 7] | [(8, 6, 7)] | fixed (c) |
| Pakiet 3 (poz. 1-12) | [3, 1..12] | [3] | fixed (c) |
| Elementy … (pakiet 8 pozycji) … - zadanie 4. | [8, 4] | [4] | fixed (c), a count |
| Pakiet 16 zad.1 | [16, 1] | [(16, 1)] | fixed (c), head 16 |
| Suministro de los 2 vehículos | [2] | [] | fixed (b) |
| Los 3: Reinigung | [3] | [3] | kept (b) |
| Lotes 1 a 5 | [1, 5] | [1, 5] | **accepted, guards' structure** |
| Lot 2, 3 ans | [2, 3] | [2] | fixed (d) |
| Pakiet 5, 2 szt. | [5, 2] | [5] | fixed (d) |
| Szkolenie dla grupy 20 osób | [20] | [] | fixed (d) |
| Lot 2 (remplace le lot 5 infructueux) | [2, 5] | [2, 5] | **accepted, guards' structure + N filter** |
| LOT C | [100] | [100] | **accepted, N filter** |
| Pakiet 8 poz. 6, Pakiet 8 poz. 7 | [(8,6), (8,7)] | same | correct; C makes G3 count one head |

Why the three accepted readings are harmless:
- **"Lotes 1 a 5"**: `a` stays a conjunction, because it is the Czech/Slovak "and" ("Část 1 a 2"). Read either way,
  the title names two or more lots, so a block carrying it is a multi-lot award. G3 (both heads in N) or G4 (p not
  a head) refusing it is a right refusal. The reading can cost recall (p ∈ {1, 5} with the other not in N), never a
  false refusal.
- **"Lot 2 (remplace le lot 5 infructueux)"**:
  - The N filter keeps 5 only when the contract notice's own lots print 5. Then the title names two of the Tender's
    own lots.
  - G3 refuses that one block, and nothing spreads: p = 2 is a head, so the block never contradicts and cannot fire
    the notice level.
  - The refusal leaves the award on the Tender, the pre-568 reading. For an award that says it replaces lot 5's
    scope, that is the conservative reading.
  - A parenthesis rule was measured and rejected: it removes real designators ("… (Los 1) und … (Los 2)" in the OOS
    descriptions).
- **"LOT C"**: the N filter keeps 100 only on a Tender with a lot printing 100. Lettered lots normalise to `c`, not
  100. One designator can never fire G3.

### Text fields (clarified)

- **Lot side:** the matched lot's `title` / `description` facts as the fold carries them at the award version. That
  means every `TEXTS` field (`TED-LOT_TITLE`, r209 `TED-TITLE`, `TED-LOT_DESCRIPTION`, `TED-SHORT_DESCR`,
  `TED-SHORT_CONTRACT_DESCRIPTION`, `TED-SHORT_DESCRIPTION_CONTRACT`, a lot-scoped `TED-TITLE_CONTRACT` /
  `TED-CONTRACT_TITLE` / `TED-NEW_VALUE.TEXT` …):
  - scoped to its nearest enclosing lot section;
  - superseded per field across the chain;
  - decoded of publisher escapes (`served_text`).
- **Tender side:** the version's Tender-scope `title` facts, including the OJ-heading fallback.
- **Block side:** the block's own `TED-CONTRACT_TITLE` / `TED-TITLE_CONTRACT` values on the `RES-n` section, decoded
  as served, each with its language.
- **`TED-TITLE` (R2.0.9 `AWARD_CONTRACT/TITLE`) is out of scope for R2.0.8 awards.** In the cache it appears only on
  CANs with their own lot sections, which the guards never read. Reading it is harmless and untested.
- **The port now reads the same table and decode** (`FOLD_TEXTS`, `notice_text_facts`, `decode_publisher_escapes` in
  `g3_port.py`).
- **Measured effect:**
  - 4 matched blocks' lot texts differ from the unit-7 port's: set dedup, and one `&Amp;` decode on 5417602.
  - 0 block titles change by decoding.
  - 0 Tender titles differ.
  - 0 outcomes change: row U8b = FINAL_AMENDED on all 1,837 in-sample and 419 OOS blocks.

### Amended numbers

The multi-lot and other-wrong columns count caught blocks. False refusals (FR) are right blocks refused, with
Clopper–Pearson 95 % intervals. The last column counts Tenders carrying a wrong block before the guards → after them.
FINAL_AMENDED includes R1 + R2. R0 is the round-1 rule.

| set | rule | multi caught | other wrong caught | FR (95 %) | Tenders with a wrong block |
|---|---|---|---|---|---|
| s150 | FINAL | 4/6 | 53/61 | 4/904 (0.12–1.13 %) | 11/150 → 5/150 |
| s150 | **FINAL_AMENDED** (= R0) | 4/6 | 53/61 | **0/904 (0–0.41 %)** | 11/150 → 5/150 |
| f30 | all three | 7/7 | – | 0/202 (0–1.81 %) | 1/30 → 0/30 |
| m77 | all three | – | – | 0/656 (0–0.56 %) | 0/77 → 0/77 |
| OOS 50 | FINAL | – | 100/100 | 22/319 (4.37–10.26 %) | 2/50 → 0/50 |
| OOS 50 | R0 (round 1) | – | 99/100 | 0/319 (0–1.15 %) | 2/50 → 1/50 (5416534) |
| OOS 50 | **FINAL_AMENDED** | – | **100/100** | **0/319 (0–1.15 %)** | 2/50 → **0/50** |
| in-sample | FINAL → AMENDED | 11/13 → 11/13 | 53/61 → 53/61 | 4/1,762 → **0/1,762 (0–0.21 %)** | uniform 12/180 → 5/180 → 5/180 |
| pooled | **FINAL_AMENDED** | 11/13 | **153/161** | **0/2,081 (0–0.18 %)** | uniform 14/230 → **5/230 (2.2 %, 0.71–5.00 %)** |

- **Attribution:**
  - In-sample, R0, R0 + R1 alone, R0 + R2 alone and FINAL_AMENDED give the same outcome on every one of the 1,837
    blocks. Only the guard that fires moves (below).
  - OOS, R2 alone catches 5416534 RES-16, and R1 alone changes no outcome.
- **What fires (FINAL_AMENDED):**
  - in-sample: G4 36, G4-notice 17, G3 10, G5 9, G6 1. R0 had G4 50 and G4-notice 3.
  - OOS: G4 87, G4-notice 13. R0 had G4 99. The 32 G3 refusals FINAL made OOS were all on counter-notice blocks G4
    already refuses, or were the "(pakiet N pozycji)" false positive.
- **The notice level still earns its keep, and now carries more.**
  - It catches 5267810 RES-7 "Pakiet 1A" and RES-8 "Pakiet", and fires on 5020611 RES-1 beside G3.
  - Under R1 it also holds 14 wrong blocks of 5267810 and 12 of 5416534, and under R2 5416534 RES-16 (see "R1 and
    R2").
  - Every G4 notice measured has 2 to 78 contradicting blocks: 8364864 2, 5020611 3, 8286472 9, 5051007 16, 5267810
    15 and 5416534 78. Under R0, 5267810 had 29 and 5416534 90. The threshold of 2 changes no catch.
- **5218711 no longer depends on N's size.** A single contradicting block now refuses only itself. With 2 or more,
  the notice's "Lot k" blocks are title-confirmed and kept.
- **Expected effect on the 156,123 corrected:**
  - touched: still 7 of 150, about 7.3 k (3.0–14.6 k);
  - fully reverted: 3 of 150 (5001424, 5020611, 8364864), about 3.1 k (0.6–8.9 k);
  - blocks back to `lot: null`: about 59 k (was 63 k);
  - right blocks refused by coincidence: 0 measured (was about 4 k in-sample, and about 20 k once the OOS was
    pooled).

### Changed outcomes, FINAL → FINAL_AMENDED (every block)

| block | label | FINAL → AMENDED | why |
|---|---|---|---|
| 5051007 / 17689000 RES-1, 2, 3 ("Pakiet nr 1/2/3") | right | null → LOT-1, 2, 3 | G4 notice (16 contradicting blocks); title-confirmed: Dn = {(p)}, non-compound |
| 5267810 / 18199880 RES-1 ("PAKIET 1") | right | null → LOT-1 | G4 notice (15 contradicting); title-confirmed |
| OOS 8286472 / 16994904 RES-1..8 ("… zadanie k") | right | null → LOT-1..8 | G4 notice (9 contradicting); title-confirmed. RES-4/5/8 also lose G3, because "(pakiet 8/6/3 pozycji)" is now a count |
| OOS 5416534 / 18634900 RES-1..14 ("Pakiet k") | right | null → LOT-1..14 | G4 notice (78 contradicting); title-confirmed |

No other block of the 2,256 moves, in-sample or OOS. 26 right blocks are restored, and no wrong block is newly kept.

**OOS 5416534 RES-16 ("Pakiet 16 zad.2") stays `null`.** FINAL refused it by G3 ([16, 2]) and the notice level.
FINAL_AMENDED refuses it by the notice level alone: its compound (16, 2) does not title-confirm (R2). Its lot is LOT-17;
LOT-16 is "Pakiet 16 - pozycja nr 1". Under R0 it moved null → LOT-16, wrong.

**Against the round-1 rule (R0 → FINAL_AMENDED):**

| block | R0 → AMENDED | why |
|---|---|---|
| OOS 5416534 / 18634900 RES-16 (wrong) | LOT-16 → null (G4-notice) | R2: a compound does not title-confirm |
| 5267810 / 18199880 RES-9..12, 15..18, 25..30 ("Pakiet 8 poz. k", 14, wrong) | null → null; G4 → G4-notice | R1: head 8 is shared with the matched lot's "pakiet 8". The CN lists pakiet 8's items as lots 8..30, and each block names another item of pakiet 8 |
| OOS 5416534 / 18634900 RES-17..20, 95..102 ("Pakiet 16 zad.3..6", "Pakiet 88 zad.6..13", 12, wrong) | null → null; G4 → G4-notice | R1, the same shape |

### Residual, re-stated: 568-attributable vs inherited

After FINAL_AMENDED, 5 of 230 uniform Tenders carry a wrong block (2.2 %, 0.71–5.00 %):
- 5 of 180 in-sample, the same five as FINAL;
- 0 of 50 OOS.

| case | blocks | attributable to | why not guarded |
|---|---|---|---|
| 4854639 RES-9/10 "Painting" on LOT-5 "5. Snagging" (its lot is "6. Repair painting"): CAN-only renumbering, title paraphrased | 2 | **568** | **Corrected text:** plain containment ("painting" ⊂ "repair painting") refuses 11 right blocks: 4854639 RES-1..6 "Cranes / Lifts / High Voltage", 8367203 RES-6, 5625827 RES-1, 8281034 RES-1, 4975428 RES-4, 8313642 RES-1. Each of those shares a content token with its own lot's title. With that condition (the block's content tokens are a subset of exactly one other numbered lot's title and share no token with the matched lot's title; critic's G5b, `scratchpad/u8/critic/g5sub.py`), it catches RES-7..10 with 0 false refusals on 1,762 + 319. **Not built:** it is fit on this one Tender and has no OOS hit. Adopt it only if a fresh draw shows a hit with 0 false refusals. |
| 4968281 RES-1 "2 Solo-Niederflubusse ca. 15 m", LN 1 = CONTRACT_NO 1 = ITEM 1; its lot is 2 | 1 | **568** | A counter guard (`LN = CONTRACT_NO`, 1..n < \|N\|) refuses 16 right blocks (0.9 %). |
| 4861483 CAN 17286230, another procedure's CAN (file 40132230 vs 40132200) | 4 | inherited: issue 364's weld | It was on the wrong Tender before 568. |
| 5364153 RES-4 "3a Leki …" on LOT-3 | 1 | inherited: the parent lot | Its title is LOT-3's title once "3a" is stripped. Lot 3a exists only in corrigendum prose. |
| 5267907 RES-1/2: the CN's two lots are identical "Produits frais et surgelés / 47 lots" placeholders | 2 | inherited: identical placeholders | No text or value tells them apart. Only winners move; no value moves. |

**568-attributable: 2 of 230 Tenders (0.87 %, 0.11–3.1 %), 3 blocks: 4854639 and 4968281.** The other three Tenders
(1.3 %) are wrong for reasons 568 inherits. 5416534 RES-16, which R0 added, is caught.

### R1 and R2 (adopted in round 2)

| variant | in-sample | OOS | 5416534 counterfactual (CAN re-keyed to the CN's own numbering, 105 blocks) |
|---|---|---|---|
| FINAL (decided) | 11/13, 53/61, FR 4 | 100/100, FR 22 | 22 refused (G3) |
| R0 (round 1) | 11/13, 53/61, FR 0 | 99/100, FR 0 | 90 refused (G4 21, notice 69) |
| R0 + R1 alone | same | 99/100, FR 0 | 0 refused |
| R0 + R2 alone | same | 100/100, FR 0 | 91 refused |
| **FINAL_AMENDED = R1 + R2** | 11/13, 53/61, **FR 0** | **100/100, FR 0** | **0 refused** |

- **R1** follows C's own convention, that a compound counts by its head.
  - It keeps the verifier's synthetic case: a lot titled "Pakiet 8 poz. 6 i 7" with a right block "Pakiet 8 poz. 6"
    printed on it. R0 refused that block (G4).
  - It keeps the counterfactual's 90 blocks.
- **R2** stops a position inside a pakiet from confirming the pakiet's lot. That position is exactly 5416534 RES-16.
- **The cost of R1, measured: more wrong blocks rest on the notice level alone.**
  - A block naming another item of the same pakiet now agrees with its lot at block level. Examples are 5267810's
    "Pakiet 8 poz. 1" printed 9 on LOT-9, which is the CN's pakiet 8 item 2, and 5416534's "Pakiet 16 zad.3" on
    LOT-17.
  - Wrong blocks refused only by the notice level: in-sample 2 → 16, OOS 0 → 13 (`scratchpad/u8a/r2_exposure.json`).
    All of them sit in two notices with 15 and 78 other contradicting blocks, so no outcome moves.
  - A notice made only of such same-pakiet offsets, with fewer than 2 other contradicting blocks, would keep them. No
    such notice is measured.
  - A lot-side reader that kept "Pakiet 16 - pozycja nr 2" as (16, 2) would let R1 compare positions. It is not
    measured, and not part of this rule.

### The read-back bar (replaces Units 11 (e))

- **(i) Parity.** On every block of the fresh 30, the served lot equals the amended port's replay lot: 0
  divergences, deterministic.
  - **The parity replay must include 570's remap** (`../570-lot-keys/remap_port.py` option B, composed into
    `g3_port.replay`'s carry) before the read-back. Without it, every block that 570's narrower G2 newly matches
    diverges by construction.
- **(ii) At most 2 of 30 Tenders carry a judged-wrong block that the port also keeps, of any shape.**
- **(iii) A wrong block of a new shape is recorded and filed, not a reopen.** Reopen only on a parity break, or on
  3 or more of 30.
- **The draw.**
  - Salt `sha256("568u8-fresh30-"+id)`, ascending.
  - From the **combined wet run's changed set** (its cursor range).
  - Excluding the s150, the f30, the m77, 8327340 and the OOS 50 (`u8-oos/draw.json`).
  - At least 10 outside 2012–2015.
  - At least 8 from changed Tenders outside `readback/population.json` that serve an R2.0.8 block on a lot. These are
    the blocks newly matched because 570 narrows G2, a stratum no sample has measured.
  - Read after the drain.
- **P(pass) at the FINAL_AMENDED residual.** (i) is deterministic given a correct build. For (ii):

  | residual rate | P(pass) |
  |---|---|
  | in-sample 5/180 (2.8 %) | 0.950 |
  | its 95 % upper bound, 6.4 % | 0.702 |
  | **pooled 5/230 (2.2 %)** | **0.973** |
  | its 95 % upper bound, 5.0 % | 0.812 |

  - The 8-Tender G2 stratum and the out-of-era 10 are unmeasured. A residual rate there unlike the sample's is what
    (ii) would detect. So would a same-pakiet offset notice without other contradictions (the R1 cost above).
  - "30 of 30" would pass with P = 0.52 at the pooled rate.

### Build deltas (Units 8, amended)

- **Reader.**
  - `lot_designators` and the SELFTEST port take the 77 cases.
  - `title_key` takes the 10 `TITLE_KEY_SELFTEST` cases, the bare-designator ones included.
  - Representation: `Designator { head, positions: Vec }`.
- **Evidence by language.**
  - `AwardText.named` becomes per language: `BTreeMap<lang, Vec<Designator>>`, with `""` for a value with no
    language.
  - `unit7_refusals` computes Dn per language and G3 on heads, as above.
  - It computes `contradicts` by heads (R1): p is a head, or a head is shared with heads(Dlot).
  - It computes `title_confirmed` as non-compound with heads = {p} (R2).
  - It counts the contradicting matched blocks before refusing. A contradicting block is refused. The rest of the
    notice is refused only on a count of 2 or more, sparing title-confirmed blocks.
- **Tests that change.**
  - `a_notice_numbered_by_a_contract_counter_matches_none_5051007`: RES-1..3 keep LOT-1..3, and the other 16 give
    `None`. Rename it.
  - The 5267810 half of the 4945774 test: RES-1 keeps LOT-1, the other 31 give `None`. RES-7 "Pakiet 1A" and RES-8
    "Pakiet" give `None` through the notice level, and so (R1) do RES-9..12, 15..18 and 25..30, "Pakiet 8 poz. k".
  - Any 5416534 case taken from the round-1 port: RES-16 is now `None` (it was LOT-16). RES-1..14 keep LOT-1..14.
    Regenerate those expectations from `g3_port.py`. `apply_amended()`'s defaults are FINAL_AMENDED, so a generator
    that calls it picks up R1 + R2.
  - **`a_contradicting_block_g3_also_refuses_still_refuses_the_notice`** (unit 8 review), under the amendment:
    - Its chain (CN 1–3; RES-1 printed 1 "Lots 2 and 3", RES-2 printed 2 untitled) now gives `[None, LOT-2]`: one
      contradiction refuses only itself.
    - **Replacement chain, which re-pins the order with two contradictions.** CN 1–4; RES-1 printed 1 "Lots 2 and 3"
      (G3, and it contradicts); RES-2 printed 2 "Lot 4" (contradicts); RES-3 printed 3 untitled. That gives
      `[None, None, None]` (RES-3 by the notice level).
    - The mutant that counts contradictions after G3/G6 gives `[None, None, LOT-3]`.
    - Checked through the port, under R0 and FINAL_AMENDED alike (`scratchpad/u8a/r2_synthetic.py`).
- **Tests that are new.** Each was checked through the port (`r2_synthetic.py`).
  - A single contradicting block beside agreeing ones refuses only itself. CN 1–3; "Lot 1" printed 1, "Lot 3"
    printed 2, untitled printed 3 → `[LOT-1, None, LOT-3]`.
  - **A fired notice spares only a title-confirmed, non-compound block (R2).**
    - CN 1–5: "Lot 3" printed 1 and "Lot 4" printed 2 contradict.
    - "Lot 3" printed 3 keeps LOT-3.
    - Untitled printed 4 → `None`.
    - "Pakiet 5 zad.2" printed 5 → `None` (R0 kept LOT-5).
  - **The 5416534 RES-16 shape (R2).**
    - CN printed 14 "Pakiet 14", 15 "Pakiet 15", 16 "Pakiet 16 - pozycja nr 1", 17 "Pakiet 16 - pozycja nr 2", 18
      "Pakiet 17", 19 "Pakiet 18".
    - The CAN: "Pakiet 14" printed 14, "Pakiet 16 zad.2" printed 16, "Pakiet 18" printed 17, "Pakiet 19" printed
      18.
    - Result: `[LOT-14, None, None, None]`. R0 gave LOT-16 for the second block.
  - **A compound agrees with its lot by head (R1).**
    - CN printed 16 "Pakiet 16 - pozycja nr 1", 17 "Pakiet 16 - pozycja nr 2", 18 "Pakiet 17".
    - The CAN: "Pakiet 16 zad.2" printed 17, "Pakiet 16 zad.1" printed 16, "Pakiet 17" printed 18.
    - Result: `[LOT-17, LOT-16, LOT-18]`. R0 refused the first by G4.
    - Also: a lot printed 3 titled "Pakiet 8 poz. 6 i 7", beside a lot printing 8, with the block "Pakiet 8 poz. 6"
      printed 3 → LOT-3.
  - "Pakiet 8 poz. 6, Pakiet 8 poz. 7" printed 8 keeps (G3 by heads).
  - A translated block. EN "Lot 2", DE "Los 3" (a typo), printed 2, keeps: EN agrees. "Lots 2 and 3" in one
    language is G3, while "Lot 2" in EN beside "Los 3" in DE is not.
- **Port parity.** `python3 -I u7/g3_port.py` gives:
  - FINAL: 11/13, 53/61, 4 FR (unchanged);
  - FINAL_AMENDED: 11/13, 53/61, 0 FR, with the 77's 656 blocks unchanged;
  - `u8-oos/amended_oos.py` FINAL_AMENDED: 100/100, 0/319, with the counterfactual refusing 0 of 105.
- **Read-back (c)** becomes the s150's **57** refused blocks. This is unchanged by R1 and R2:
  - 5051007 RES-4..19 (16), 5267810 RES-2..32 (31);
  - 8364864 × 2, 5020611 × 4, 5001424 × 1;
  - 4854639 RES-7/8, 4975428 RES-2.

  5051007 RES-1..3 and 5267810 RES-1 keep LOT-1..3 / LOT-1.
- **/docs.** Add, after the caveat sentence of Units 8: "An r2.0.8 award's lot is inferred from the number it prints
  against the contract notice's lots; a small recorded residual (about 2 % of such Tenders) keeps a block on another
  lot." Pin both sentences with `assert_says`.

### Risks, amended

1. **Keyword false positives.** The false-refusal rate is in-sample, where the reader was tuned, plus the OOS 50.
   - In-sample it is 0/1,762, at most 0.21 %. OOS it is 0/319, at most 1.15 %.
   - Before the amendment the OOS surfaced two false-positive shapes in two Tenders, "(pakiet N pozycji)" and "zad."
     as a second lot. Both are fixed.
   - A single false positive now refuses at most its own block. The notice level needs two contradicting blocks, and
     spares title-confirmed ones.
2. **Language gaps.** Unchanged. The per-language rule is untested on real translated titles: 13 matched blocks
   carry 2 or more languages, and no two languages of one block name different Dn.
3. **Coincidental rights.** These are now kept when title-confirmed: 26 restored across in-sample and OOS. R2
   closes the shape R0 opened, a compound whose head coincides with a counter.
4. **Same-pakiet offsets rest on the notice level (R1).** 26 wrong blocks lost their block-level G4 and are held only
   because their notices have 15 and 78 other contradictions. Read-back (ii) is what would see a notice without them.
5. **The residual.** 5/230 (2.2 %), of which 2/230 (0.87 %) is 568-attributable, in the shapes above. 4861483 is
   364's. 5267907 moves winners only. 5364153 sits on its parent lot.

**Status line (amended):**

> Status: ready-for-agent — UNIT 8 AMENDED 2026-10-18 (critic wf_2f6dde5b-372 adopted; round 2 adopted R1 + R2 and the
> quantity-word rule): G4 block-level, notice-level only on ≥ 2 contradicting blocks; agreement by head (R1);
> title-confirmed = non-compound with heads {p} (R2); G3 on heads; titles per language; reader fixes (tight dash, `Los`
> case, compounds incl. `zad.`, counts, quantity words); G5 title key pinned. In-sample 11/13 multi, 53/61 other, FR
> 0/1,762; OOS 100/100, FR 0/319; pooled residual 5/230 Tenders (2.2 %, 0.71–5.0 %), 2/230 568-attributable.
> NEXT: unit 8 build to the amended port → gate → deploy → ONE refold with 570 → read back on the new bar (parity
> with 570's remap in the replay, 0 divergences; ≤ 2/30 wrong-kept; new shapes filed).
