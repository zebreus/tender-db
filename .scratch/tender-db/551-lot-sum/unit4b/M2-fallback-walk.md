# 551 unit 4b, M2 fallback: "vouched everywhere" from the same salted walk (2026-10-17)

**Pre-draw record, written 2026-10-17T16:10Z, before any id past walk.jsonl's 54th was read.**

- **Why:** M2 failed (fresh 30: 27 right, 3 wrong; bar ≤ 1 wrong). The decision fixes one fallback, tested once:
  "vouched everywhere", v2 with R10 at every size.
- **Rule:** `rule_v2.py` sha256 `39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce`, unchanged. A
  **vouched ship** is a v2 ship whose facts carry `vouched_by`: a stated total in S's currency within 1 % of S that
  equals no single summed award and is not a single-lot notice's own award (R10's test, at any size).
- **Order:** the same salt `0x551B4` and order rule as M1; the same population (sha256 `930422d4…2d44`) and the same
  279 skipped. `walk_fallback.py` (sha256 `1338490669402da7865d9cfc725c0c060fa7c47c9c658ec427aebe0b517a9a6b`) imports
  `walk.py` (sha256 `774fe399…95ac`), refuses to run unless walk.jsonl + walk_fallback.jsonl is a prefix of the order,
  and continues from order position 54.
- **Reused:** walk.jsonl's 54 rows as they stand. They hold 5 vouched ships (7399356, 6094289, 6527544, 307725,
  1042997), all read **right** in M2 by two agreeing reads. None of M2's 3 wrongs was vouched.
- **Stop:** 30 vouched ships in total (so 25 new).
- **Read:** only the 25 new ones, under M2's amended protocol (two independent reads per head, a reader and an
  adversarial skeptic, a third reader on disagreement; wf_dc85fa7e-c00's rubric). Any new vouched ship at or above
  EUR 100 m that is not among the judge's 15 band heads is also read under the band protocol.
- **Bar (unchanged):** at most 1 wrong of the 30, and 0 farther or double_count among band heads. M2's band read
  (15 right) stands: every band v2 ship is already vouched, so the band set is the same.
- **Outcome rule:** PASS: B1 to B5 run with R10 at every size. FAIL: unit X (path W stays withdrawn for good; 551
  closes on path V).
- **Data note:** 567's fix (lot keys of legacy TED award blocks) is not deployed; the walk reads today's API. It must
  finish before 567's refold writes, so that every walked id is read under one projection.

## Result

### Walk — COMPLETE (2026-10-17T16:10:44Z → 16:45:15Z, 1,349 GETs)

- 223 new ids walked (order positions 54..276), 277 in all. Outcomes over all 277: 134 v2 ships, 142 refused,
  1 dropped (179732: v1 does not fire, its procedure figure is not a sentinel), 0 unread, 0 port errors.
- **Vouched ships: 30** (5 reused + 25 new): 941214, 64640, 6535408, 202168, 100472, 655266, 672947, 8451328, 7721890,
  881605, 5816379, 775393, 161565, 481715, 590618, 5618652, 7278290, 674079, 1084063, 131063, 7810318, 316811,
  1027821, 5889767, 6311845. Vouching kinds of the 25: BT-118 13, TED VAL_TOTAL 6, BT-1118 3, TED VAL_RANGE LOW 2,
  HIGH 1.
- **Projected fallback fires:** 15,352 × 30/276 ≈ **1,669** (the decision expected about 2,450; 95 % 1,100–4,470).
- **Band:** no vouched ship at or above EUR 100 m (largest 481715, EUR 22.54 m); M2's band read (15 right) stands.
- First refusing check of the 118 new refusals: R3 36, R8 19, R2c 19, R1 11, R2a 8, R2d 6, R10 5, R5.15 3, R7 3,
  R5.1 2, R6 2, R5.13 2, R5.3 1, R5.2 1.
- Items for the read: `M2-fallback-items.json`. Read: workflow wf_03469a70-c1b (M2's script, file and group keys
  changed; items carry `vouched_by`).

### Read

(pending)
