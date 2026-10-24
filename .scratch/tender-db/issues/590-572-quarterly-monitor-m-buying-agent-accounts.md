# 590 — 572's quarterly monitor M (buying-agent accounts), first run 2027-01-19

Status: ready-for-agent — monitor carrier, filed 2026-10-24 from 572's close (`572-triage/u2/close.md` "Gaps": with 572
DONE, nothing on the board held its quarterly run). Next run on or after **2027-01-19**, then every quarter; the hourly
routine's audit step picks it up on the first firing on or after the date. Before the date nothing is owed.
Kind: monitor (a reopen trigger for 572's decision 572-L)
Relates to: 572 (rule 572-L, its "Monitor M" and "Reopen" lists), 589 (F1, `procuringEntity` as a role: trigger 3),
545 (the two Tenderer slips still bound at M0), 524 (RPJ3's head).

## What each run does

1. Re-run `572-triage/u2/m0_monitor.sql` (one bounded `/v1/sql` statement over the 8 agent-account orgs resolved by
   identifier; plan-probe it first, SEARCH only; never retry a 408) and score it with `m0_score.py` against the frozen
   list (a). Add the q6 shape for current P@A buyer rows (572 RESOLVED-VERIFIED; M0 read 97, trigger 190).
2. Add the two principal-share triggers M does not carry (32 or more principals above 10% of their own list; any
   principal with a list of 20 or more tenders above 30%): the q9 shape (`q9a` + `q9b`; `572-triage/build_q9.py`), or bound them from
   the change in routed rows as M0 did. At most 1 more statement.
3. Compare every line to 572's reopen triggers; append the run to 572 (a dated line under "RESOLVED-VERIFIED") and set
   this issue's next date one quarter on.
4. Any trigger fired → reopen 572 as "serve the principal as buyer" with the numbers.

## Verify

    python3 -c "import json;print(json.load(open('.scratch/tender-db/572-triage/u2/m0.json')).get('pa_on_agent_accounts'))"

- **M0 (2026-10-24)**: P@A on the agent accounts 154 (152 without the 2 slips; trigger 306), NHC member mentions 97 over
  52 bodies (trigger 194), Velindre on NWSSP's PPON 49 (50 by substance, notice 47434180), unreviewed keys 1 (COMMUNITY
  GATEWAY on NHC, 47418946), current P@A buyer rows 97 (trigger 190). No trigger fired.
- **done** (per run): the run is appended to 572 and the next date is set; this issue stays open as the carrier.
