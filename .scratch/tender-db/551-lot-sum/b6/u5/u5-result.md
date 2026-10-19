# 551 B6 U5: fresh pre-flip read and B-band, tallied (2026-10-19)

**Result: GO.** Round a has 30 right, 0 neutral and 0 wrong, so point 7 gives GO and round b is not needed. The B-band
re-read set is empty and has 0 wrong, so point 8 passes.

| step | read set | right | neutral | wrong | rule | result |
|---|---|---|---|---|---|---|
| Point 7, round a | P minus every read id, ranks 1–30 by `sha256('551-b6-fresh-'+id)` | 30 | 0 | 0 | 0 wrong → GO | **GO** |
| Point 7, round b | ranks 31–60 | — | — | — | needed only if round a has exactly 1 wrong | not needed, not read |
| Point 8, B-band | band heads that changed, are in L_changed or are in the delta | — | — | 0 | 0 wrong of any class | **PASS** (set is empty) |

Machine record: `b6/u5/u5-result.json`. It holds the verdicts per id, the counts, both decisions, the input sha256s, the
request-log audit and `checks_failed: []` (sha256 e3e50cc6…).

To reproduce, run `python3 -I b6/u5/tally_u5.py`. The script is offline (sha256 59673a7d…). It reads the verdicts handed
to the tally from `b6/u5/round_a_verdicts.json` (9a9a4f95…) and writes only `u5-result.json`. A re-run gives an identical
file.

## Inputs

- P = v2+both, 3,467 ships (`b6/u2/P.json`, ids_sha256 a2619fd9…; file sha256 5f8b7f7c…). The port decisions per id are
  in `b6/u2/decisions.jsonl.gz` (0add92e3…).
- Order: `b6/u5/order.json`, built by `b6/u5/build_order.py`. It covers P minus the 132 already-read ids
  (`excluded_sources.json`), which leaves 3,335 eligible ids (ordered_ids_sha256 e099007f…).
- Readers `read_a_1` … `read_a_6` covered ranks 1–5, 6–10, 11–15, 16–20, 21–25 and 26–30. Each reader got the
  port-predicted S, currency, n, the summed keys and notices, the G-filtered `vouched_by`, and the live head from
  `GET /v1/tenders/{id}`. Path W is off, so the live head is the max-elected figure, not the sum. The question for each id:
  under M2, is serving S right, neutral or wrong?

## Checks done by the tally (all passed)

- The 30 round-a ids are `order.json` `first_60[:30]` in order, ranks 1–30. Each order key was recomputed and is ascending.
  None of the 30 is an already-read id, and all 30 are in P.
- S, currency, n and `vouched_by` agree for every id across order.json, P.json and decisions.jsonl.gz (v2+both).
- Each verdict and class handed to the tally matches the reader's own result table in `read_a_k.md`, and so do S,
  currency, n and rank.
- Every class is valid for its verdict (right → ok). No id has an estimate-twin flag (`flags: []` on all 30).
- The live head was fetched for all 30. Every non-null head is in S's currency.

## Round a verdicts

"Head % of S" is the live head (path W off) as a share of S. "null" means the tender serves no value today.

