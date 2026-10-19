# B6 U4: guard-cost tally (issue 551)

Tallied 2026-10-19. The verdicts come from the U4 reads (`read_1.md` to `read_5.md`), the adversarial skeptics on every
non-right (`skeptic_2.md`, `skeptic_4.md`, `skeptic_5.md`) and two recorded priors. This step does no fetch and no read.
The machine-readable copy, with every id, the read-order check and the P id list, is `u4-result.json`.

## Rule applied (decision point 3, plus the U2 adoption-rule gap)

- Read set: min(|R|, 30) in `sha256('551-b6-gcost-'+id)` ascending order. An id read earlier keeps its recorded verdict.
- A guard is adopted iff **at least 10 % of its read set is non-right** (wrong or neutral).
- G-single is judged on the first 30 of R_single (|R_single| = 34). G-cover is judged on R_excess = {7202436}.
- If G-single is REJECTED and R_excess would adopt G-cover, G-cover is judged on all 13 of its refusals instead.
- The read order recomputed from `b6/u2/R_single.json` matches `read_set.json` exactly. Ranks 31 to 34
  (7354861, 6838776, 5663701, 7600895) are not read for G-single.

## Result

| guard | read set | right | neutral | wrong | non-right | share | bar | decision |
|---|---|---|---|---|---|---|---|---|
| **G-single** | R_single, first 30 | 27 | 0 | 3 | 3 | 10.0 % | ≥ 3 of 30 | **ADOPT** |
| **G-cover** | R_excess (1) | 0 | 0 | 1 | 1 | 100.0 % | ≥ 1 of 1 | **ADOPT** |
| G-cover, gap case (record only, not triggered) | all 13 refusals | 9 | 0 | 4 | 4 | 30.8 % | ≥ 2 of 13 | (would adopt) |

- **G-single: ADOPTED.** 3 of 30 are non-right: 6582856, 6465510 and 7179291, all wrong / other (the 6465510 shape).
  That is exactly the bar.
- **G-cover: ADOPTED** on R_excess: 7202436 is wrong / other. The gap rule is not triggered because G-single is adopted.
- **P = v2+both, 3,467 ships** (`b6/u2/P.json` guard set `v2+both`, ids_sha256 `a2619fd9e380d3d5672b3417d807e504b3a85965b8e5ea319425564e3cd21872`).
  The other candidate sizes were v2 3,502, +single 3,468 and +cover 3,489.
- Flip switches: `R10_G_SINGLE = true` and `R10_G_COVER = true`.
- **Removed from v2 (35)**, which is R_single's 34 plus 7202436:
  402503, 472292, 5581720, 5663701, 5884304, 6003114, 6234738, 6342738, 6356382, 6403567, 6465510, 6582856, 6595330, 6784258, 6815094, 6838776, 6852228, 6937514, 7014656, 7102361, 7163120, 7166738, 7179291, 7202436, 7348648, 7354861, 7380025, 7394272, 7423850, 7600895, 7605188, 7608791, 7651470, 7675062, 7785319.
- 7600895 is the one G-cover overlap id outside the 30. It reads **right** and is recorded separately. It is not counted
  for G-single and is used only in the gap-case row.

## Sensitivity (not operative)

G-single sits exactly on its bar. Turning any right into a non-right only strengthens the adoption. The adoption is lost
only if one of the three non-rights is overturned. Each was held by an adversarial reader-and-skeptic pair:

- 6465510: B5 investigation, recorded.
- 6582856: `skeptic_2.md`, upheld.
- 7179291: `skeptic_4.md`, upheld.

If one were overturned, G-single would be rejected at 2 of 30. The gap rule would then judge G-cover on its 13 refusals:
3 of 13 = 23.1 % non-right, still adopted. P would then be v2+cover, 3,489 ships.
Either way, G-cover is adopted.

## Per-id verdicts

