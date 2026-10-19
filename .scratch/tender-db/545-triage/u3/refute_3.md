# 545 unit 3: adversarial pass over review 3 (2026-10-19)

Scope: u2a/verdicts.json rows 200, 202, 204, 207, 208, 210, 212, 213, 215, as judged in `review_3.md`. I tried to refute every CONFIRM (7 rows) and each mention marked `rehome` (7 mentions). Review 3 changed no destinations. The two PARK rows (210 PPI, 215 Watkins) are recorded at the end. Parking is the conservative outcome, so I did not attack them.

## Method
- I re-read the saved bodies, all fetched today between 09:49 and 09:53Z by reviewer 3:
  - the 9 evidence notices;
  - the 9 tenders;
  - every holder and destination org, the JEOL, EEP and Swansea Bay pool rows, and 8878703.
- I re-read u2a's r8 name-range output (`u2a/r8_dest.out.json`) for the `compendium`, `rss international`, `the law society`, `law society` and `swansea bay university health board` ranges.
- **0 new GETs.** The open questions are about legal identity: whether a provisional row is the same company, and who really owns a magnet PPON. `/v1/organizations/{id}` returns only name, country, identifier and mention count, so another org or notice read could not settle any of them.
- No /v1/sql, no POST, no admin, no git, and no edits outside u3.

## Shared-section check (G1), done mechanically
For each evidence notice I counted the holder's PPON in the raw body and listed every `is_ref` reference to its section:

| notice | holder PPON occurrences | references to the section |
|---|---|---|
| 46853619, 46709309, 46855540, 46834978, 46807946, 47293389, 46832653 | 3 each: section id, BT-501, one ref | `TPA-1-0 OPT-300-Tenderer` only |
| 46784746 (Swansea Bay) | 3 | `PROCEDURE OPT-300-Procedure-Buyer` only |
| 46755521 (Watkins) | 8 | 6 x `OPT-300-Tenderer` (TPA-1-6 .. TPA-6-11), the same party |

None of these notices has a `#2` section for the holder's PPON. In each one the holder's own name never appears, and the buyer (and co-buyer NWSSP, and the Tenderer Karl Storz) each have their own section. So no mention binds more than one party, and nothing parks on that rule.

## Per row

| idx | name | review 3 | my result |
|---|---|---|---|
| 200 | Compendium Regeneration Limited | CONFIRM -> 1307252 (provisional) | not refuted (weakest destination, caveat below) |
| 202 | RSS International Ltd | CONFIRM -> 30929573 | not refuted |
| 204 | The Law Society | CONFIRM -> 14894295 (provisional) | not refuted (caveat below) |
| 207 | SWANSEA BAY UNIVERSITY HEALTH BOARD | CONFIRM -> 30916810 | not refuted |
| 208 | Jeol (U.K.) Limited | CONFIRM -> 12991212 | not refuted |
| 210 | Pensions Policy Institute | PARK (keep, medium) | park stands |
| 212 | Equal Education Partners | CONFIRM -> 30928130 | not refuted |
| 213 | Portsmouth Hospitals University NHS Trust | CONFIRM -> 30971975 | not refuted |
| 215 | WATKINS COACHES LTD | PARK (keep) | park stands |

### 200, 202, 204: the magnet PPON PZLB-9446-RVML on 30926847 ("Network Rail", 9 mentions)

**Whose PPON is it?** Two unrelated buyers printed this PPON for three unrelated firms:
- Thirteen Housing Group (GB-MPR-7522) printed it for Compendium;
- National Highways (PGMJ-3793-VBBN) printed it for both RSS International and The Law Society.

It might be Network Rail's real PPON that a buyer's system fills in by default, or a placeholder. That does not matter for these mentions. Each printed name is the party the procedure text describes, so the mention belongs on that party's org whoever owns the PPON:
- "Packaged Land Deal. Brought to THG by Lovells.";
- "... Rail Deck - Main Scheme - RSS Int.";
- "The Law Society - Practising Certificates (SRA)".

**Attack: is the holder an agent, parent, trading name or consortium partner?**
- 202 is the only row where Network Rail could plausibly be involved: highway works over a railway often involve Network Rail asset protection. But the procedure text names "RSS Int." itself (BT-24), and the same buyer reused the PPON for The Law Society, where no Network Rail link is possible.
- So the PPON is a copied identifier and not a relation. Refutation fails.

**Attack, 200 (destination is a dissolved predecessor or a different company):**
- 1307252 is provisional, GB, 1 mention, with a very low id. That suggests an old notice, and the API cannot show which notice it is.
- I cannot rule out that the name was dissolved and then reused. But the decision explicitly accepts "a provisional row whose name_norm matches exactly", and a UK registered name belongs to one company at a time.
- The neighbouring old row 1307773 "Compendium Group" fits a single Compendium (Lovell-linked) group. That is consistent with the 2026 text "Brought to THG by Lovells".
- adv2a found a structural limit: near_identified only sees rows inside the name range. It does not bite here. Every plausible variant name starts with "compendium", and r8's range holds no identified GB Compendium row (the Compendium rows outside GB are Polish, German or have no country).
- Result: not refuted. This is the weakest destination in the set, accepted only because the rule allows it.