| rank | id | S | n | vouched_by (G-filtered) | head % of S | verdict | reader |
|---|---|---|---|---|---|---|---|
| 1 | 77882 | EUR 4,521,000.00 | 9 | BT-118 on 25504477 | null | right / ok | read_a_1 |
| 2 | 6471017 | EUR 1,078,485.00 | 2 | TED-VAL_ESTIMATED_TOTAL on CN 20349868 | 63.9 % | right / ok | read_a_1 |
| 3 | 285956 | EUR 4,477,729.39 | 7 | BT-118 on 25517196 | null | right / ok | read_a_1 |
| 4 | 7021235 | HUF 119,444,475 | 2 | TED-VAL_TOTAL on 21978620 | 51.3 % | right / ok | read_a_1 |
| 5 | 7429588 | EUR 266,078.95 | 2 | TED-VAL_TOTAL on 22628724 | 27.5 % | right / ok (partial sum) | read_a_1 |
| 6 | 7763513 | BGN 200,000.00 | 2 | TED-VAL_ESTIMATED_TOTAL on CN 23214436 | 15.0 % | right / ok | read_a_2 |
| 7 | 5733 | PLN 785,837.16 | 21 | BT-161 on 24561226 | 28.3 % | right / ok | read_a_2 |
| 8 | 6570707 | HUF 459,995,328 | 2 | TED-VAL_ESTIMATED_TOTAL on CN 20595292 | 79.5 % | right / ok | read_a_2 |
| 9 | 6660753 | HUF 1,381,739,164 | 20 | TED-VAL_TOTAL on 21475284 | 4.7 % | right / ok | read_a_2 |
| 10 | 5663715 | EUR 10,300,000.00 | 3 | TED-HIGH on 19055486 | 43.7 % | right / ok | read_a_2 |
| 11 | 6826783 | HUF 476,573,000 | 3 | TED-VAL_TOTAL on 21404947 | 57.5 % | right / ok | read_a_3 |
| 12 | 6611306 | EUR 7,267,960.13 | 7 | TED-VAL_TOTAL on 21302219 | 14.1 % | right / ok | read_a_3 |
| 13 | 8488091 | EUR 164,676.75 | 3 | TED-VAL_ESTIMATED_TOTAL on CN 22059514 | 16.0 % | right / ok | read_a_3 |
| 14 | 118880 | EUR 290,383.00 | 3 | BT-1118 on 24501593 | null | right / ok | read_a_3 |
| 15 | 805752 | HUF 15,060,500,000 | 4 | BT-1118 on 24457168 | null | right / ok | read_a_3 |
| 16 | 6530219 | HUF 464,332,787 | 5 | TED-VAL_TOTAL on 20765679 | 13.9 % | right / ok | read_a_4 |
| 17 | 8441074 | EUR 25,689,400.00 | 3 | TED-VAL_ESTIMATED_TOTAL on CN 20567519 | 19.8 % | right / ok | read_a_4 |
| 18 | 712964 | EUR 1,140,120.00 | 5 | BT-118 on 24751505 | null | right / ok | read_a_4 |
| 19 | 7872311 | PLN 337,651.20 | 2 | TED-HIGH on 23660402 | 87.3 % | right / ok | read_a_4 |
| 20 | 6580960 | EUR 116,530.00 | 3 | TED-VAL_TOTAL on 20835362 | 8.6 % | right / ok | read_a_4 |
| 21 | 64472 | EUR 334,400.49 | 3 | BT-118 on 24822363 (EUR 332,553.00, S +0.56 %) | null | right / ok | read_a_5 |
| 22 | 7442464 | EUR 4,442,428.69 | 7 | TED-VAL_TOTAL on 22708527 | 10.7 % | right / ok | read_a_5 |
| 23 | 5861802 | BGN 342,000.00 | 2 | TED-VAL_ESTIMATED_TOTAL on CN 19145047 | 63.7 % | right / ok | read_a_5 |
| 24 | 7085538 | HUF 1,630,930,456 | 5 | TED-VAL_TOTAL on 22173860 | 21.5 % | right / ok | read_a_5 |
| 25 | 23846 | EUR 1,069,830.00 | 3 | BT-118 on 24991969 | null | right / ok | read_a_5 |
| 26 | 8524598 | EUR 5,849,968.70 | 3 | TED-VAL_ESTIMATED_TOTAL on CN 23230099 | 46.6 % | right / ok | read_a_6 |
| 27 | 6757752 | BGN 2,657,000.00 | 4 | TED-VAL_TOTAL on 21360130 | 58.3 % | right / ok | read_a_6 |
| 28 | 6960026 | PLN 907,500.00 | 2 | TED-HIGH on 21615726 | 84.7 % | right / ok | read_a_6 |
| 29 | 844584 | EUR 2,023,000.00 | 3 | BT-118 on 24887461 | null | right / ok | read_a_6 |
| 30 | 6763031 | EUR 2,160,200.00 | 2 | TED-VAL_TOTAL on 20974563 | 26.7 % | right / ok | read_a_6 |

Vouch kinds across the 30:

- 14 TED-VAL_TOTAL, BT-118 or BT-1118 equal to S to the cent. 6611306 is the exception: the CAN drops 13 cents.
- 8 CN estimate totals, each from a CN whose Lot sections are all in S.
- 3 Range HIGH vouches (LOW = HIGH on 7872311 and 6960026; the stated maximum on 5663715).
- 1 BT-161 that names 20 of the 21 summed keys and already includes the 21st (5733).
- 1 BT-118 that S exceeds by 0.56 % (64472).

Seven heads are null today, so S replaces no figure on them. On the other 23, the head is one lot's figure, between 4.7 %
and 87.3 % of S.

## Shapes the readers flagged (no effect on any verdict)

These are recorded so a later re-read, or the 14-day watch, knows where the weak points are.

- **7429588: partial sum** (the read18 7398935 shape). Lots 1 and 3 equal the F03's VAL_TOTAL. Lot 2's re-award sits on a
  lot-null row and is not summed. S is 78.4 % of the full EUR 339,267.19; the head is 21.6 %. The shape is fragile: a
  single-section placement drain would make R10 refuse it. It is **not** on point 12's partial-sum pin list.
