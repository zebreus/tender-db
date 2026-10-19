# 551 B6 U4 guard-cost read, skeptic 5: the non-right from read 5 (7202436, R_excess)

2026-10-19, skeptic `skeptic_5`. This is the adversarial skeptic on the one non-right verdict in `read_5.md`: 7202436,
the only id in R_excess (the ships G-cover refuses beyond G-single). The brief: try to show that the v2 ship is in fact
RIGHT under M2, meaning the summed awards are the tender's value and the stated total vouches for them. Keep neutral or
wrong only where the documents show that the sum misstates the value.

**Inputs (hashes as `read_set.json` d77efbc6… records, re-checked):** `b6/u2/R_single.json` c1b413e0…,
`b6/u2/R_excess.json` 3d134390…, `b6/u2/decisions.jsonl.gz` 0add92e3…. The tender and notices come from the fresh U2
layer (`scratchpad/b6/tenders`, `scratchpad/b6/notices`) and from the bodies `read_5` saved in `b6/u4/bodies/`. Each of
these is JSON-equal to its U2-layer copy:
- `tenders_7202436.json` (sha256 c16fc048…)
- `notices_21970781_content.json` (the CN, F02)
- `notices_22139899_content.json` (the open procedure's award notice, F03)
- `notices_22179136_content.json` (the negotiated F03 that carries the vouch)

**Prior read:** none. 7202436 has no verdict in b3/read18, the B5 investigation 61, the post-drain 279, M1, M2 or the M2
fallback. In b5 it appears only as a population or order member: `confirm_result.predraw.json` order index 2,709 of
3,429 (not in the confirm30), `p2575_ids.json`, `served_lsa_rows.json`, `post_switchoff_rows.json` and
`corpus_changes.json`. Its order key is `sha256('551-b6-gcost-7202436')` = 173a4e61…. It is rank 1 of 1 in R_excess.

**Rubric (M2, wf_dc85fa7e-c00's, as read_1 and read_5 state it):**
- **right/ok:** S is the procedure's value. That means distinct lots (TED `LOT-k` checked against the printed LOT_NO),
  each lot's latest award, no double count, no ×10ᵏ slip and no VAT mix. S must also be no farther from the stated
  total than the head it replaces.
- **Wrong classes:** `not_additive`, `double_count`, `farther`, `slip` and `other`. `other` covers the case where the
  vouching total belongs to something else, as in 6465510.
- **Neutral:** S is not wrong, but it is not clearly better than the head without W.

**New GETs: 2 of the 4 allowed.** Both are logged in `api_requests.log` as `reader=skeptic_5`. Each returned 200 on its
first attempt, and they ran 2.76 s apart under the shared lock. The fetcher is `scratchpad/u4s5/get.py`, which is
read_5's with the reader tag, the attempt and sha256 logging, and the `skeptic5_` body prefix changed.
- `/v1/tenders/7202436` → `bodies/skeptic5_tenders_7202436.json`. It has sha256 c16fc048…, byte-identical to the U2
  copy (fetched 07:24:05Z) and to read_5's body. The head is version 3 and serves EUR 26,981.20 (`result_value`,
  lot null), with `value_derivation` null. Its three versions are CN 21970781, then 22139899, then 22179136.
- `/v1/tenders?buyer=688&cpv=15100000&limit=100` → `bodies/skeptic5_tenders_buyer-688_cpv-15100000_limit-100.json`.
  This looks for a twin tender that could carry a total equal to S: a separate record of the negotiated procedure
  Nr.6506, or a later notice for Nr.5610-1. The result has 14 tenders, `more` false and no ignored filters. Only
  7202436 carries `ojs:2021-513043`. The other "5610" record, 7266775 "Maisto produktai (Nr.5610-3)", is a different
  procedure (`ojs:2021-660203`, 33 lots, 2022-02-14), so the procedure has no twin.

## Verdict

| id | set | prior | v2 S (n) | v2 vouched_by | reader | skeptic 5 |
|---|---|---|---|---|---|---|
| 7202436 | R_excess (rank 1 of 1; counts for G-cover) | no | EUR 27,222.00 (2) | TED-VAL_TOTAL EUR 26,981.20 on 22179136 | wrong / other | **wrong / other (upheld)** |

## The case for RIGHT, and why it fails

The port's v2 decision ships S = 2,722,200 cents EUR (n = 2): LOT-6 1,585,000 and LOT-18 1,137,200, both from
22139899. It ships on one candidate, TED-VAL_TOTAL 2,698,120 on 22179136. That candidate is F03, kind award,
`round_results` 3, `content_lot_results` 3 and `names_summed_keys` []. {cover} and {single, cover} remove it, and R10
then refuses. {single} keeps it, because the notice has three results.

1. **The summed lots are clean.** This holds.
   - 22139899 RES-1 prints LOT_NO **6**: Kiauliena (pork), UAB "Daivida", contract S-6.19E-732/2021 (2021-12-09),
     7 tenders, VAL_TOTAL 15,850.00.
   - RES-4 prints LOT_NO **18**: Vanduo ir gaivieji gėrimai (water and soft drinks), UAB "Mineraliniai vandenys ir
     alus", contract S-6.19.-709/2021 (2021-12-01), 1 tender, VAL_TOTAL 11,372.00.
   - Neither key carries any other row in the tender, so each figure is that lot's latest award. There is no F20, no
     double count, no slip and no currency or VAT mix: both figures are TED-VAL_TOTAL in EUR.
   - So S passes every part of the rubric except the requirement that S is the procedure's value.
2. **S is the procedure's value.** This fails, under every reading of "the tender".
   - The tender is these three notices. Both F03s cite the CN 2021/S 197-513043 as `THIS_PROCEDURE`.
   - **The open round (Nr.5610-1).** Its award notice 22139899 states PROCEDURE VAL_TOTAL = VALUE (PROCUREMENT_TOTAL)
     = **EUR 48,142.40**. That is exactly its four awards: 15,850.00 (lot 6) + 5,804.00 ("8,9", Riamona) +
     15,116.40 ("12,15,16", Officeday) + 11,372.00 (lot 18). S takes only the two keyed results, 56.5 % of the
     buyer's own total for the very notice S is summed from.
   - **The whole chain.** The open round plus the negotiated award of former lot 4 comes to EUR 75,123.60. S is
     36.2 % of it.
   - **The negotiated round alone (Nr.6506).** Its value is 26,981.20, and S contains none of its lots.
   - The CN 21970781 (F02, 18 lots) states no amount of any kind.
3. **The total vouches S.** This fails.
   - 22179136's PROCEDURE VAL_TOTAL (26,981.20) equals its VALUE (PROCUREMENT_TOTAL) and the VAL_TOTAL of its only
     awarded result, RES-1. RES-1 prints LOT_NO "1 (buv.4 p.d.)", frozen pastry, 8,410 kg: UAB "Mantinga", contract
     S-6.19E-21/2022 (2022-01-12), 2 tenders, titled "Maisto produktai (Nr.6506)".
   - RES-2 ("2 (buv. 5 p.d.)", ice cream) and RES-3 ("3 (buv.7 p.d.)", beef) are PROCUREMENT_UNSUCCESSFUL.
   - D_JUSTIFICATION places the notice under VPĮ 63(1)(2), after the open procedure's bids were rejected as too
     expensive. So the figure is one negotiated contract's own award, for a lot that is not in S, with a different
     supplier, a different procedure number and a later date.
   - It lands −0.885 % from S, inside R10's 1 %, by coincidence. This is the rubric's explicit `other` criterion, the
     same one M2 applied to 6465510 (reader and skeptic agreeing) and skeptic_2 upheld on 6582856.
4. **Could 26,981.20 restate S on another basis?** No.
   - **VAT:** 27,222.00 / 26,981.20 = 1.00892. That is no Lithuanian VAT rate (21 %, 9 % or 5 %) in either direction.
   - **Cumulative total:** the procedure total already published before 22179136 was 48,142.40, and with it the
     procedure reaches 75,123.60. Neither is 26,981.20.
   - **Re-award:** lots 6 and 18 were signed in December 2021 under the open procedure. 22179136 re-awards neither, and
     its only rows held on a lot are the two unsuccessful ones (LOT-2 and LOT-3).
   - **A twin record:** the buyer listing shows that no other record of this procedure exists. No document of the
     procedure states 27,222.00 or any figure within 1 % of it except this coincidence.
5. **Could it be neutral rather than wrong?**
   - S is not `farther`: it is marginally closer than the head without W. Against 48,142.40, S is 56.5 % and the head
     56.0 %. Against 75,123.60, S is 36.2 % and the head 35.9 %. S and the head differ by 0.9 %.
   - S is therefore not clearly better, which is the neutral threshold. Its vouch is not an identity vouch, as
     7749755's was (that neutral rested on a real buyer total of exactly the summed awards). It comes from a notice that
     awards none of the summed lots, which is 6465510's class: wrong / other.
   - Serving it would also label the head as a vouched two-lot sum on a figure that describes frozen pastry.
   - For the adoption count, neutral and wrong are both non-right, so the choice between them does not move any tally.
6. **The guard reads the right fact.**
   - G-cover's accessor sees 22179136 holding rows only on LOT-2 and LOT-3, both unsuccessful and neither summed. Its
     awarded result is keyless, because the printed "1 (buv.4 p.d.)" did not key. So `names_summed_keys` is [] and the
     vouch is correctly removed.
   - G-single misses it because the round has three LotResults. This is the gap that R_excess exists to measure.

**Side note (does not affect the verdict): read_5's lot-key observation, confirmed.**
- The live head's lot titles mix the two rounds:
  - LOT-1 "Šaldyti pyrago gaminiai" (the CN's lot 1 is "Kefyras, jogurtas, varškė").
  - LOT-2 "Ledai" (the CN's lot 2 is "Kiti pieno gaminiai").
  - LOT-3 "Jautiena" (the CN's lot 3 is "Pyrago gaminiai").
- The negotiated notice's renumbered sections have overwritten the open procedure's LOT-1, LOT-2 and LOT-3.
- If a future lot-key change keyed 22179136 RES-1 by its leading "1", it would land on LOT-1 (kefir and yoghurt). S
  would then become EUR 54,203.20, a sum of the right money on the wrong lot. No stated total vouches that within 1 %
  (48,142.40 is −11.2 %), so R10 would refuse at every guard setting.
- A pin beside 6908248's before any such drain is still worth having.

## Effect on the adoption rule

- **G-cover, own read set (R_excess, 1 read):** 7202436 stays non-right, so 1 of 1 is non-right. 100 % ≥ 10 %, so
  G-cover meets its bar.
- **G-cover gap case (13-read set, if U4 rejects G-single):**
  - 7202436 joins the 12 overlap verdicts. With 6465510 (recorded wrong, B5 investigation) it gives at least 2 of 13 =
    15.4 % ≥ 10 %, whatever the other 11 are.
  - skeptic_2 (6582856) and skeptic_4 (7179291) each upheld a wrong among those 11.
- **G-single:** 7202436 is not in its read set, so there is no effect.