**Attack, 202 (RSS International vs RSS Infrastructure):**
- A rail deck invites the guess that the real supplier was the rail contractor RSS Infrastructure Ltd. But BT-500 reads "RSS International Ltd" and BT-24 repeats "RSS Int.", so the notice is internally consistent.
- 30929573 GBCOH06707779 "RSS INTERNATIONAL LTD" is the only identified row in the range and matches after case folding.
- The two provisional rows, 31546684 and 31540612, do not outrank an identified row.
- Result: not refuted.

**Attack, 204 (which Law Society; Law Society or SRA):**
- Practising-certificate fees go to The Law Society of England and Wales. The SRA is its regulatory arm, and the printed name is The Law Society.
- 14894295 is the only row named exactly "The Law Society" (provisional, GB).
- No identified row exists under "the law society%" or "law society%". The Ireland and Scotland rows are different bodies.
- A destination under a different head name, such as an SRA row, would contradict the printed name.
- Result: not refuted.

### 207: Swansea Bay UHB on 30915227 "Storz Medical (UK) Limited" (3 mentions)
- **Attack: PPON ownership.** The holder has only 3 mentions: 2 under the head name and 1 for Swansea Bay.
  - If PZZV-3765-WXZQ were actually a second Swansea Bay registration, the two Storz Medical mentions would be the misbinds.
  - Either way, *this* mention, the Swansea Bay buyer section, belongs on a Swansea Bay row.
  - Swansea Bay's own PPON is PQDY-1678-DHXN (30926961), so the board does not need PZZV.
- **Attack: role (rule 4).** The role is Procedure-Buyer, and both the destination and the printed name are a health board. That is consistent.
- **Attack: G2 (shared procurement).** It does not apply, because the holder is a supplier. NWSSP is a separate co-buyer section, so no A-class confusion on this section.
- **Attack: tiebreak.** The two identified rows are both the board:
  - 30916810 GBNHS7A3 (59 mentions);
  - 30926961, its own PPON (15 mentions).

  Neither choice can land on a different entity.
- After the move, tender 8633322 shows SBUHB plus NWSSP as buyers and Karl Storz as Tenderer.
- Result: not refuted.

### 208: Jeol on UKRI (the issue's exhibit)
- **Attack: does the tiebreak cross distinct entities?** The pool is:
  - 939456, 00939456 and GBCOH939456: one company number;
  - 226825653, which I checked with the HMRC mod-97 rule: digits 2,2,6,8,2,5,6 weighted 8..2 sum to 141, and 141 + 53 = 194 = 2 x 97. It is a **valid GB VAT number**, not a second company number, so distinct_entities correctly does not fire.
- 12991212 also carries JEOL's own PPON PJXB-8739-JZGH, merged in.
- UKRI (3,594 mentions) is not involved in a UKHSA maintenance award.
- Result: not refuted.

### 212: EEP on Torfaen
- **Attack: Torfaen as agent or consortium lead for Blaenau Gwent.** An agent would sit in a buyer section, not on the Tenderer section. Here the only buyer section is Blaenau Gwent's (PDMD-4229-TQWR). Version 1 of tender 8655904 is the UK4, which has no tenderer.
- **Attack: two identified rows, different entities?**
  - 31582974 has company number 07611972 under the name "Equal Education Partners".
  - 30928130 is a PPON row, "Equal Education Partners Limited".
  - Companies House treats names that differ only by legal form as the same, so there can be only one such company. No second company number exists.
  - The 2 vs 1 margin is thin, but both rows are one entity.
- Result: not refuted.

### 213: PHUT on NHS England
- **Attack: is NHS England the principal (commissioning or funding)?**
  - The buyer is the Isle of Wight Council.
  - The text says "The role is hosted by Portsmouth Hospitals University NHS Trust", and Hampshire County Council funds the other 4 days.
  - NHS England plays no part. Its PPON (1,887 mentions) is a copied value.
  - The PHUT/IOW group link is between PHUT and the IOW NHS Trust, not the holder.
- 30971975 is PHUT's own PPON with the exact name, and it is the only identified row.
- Result: not refuted.

### 210 PPI and 215 Watkins: PARK, not attacked
- **210:** the consortium inference (IFS's multi-funder pensions research programme) is a sound reason to keep, and keep is the safe side.
- **215:** the park is in fact *stricter* than G3(c). 8878703's name_norm "watkins coaches" is not exact against "watkins coaches ltd", so the rule as written would still pick 2479435. But u2a's premise ("the only GB row of the two") is false, and "Watkins Coaches" (8 mentions) may be the same operator or a different one. Keep is defensible. If the panel wants it moved, that needs a register read, not this pass.

## Bottom line
- 0 of 7 CONFIRMs refuted, and 0 of 7 rehome mentions refuted. Both PARKs stand.
- The weakest destinations are the two 1-mention provisional exact-name rows, 1307252 (Compendium) and 14894295 (The Law Society). They pass only because the decision accepts exact-name provisionals. If unit 4 wants a stricter bar, these two are the ones to hold back.
