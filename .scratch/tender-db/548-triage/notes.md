# 548 triage — 2026-10-19

Read-only. Box use: 0 admin reads succeeded (1 refused, below), 1 `/v1/sql` request (q01, 1.6 s, 8 rows),
2 public GETs (`api_requests.log`). No writes, no jobs, no code edits, no git.

## The stated Next step is blocked

Next was: measure each hold's right/wrong split on its full listing (`held` in `org-name-refresh-plan`).

- The listing exists only in the stored report (`reports` row, kind `org-name-refresh-plan`).
- `reports` is not on `/v1/sql`'s allow-list (`crates/app/src/v1/sql.rs` `ALLOWED`), so no compliant SQL path exists.
- The one admin path, `ssh root@zebreus.click "/root/aj.sh /admin/reports/org-name-refresh-plan"` (a single
  primary-key read, `admin.rs::report` → `Db::latest_report`), was REFUSED by this session's permission
  classifier ("Production Reads"). It was not retried and no workaround was tried.
- Rebuilding the holds from mentions would be a corpus-scale walk (5.68 M orgs and 49.7 M mention rows in plan
  2562). The rules forbid that.

So the full-listing measurement is still open. It needs a session where the report read is allowed (524's own
sessions have read it before). Per triage-labels.md, the issue stays `ready-for-agent` with the blocker recorded.

## What code reading settles without the listing (rev 75d8da3, epoch `524u2-v12`)

`store::head_election::elect_head_counted`, in order: E1–E3 → R0 → D4 → S1 → bars → window (`recent`) → residue →
serve [bind guards = `head_change_hold` (family, gb_family, consortium_removed, nhs_check, placeholder_id) → R2 only:
`head_r2_hold` (initialism) → serve holds] → `head_current` (R1, head printed within 1,095 days) → `top_stale`
(R1, a rival cluster, top's newest print older than 365 days).

1. **The issue's counts (553 holds, plan 2515) are from rule v5.** That was before the window (v7), residue (v8),
   serve holds (v9), `head_current` (v10) and `top_stale` (v11). Today a bind guard only sees a change that has
   already passed the window and the residue holdout. A hold it RELEASES would then meet `head_current` and
   `top_stale`, and only an R1 change can ship (every wet run passes `exclude_rules:["R2","R0w"]`). So
   "~270 of ~350 keep a worse head" overstates what a release would ship now. The shippable part of a released
   bind-guard hold is: rule R1, AND head_newest older than as_of − 1,095 d (or a junk release), AND not
   (top_newest older than 365 d with a rival cluster). Every one of those fields is in each `held` entry (`rule`,
   `head_newest`, `top_newest`, `rival_newest`, `clusters`, `head`), so unit 1 can cut the listing offline before
   judging.
2. **`initialism` is R2-only** (`elect_head_counted` line ~776: `if rule == HeadRule::R2 { r2_hold }`). R2 is
   withheld whole (v9 fallback, 2026-10-16). So none of the 10-of-30 "wrong holds" could ship if released. The
   line is moot while R2 stays planner-only.

## Probe q01 (bounded, plan read locally first): the gb_family exhibits read the other way round

Plan (`q01_gb_spellings.plan.txt`, sqlite3 on the v551 plan copy): `SEARCH m USING INDEX organization_mentions_org
(organization_id=?)`, `SEARCH n2 USING INTEGER PRIMARY KEY`, for each arm of the UNION ALL. Org ids from the public
API's `?identifier=` lookups (`g_ident_*.json`).

| org | CH no. (register name, 469/448 caches) | served head | spellings (n, newest print) |
|---|---|---|---|
| 11628199 | 02081330 (CAPITA PLC) | `Capita Business Services Limited` | head 1 (2018-06-15); `Capita PLC` 2 (2026-09-16), `Capita Plc` 1 (2024-01-16) |
| 10213161 | 04958135 (SCOTIA GAS NETWORKS LIMITED) | `Southern Gas Network PLC` | head 1 (2017-08-11); `SCOTIA GAS NETWORKS LIMITED` 47 (2026-10-02), `… Ltd` 1, `… Plc` 1, `Scotland Gas Networks (SGN)` 1 |

- The issue describes the first as "Capita plc → Capita Business Services Ltd" (a parent head on a subsidiary's
  number). The data says the opposite. The number is the parent's, the HEAD is a subsidiary's one stray print, and
  the held change serves the register entity's own name.
- 10213161 is the same shape: a sibling group company's single 2017 print, on Scotia Gas Networks Ltd's number.
- In both, H = 1 and the head is older than 3 years, so `head_current` would not re-hold them. The top was printed
  in 2026, so `top_stale` would not either. A gb_family release would ship both, and both would be right.
- Capita's top holds 3 of 4 votes (75%). The issue's candidate rule ("CH number and winner ≥ 90%") would keep it
  held. A "stray stale head" release would free both: H ≤ 1, head_newest older than 3 years, R1 passed.

## top_stale lead, measured offline on 524 gate (d)'s 20 judged holds (`top_stale_t1.py`, `.out.txt`)

Variant T1: hold `top_stale` only when `rival_newest > top_newest`, i.e. a rival printed after the top. Otherwise
release.

| arm | harmful hold (change better) | neutral | right hold (change worse) |
|---|---|---|---|
| released by T1 (12) | 5 | 6 | 1 (5638950 `Brlić d.o.o.` → `Brlic`: same entity, diacritic and form lost) |
| still held (8) | 4 (9938875, 6160661, 15346526, 15082073) | 2 | 2 (7301922 real rename to a fresher rival; 8291325 rival is another entity) |

- T1 recovers 5 of the 9 harmful holds.
- The one right hold it releases is a degraded same-entity spelling, not another entity or a revert.
- Both right holds whose rival is fresher stay held. Those are the cases where "elect the rival" would be right
  (7301922) or wrong (8291325, `Seine et Yvelines Numérique`, another entity). So electing the rival is NOT safe as a
  blanket rule.
- 20 items is a small sample, and these 20 are on the skip list, so a fresh dated draw of the released arm is the
  bar before any build ships.

## Files

- `q01_gb_spellings.sql`, `.plan.txt`, `.out.json`, `run.sh`, `sql_requests.log`
- `g_ident_02081330.json`, `g_ident_04958135.json`, `api_requests.log`
- `top_stale_t1.py`, `top_stale_t1.out.txt`