- **Pre-modification values.** Five ids are summed at their original awards, and their F20 value increases are excluded
  under the 7380025 convention. Against the post-modification totals, S stays much nearer than the head:

  | id | S vs post-modification total | head vs post-modification total |
  |---|---|---|
  | 6826783 | −5.4 % | −45.6 % |
  | 6611306 | −5.4 % | −86.7 % |
  | 7085538 | 95 % of it | 20.5 % of it |
  | 6763031 | −10.9 % | −76.2 % |
  | 7442464 | not given | not given |

  For 7442464, S equals the procedure VAL_TOTAL and the VAL_TOTAL_BEFORE in all seven F20s.
- **Range and maximum vouches.** 5663715's S equals the CAN's stated procedure maximum (TED-HIGH). Its LOW, EUR 3.5 m, is
  the sum of the lot minima. 64472 sums the BT-720 winning tenders, and its vouch is the BT-118 sum of the BT-709 maxima.
- **Copied BT-709 not summed.** On 285956 and 844584, every result carries the whole BT-709 total. S sums BT-720 instead,
  so there is no double count.
- **8524598 PIN.** The F04 PIN 23185304 stated EUR 21.56 m and is stored as a separate tender, 7711779. It is a forecast:
  the CN and the CAN agree on EUR 5.85 m.
- **Annual or other bases.** 6530219 is consistently annual premiums. 64472 may be annual values. Neither changes the
  verdict.

## Sensitivity (record only)

The rule was fixed before any read, in decision point 7.

- If one of the 30 rights were later overturned to wrong, round a would have exactly 1 wrong. Point 7 would then require
  ranks 31–60 (`order.json` `first_60[30:]`, starting 6763230, 841643, 6787726 …) to have 0 wrong.
- If two were overturned, the result would be KEEP OFF and re-decide.
- No verdict here sits on a bar the way G-single did in U4.

## B-band (point 8)

The rule: re-read any of the 29 band heads (B_P) whose predicted S, n or `vouched_by` changed since its last read, plus
every band row from L_changed or the delta. The re-reads need 0 wrong of any class.

- **Changed against the last read: none.** For each of the 29 heads, (S, currency, n, `vouched_by`) under v2+both is
  strictly equal to the figures its last recorded read judged (`band_set.json` `rows`). The tally recomputed all 29 and got
  `[]`. The last recorded verdicts are 29 right: 15 from the unit 4b M2 band re-read with no class, and 14 right / ok.
  All 29 are in P.
- **Band rows in L_changed: none.** The provisional L_changed ∩ 3,502 is 0 under the 2630-excluded reading that binds U2
  and U6. The 545, 571 and 541 sets contain no band head and no P id.
- **Delta:** none. The delta (salt `551-b6-delta-`) is the set of new fires after the drain, so it does not exist before
  the flip.
- **Re-read set: empty.** That means 0 re-reads and 0 wrong, so point 8 passes. The pass is vacuous at U5. The band
  obligation comes back after the drain (delta rows) and if U6's re-prediction moves a band head.
- **D4 watch.** Two ids sit within 2 % of EUR 100 m: 5842187 (in B_P) and **899272** (outside B_P, HUF, S ≈ EUR 98.95 m
  at the port's rates). Under point 8 D4, if 899272 differs from B_P at drain time, that is stop-and-explain, not a
  switch-off.

## Request log audit (`b6/u5/api_requests.log`, bodies under `b6/u5/bodies/`)

- **Volume:** 134 requests, all GET and all HTTP 200, from 15:47:02Z to 16:09:40Z on 2026-10-19.
- **Pacing and retries:** the minimum gap between requests was 2.02 s, with 0 gaps under 2 s. There were no 408s, no
  retries and no repeated path.
- **Targets:**
  - 31 tender heads: the 30, plus 7711779 (8524598's PIN tender).
  - One lookup by publication_id (262739-2023).
  - 102 notices.
- **Bodies:** 134 files in `bodies/`.
- **By reader:** read_a_1 15, read_a_2 26, read_a_3 19, read_a_4 21, read_a_5 32, read_a_6 21.
- **Not used:** /v1/sql, ssh, POST, git and cargo.

## Still owed after U5 (from the decision, not new)

- U6 re-predicts P on refetched facts for L_changed ∩ 3,502. That set is 0 today, but it is fixed per id at U6's fetch
  time.
- Re-run the zero-drift probe on the deployed rev, refresh the rates, and add 0787596 to the code-drift ledger.
- If U6 moves a band head's S, n or `vouched_by`, that head joins the band re-read.
- The post-drain delta read uses salt `551-b6-delta-`.
