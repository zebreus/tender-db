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

(pending)