| set | rank | id | S (port v2) | n | v2 vouched_by (removed by the guard) | in G-cover | verdict / class | source |
|---|---|---|---|---|---|---|---|---|
| R_single | 1 | 7380025 | EUR 2,784,373.10 | 3 | TED-VAL_TOTAL 22369006 | yes | **right** / ok | u4 read_1 |
| R_single | 2 | 472292 | HUF 513,615,371.00 | 4 | BT-161-NoticeResult 23758321 | no | **right** / ok | u4 read_1 |
| R_single | 3 | 7785319 | PLN 41,500.00 | 2 | TED-VAL_TOTAL 23564346 | no | **right** / ok | u4 read_1 |
| R_single | 4 | 7014656 | EUR 163,160.00 | 2 | TED-VAL_TOTAL 21635083 | no | **right** / ok | u4 read_1 |
| R_single | 5 | 7675062 | EUR 182,106.53 | 3 | TED-VAL_TOTAL 23312560 | no | **right** / ok | u4 read_1 |
| R_single | 6 | 6234738 | EUR 68,497.20 | 2 | TED-VAL_TOTAL 20033615 | no | **right** / ok | u4 read_1 |
| R_single | 7 | 6582856 | EUR 4,160.40 | 7 | TED-VAL_TOTAL 20932731 | yes | **wrong** / other | u4 read_2 + skeptic_2 (upheld) |
| R_single | 8 | 7348648 | HRK 97,626,068.20 | 17 | TED-VAL_TOTAL 22886555 | no | **right** / ok | u4 read_2 |
| R_single | 9 | 7423850 | EUR 1,176,865.00 | 2 | TED-VAL_TOTAL 22432073 | yes | **right** / ok | u4 read_2 |
| R_single | 10 | 6342738 | EUR 180,736.00 | 2 | TED-VAL_TOTAL 20223375 | no | **right** / ok | u4 read_2 |
| R_single | 11 | 5581720 | EUR 1,380,000.00 | 2 | TED-VAL_TOTAL 18983294 | no | **right** / ok | u4 read_2 |
| R_single | 12 | 7608791 | EUR 52,750,000.00 | 2 | TED-VAL_TOTAL 23061269 | no | **right** / ok | u4 read_2 |
| R_single | 13 | 6815094 | EUR 677,400.00 | 4 | TED-VAL_TOTAL 21285538 | no | **right** / ok | u4 read_3 |
| R_single | 14 | 6403567 | EUR 280,000.00 | 2 | TED-VAL_TOTAL 20520516 | no | **right** / ok | u4 read_3 |
| R_single | 15 | 6595330 | EUR 2,295,195.00 | 2 | TED-VAL_TOTAL 20747457 | no | **right** / ok | PRIOR b3/read18/read18.json (B3 read18, wf_24a65e6a-e63, reader+skeptic agree) |
| R_single | 16 | 6465510 | EUR 162,020.92 | 2 | TED-VAL_TOTAL 20772908 | yes | **wrong** / other | PRIOR b5/investigate/read/read.json (B5 investigation read, the 61, confirm30, wf_eecbe1b7-bc7, reader+skeptic agree) |
| R_single | 17 | 7651470 | EUR 5,667,313.10 | 2 | TED-VAL_TOTAL 23006164 | yes | **right** / ok | u4 read_3 |
| R_single | 18 | 6937514 | EUR 88,732.00 | 4 | TED-VAL_TOTAL 21530641 | no | **right** / ok | u4 read_3 |
| R_single | 19 | 6356382 | EUR 285,379.52 | 2 | TED-VAL_TOTAL 20317908 | yes | **right** / ok | u4 read_3 |
| R_single | 20 | 7166738 | EUR 898,804.96 | 2 | TED-VAL_TOTAL 22450615 | yes | **right** / ok | u4 read_3 |
| R_single | 21 | 7102361 | EUR 1,000,000.00 | 2 | TED-VAL_TOTAL 22048453 | no | **right** / ok | u4 read_4 |
| R_single | 22 | 6003114 | PLN 38,200.00 | 3 | TED-VAL_TOTAL 19427538 | no | **right** / ok | u4 read_4 |
| R_single | 23 | 7179291 | EUR 83,349.90 | 3 | TED-VAL_TOTAL 22780357 | yes | **wrong** / other | u4 read_4 + skeptic_4 (upheld) |
| R_single | 24 | 402503 | HUF 35,298,888,888.00 | 2 | BT-161-NoticeResult 24416505 | no | **right** / ok | u4 read_4 |
| R_single | 25 | 7394272 | EUR 3,692,965.50 | 5 | TED-VAL_TOTAL 22388431 | yes | **right** / ok | u4 read_4 |
| R_single | 26 | 6852228 | PLN 19,822,610.00 | 6 | TED-VAL_TOTAL 21600409 | no | **right** / ok | u4 read_4 |
| R_single | 27 | 6784258 | PLN 46,569,105.69 | 2 | TED-VAL_TOTAL 21353722 | no | **right** / ok | u4 read_5 |
| R_single | 28 | 7605188 | EUR 3,816,100.00 | 2 | TED-VAL_TOTAL 22923984 | yes | **right** / ok | u4 read_5 |
| R_single | 29 | 7163120 | EUR 4,204,400.00 | 2 | TED-VAL_TOTAL 22136632 | yes | **right** / ok | u4 read_5 |
| R_single | 30 | 5884304 | EUR 120,222.44 | 2 | TED-VAL_TOTAL 19344705 | no | **right** / ok | u4 read_5 |
| R_excess | 1 | 7202436 | EUR 27,222.00 | 2 | TED-VAL_TOTAL 22179136 | yes | **wrong** / other | u4 read_5 + skeptic_5 (upheld) |
| cover_overlap_extra | 34 | 7600895 | EUR 22,046,027.00 | 3 | TED-VAL_TOTAL 22875211 | yes | **right** / ok | u4 read_5 (cover_overlap_extra; not counted for G-single) |

The full reasoning for each id is in `read_1.md` to `read_5.md`, `skeptic_2.md`, `skeptic_4.md` and `skeptic_5.md`. The
two priors are `b3/read18/read18.json` (6595330, right / ok: same S, currency and n as v2, EUR 2,295,195.00, n = 2) and
`b5/investigate/read/read.json` (6465510, wrong / other: same S, currency and n, EUR 162,020.92, n = 2).

## Inputs and API use

- Inputs, sha256:
  - `b6/u2/R_single.json` `c1b413e034dc8bbc57229cc6b89b4eacc7be60f700dc6d93890fefd0afa62c60`
  - `b6/u2/R_excess.json` `3d134390f82269a5c568e3a82269264b5f16648bcfc5cb63372cc8f711b8e252`
  - `b6/u2/decisions.jsonl.gz` `0add92e36c70ca76b81049e9faf678e91305e790d1fd914ff4952a0f1d0749cb`
  - `b6/u2/P.json` `5f8b7f7ce50b07d9a474539d0aa7a162a6a1933e638ec9914218c22727f4f75b`
  - `b6/u4/read_set.json` `d77efbc65b5366cef3f742ec5a14fa8b8b47e7ca5cbe2b8e274efa84771eb756`
- API, from `b6/u4/api_requests.log`:
  - 157 GETs to the public API, every one spaced at least 2 s apart: 35 tenders, 118 notice contents and 4 listings.
  - 156 returned 200. One got a connection reset (rc=0, `/v1/notices/21584710/content`, read_4). There were no 408s.
  - The 156 bodies are under `b6/u4/bodies/`.
  - No `/v1/sql`, ssh, POST, git or cargo was used.
