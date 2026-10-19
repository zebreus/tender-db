# 551 B6 U4 guard-cost read, skeptic 2: the non-right from read 2 (6582856)

2026-10-19, skeptic `skeptic_2`. This is the adversarial skeptic on the one non-right verdict in `read_2.md` (R_single
salted ranks 7–12). The brief: try to show that the v2 ship is in fact RIGHT under M2, meaning the summed awards are
the tender's value and the stated total vouches for them. Keep neutral or wrong only where the documents show that the
sum misstates the value.

**Inputs (hashes as `read_set.json` d77efbc6… records):** `b6/u2/R_single.json` c1b413e0…, `b6/u2/R_excess.json`
3d134390…, `b6/u2/decisions.jsonl.gz` 0add92e3…. Tender and notices come from the fresh U2 layer
(`scratchpad/b6/tenders`, `scratchpad/b6/notices`) and from the bodies `read_2` saved in `b6/u4/bodies/`.

**Prior read:** none. 6582856 does not appear in b3/read18, the B5 investigation 61, the post-drain read, M2, the M2
fallback or M1's walk. Its salted rank is re-derived as **7 of 34** (`sha256('551-b6-gcost-6582856')`), and the first
30 equal `read_set.json`'s list. It is also one of G-cover's 12 R_single-overlap ids.

**New GETs: 2 of the 4 allowed.** Both are logged in `api_requests.log` as `reader=skeptic_2`, and each returned 200 on
its first attempt.
- `/v1/tenders/6582856` → `bodies/skeptic2_tenders_6582856.json`. JSON-equal to the U2 copy: version 7, head EUR 945.00.
- `/v1/notices/20568018/content` (the CN) → `bodies/skeptic2_notices_20568018_content.json`. JSON-equal to the U2 copy.

The five award notices `read_2` saved (20722713, 20738475, 20781489, 20932731, 20989144) are each JSON-equal to their
U2-layer copies.

## Verdict

| id | set | prior | v2 S (n) | v2 vouched_by | reader | skeptic 2 |
|---|---|---|---|---|---|---|
| 6582856 | R_single (also G-cover overlap) | no | EUR 4,160.40 (7) | TED-VAL_TOTAL EUR 4,200.00 on 20932731 | wrong / other | **wrong / other (upheld)** |

## The case for RIGHT, and why it fails

The port's v2 decision ships S = 416,040 cents EUR (n = 7) on one candidate: TED-VAL_TOTAL 420,000 on 20932731. That
candidate is F03, kind award, `round_results` 1 and `names_summed_keys` []. Every guard set removes it, and R10 then
refuses.

1. **The summed lots are clean.** This holds. Each award's lot section prints LOT_NO equal to its key: 1 (20989144,
   945.00), 3 + 7 (20722713, 320.40 + 774.00), 5 + 8 (20738475, 1,280.00 + 115.00) and 4 + 9 (20781489, 388.00 +
   338.00).
   - Every lot was awarded once, with no F20 in the chain, so each figure is the lot's latest award.
   - There is no double count, no ×10ᵏ slip and no VAT mix: every figure is TED-VAL_TOTAL.
   - So S passes every part of the rubric except the requirement that S is the procedure's value.
2. **S is the procedure's value.** This fails.
   - The CN 20568018 (F02, 9 lots) states no amount of any kind: no amount-type field in any section.
   - 20957018 closes lot 2 with no award.
   - That leaves 8 awarded lots, EUR 8,360.40 in total. S leaves out **lot 6, the defibrillator (EUR 4,200.00 on
     20932731)**, which is the largest award and 50.2 % of the money.
   - S covers 49.76 % of the awarded total. A value that misses half the awarded money is not the procedure's value.
3. **The total vouches S.** This fails.
   - 20932731 has one Lot section (section LOT-1, printed LOT_NO **6**, "Ārējais automātiskais defibrilators").
   - Its one LotResult (CONTRACT_NO 6, no LOT_NO printed) has VAL_TOTAL 4,200.00. Its PROCEDURE VAL_TOTAL / VALUE
     (PROCUREMENT_TOTAL) is the same 4,200.00.
   - The figure is lot 6's own award. It lands −0.943 % from S, inside R10's 1 %, by coincidence.
   - This is the rubric's explicit wrong criterion: the vouching total belongs to something else. M2 applied the same
     criterion to 6465510, with reader and skeptic agreeing.
4. **Could 4,200.00 be a cumulative or procedure-wide total?** No. Some rights in R_single restate a running procedure
   total on a one-result notice. The arithmetic rules that out here:
   - By 20932731's dispatch (2020-03-11), the awards already published add to EUR 3,215.40. With lot 6 they make EUR
     7,415.40. With lot 1 (2020-04-07) they make EUR 8,360.40. None of these is 4,200.00.
   - This buyer's other four award notices each state exactly their own lots' sum: 1,094.40, 1,395.00, 726.00 and
     945.00.
   - 4,200.00 cannot be the seven summed lots on another basis either. S is 4,160.40, and RES-1 assigns 4,200.00 to the
     defibrillator lot.
5. **Could it be neutral rather than wrong?** 7749755 was neutral because its vouch was an identity vouch: a real
   buyer-stated total of exactly the summed awards. Here the vouch comes from a notice that awards none of the summed
   lots, which is 6465510's class (wrong / other).
   - S is nominally closer to the truth than the head without W: 49.8 % against 11.3 %, where the head is lot 1's
     945.00. 6465510 was also nominally closer (18.8 % against 8.2 %) and was still wrong.
   - For the adoption count, neutral and wrong are both non-right, so the choice between them does not move any tally.
6. **The guard reads the right fact.** 20932731's lot section id is `LOT-1`, which is a summed key. Its result, though,
   is keyless (stored lot = null, because printed 6 ≠ ordinal 1). G-cover's accessor counts only keyed results, so it
   correctly sees that the notice names no summed key. A section-id reading would wrongly have kept the vouch.

**Side note (does not affect the verdict):** a future placement drain could key 20932731's result as LOT-6 (567's
single-section no-number placement). S would then become EUR 8,360.40, the true total. No stated total vouches that
within 1 % (the largest is 4,200.00), so R10 would refuse at every guard setting. The live winner name (UAB MedUS
Medical Latvijas filiāle, while the notice prints UAB "GRAINA" Latvijas filiāle) is an organisation-identity matter,
not a value matter.

## Effect on the adoption rule

- **G-single (R_single read set of 30):** 6582856 stays non-right. Read 2's slice tally is unchanged: 6 read, 1
  non-right.
- **G-cover gap case (13-read set):** 6582856 keeps wrong / other as one of the 12 overlap verdicts.
