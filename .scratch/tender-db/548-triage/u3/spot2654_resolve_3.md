# Job 2654 (rule 524u2-v13): resolving the contested samples in block 3 (samples 20..29)

Runbook: docs/operations.md, refresh-org-names step 5. Go needs 0 former-name reverts and at most 1 other wrong.
Inputs:
- the judges' write-ups `spot2654_3A.md` and `spot2654_3B.md`
- `applied_2654.json` body.applied_samples
- plan v13 (`org-name-refresh-plan-2026-10-19-v13.json.gz`) body.planned
- the saved bodies in `bodies2654/` (3A_*, 3B_*)

Labels follow `u1/judgements.json`. That file has a precedent for a misprint fix: head_current_dated30 23626335, "R GmbHx typo".

Only one sample in this block was contested: #29, org 22081870. Judge A said R and Judge B said N. Neither judge said W or REVERT, and both found the org serving its new name. The other 9 samples in the block had no contest to resolve.

## #29 — 22081870 (PL, NIP 2220915518, Mysłowice): **R**

| | Spelling | Plan count | Newest |
|---|---|---|---|
| old head | `Usługi Techniczne Inżynieryjne Kamil Lisowski` | 2 (H) | 2023-10-20 (`head_newest`) |
| new (elected) | `Kamil Lisowski Usługi Techniczno-Inżynieryjne` | 13 of 22 (T) | 2026-05-12 (`top_newest`) |
| rival | `Usługi Techniczno Inżynieryjne Kamil Lisowski` | 4 | 2026-07-22 (`rival_newest`) |
| other | `Oferta1 …` / `Oferta 2 Usługi Techniczne Inżynieryjne Kamil Lisowski` | 1 + 1 | 2025-07-18 |

Is it serving the new name? Yes. GET /v1/organizations/22081870 at 12:17:30Z (`3C_org_22081870.json`) returns `Kamil Lisowski Usługi Techniczno-Inżynieryjne`. This matches the judges' reads at 12:10:44Z and 12:13:24Z.

Prints, all from tenders in the org's winner list (`3A_lw_22081870.json`), mention_name per procedure:

| Tender | Published | Printed spelling |
|---|---|---|
| 7675221 (notice 23513405) | 2023-10-20 | `Usługi Techniczne Inżynieryjne Kamil Lisowski` (LOT-1, LOT-14). This is the head. |
| 868736 | 2024-09-09 | `Usługi Techniczno Inżynieryjne Kamil Lisowski` (rival) |
| 895929 | 2025-01-13 | `Kamil Lisowski Usługi Techniczno-Inżynieryjne` (elected) |
| 986115 | 2025-07-18 | `Oferta1 …` / `Oferta 2 Usługi Techniczne Inżynieryjne Kamil Lisowski`. The buyer's bid labels are glued onto the name. |
| 36116 | 2026-05-12 | `Kamil Lisowski Usługi Techniczno-Inżynieryjne` (elected) |
| 1163480 | 2026-06-01 | `Usługi Techniczno-Inżynieryjne Kamil Lisowski` (rival words, with the hyphen) |
| 765377 | 2026-06-29 | `Usługi Techniczno Inżynieryjne Kamil Lisowski` (rival) |
| 994832 | 2026-07-22 | `Usługi Techniczno Inżynieryjne Kamil Lisowski` (rival) |

I read the head at source: GET /v1/notices/23513405/content at 12:17:33Z (`3C_nc_23513405.xml`). TED-OFFICIALNAME is `Usługi Techniczne Inżynieryjne Kamil Lisowski`, with `NIP: 2220915518`, 41-400 Mysłowice. That is the same NIP the org carries, so the head and the served name belong to one sole proprietorship. Every print comes from a buyer's result notice, mostly Wody Polskie. None of them is the firm's own registry entry.

Why R and not N:
- **The head is a misprint, not a name.** "Usługi Techniczne Inżynieryjne" puts two adjectives side by side. Every other print from 2024 to 2026 uses the linking form "Techniczno(-)Inżynieryjne", in either word order. The only later prints with "Techniczne" are the 2025-07-18 `Oferta1` / `Oferta 2` labels. Those are malformed in the same way, with junk in front.
- **The elected spelling fixes that misprint.** It is also the majority form (13 of 22), and it prints the compound correctly with the hyphen. Fixing a misprint is R by the u1 precedent (23626335, "GmbHx typo"). The test is whether the new name is better than the old head, and it is.
- **The word-order question does not make it N.** The rival holds the three newest procedures (2026-06-01, 2026-06-29, 2026-07-22), and in procedure count it roughly ties the elected spelling. But it is the same words in the other order, and two of its three newest prints drop the hyphen. Choosing between two correct orders is neutral. It would only matter if we were labelling the elected name against the rival, and the label is old against new.

Why not W or REVERT (neither judge claimed them; checked anyway):
- **Same entity.** The NIP is the same, the city is the same, and the trade words are the same.
- **Not a former name.** The elected spelling is printed on 2025-01-13 and 2026-05-12, alternating with the rival from 2024-09 to 2026-07. The head spelling's last exact print is 2023-10-20.
- **Not clearly worse.** It is better than the head, as explained above.

Judge B's own note says "R would be defensible", and Judge A's says "N is defensible". The two judges differ only in how much weight they give the misprint fix. The source print settles it in favour of R.

## Block 3 tally after resolution

The 9 uncontested samples keep the labels both judges gave them: #20 (15497200) is N, and #21 to #28 are R. Judge A rated #28 "R (mild)", and Judge B plain R. With #29 resolved as R, the block is **9 R, 1 N, 0 W, 0 REVERT**.

This resolution does not change the Go count for block 3: 0 former-name reverts and 0 other wrong.

## Budget

- 2 new public GETs, spaced at least 2 s apart, with curl `--max-time 30`, no 408s and no retries. Both are logged in `api_requests.log` (12:17:30Z, 12:17:33Z).
  - /v1/organizations/22081870 saved to `bodies2654/3C_org_22081870.json`
  - /v1/notices/23513405/content saved to `bodies2654/3C_nc_23513405.xml`
- No /v1/sql, no ssh, no POSTs, no git, and no writes outside `u3/`.
