# 466 — GB company numbers minted after 452's register census were never checked, and the census cannot be re-run

Status: ready-for-agent — UNITS 1, 3 AND 4 DONE 2026-10-10: 514 verdicts POSTed as cohort `466-census-2026-10-10`
(recorded 514, stale 0, 417 served statuses changed). The rekey dry run (job 2111) plans 175 merges and 81 moves.
NEXT: unit 4's last step, the 453-shape register review of those 256 plan rows; then the wet rekey; then unit 2 (the 83
`related` numbers) and the Verify.
- **Unit 4, the review** (`wf_90b6a9e0-d81`, 45 agents, about 4.5 M tokens).
  - The reviewer and challenger read every one of the 535 cases (`452-census/cases-466-2026-10-10.json`) against
    `rubric.md`. An adjudicator decided every disagreement and every residue case: 47 adjudicated, 488 agreed.
  - Verdicts (`452-census/verdicts-466-2026-10-10.json`): **355 wrong** (284 with the right number found), 97 right,
    62 related, 21 unclear (not posted).

    | kind | wrong | related | right | unclear |
    |---|---|---|---|---|
    | live-mismatch (185) | 49 | 49 | 80 | 7 |
    | never-issued (257) | 257 | | | |
    | absent-mismatch (59) | 48 | 8 | 2 | 1 |
    | 452 residue (34) | 1 | 5 | 15 | 13 |

  - The residue: 21 of 34 settled; 13 stay unclear with the reason written in the verdict.
  - Register pages: Companies House blocked the box's IP with 403s after about 480 fetches, so the last 54 were
    fetched from the session container. A census run should keep its page fetches under a few hundred per IP.
- **POST.** `452-census/identifier-verdicts-466-2026-10-10.json` (`verdict_post.py`): recorded 514, stale 0,
  changed 417. From the next planner run or fold, the 355 wrong numbers are withheld from R2/E0/R3, the altid arm
  and the resolver's canonical bind, and the 417 orgs serve `register_mismatch` or `related_entity`.
- **Rekey dry run, job 2111** (`466-census/rekey-plan-2111.json`). 469 wrong verdicts carry a right number, across
  452 and 466.
  - Not acted on: 170 gone (applied by 453), 10 not high, 3 unkeyed.
  - Denied: 25 names, 1 legal form, 1 withheld target, 1 pending move, 1 destination verdict, 1 keep verdict.
  - Plan: **175 merges + 81 moves**. A merge deletes the wrong-number org into the right number's org, so 453's step
    applies before the wet run: an adversarial register review of every right number in the plan. 453's review held
    2 of 158 whose right number was itself wrong.
- **Unit 3, the join.**
  - Input: the public listing above the watermark (`cursor=31590759`), read 2026-10-10. 10,150 GB national orgs, up
    from 8,057 on 10-01: the dailies kept minting. Saved on the box as `/root/census/gb-orgs-above-31590759.jsonl`.
  - **A gap in 452's shape.** `GBCOH3433043` is company 03433043 with its leading zero dropped. 452's rule (8
    characters after `GBCOH`) skipped every 6- or 7-digit `GBCOH` number: 1,408 orgs on the 09-30 input were never
    checked. `census.py --pad-short` restores the zeros. It is off by default, so the 452 reproduction stays exact.
  - Against the 2026-10-01 snapshot, with `--pad-short`:

    | cohort | orgs (numbers) | match | mismatch | absent | disjoint |
    |---|---|---|---|---|---|
    | above the watermark | 7,219 (7,063) | 6,215 | 566 | 438 | **148** |
    | the 09-30 input, now with the short numbers | 32,265 (31,435) | 29,716 | 1,431 | 1,118 | 390 |
    | of which new against 452 (≈ the 1,408 short numbers) | | | +128 | +109 | **+35** |

  - So unit 4's review set is: 148 disjoint and 438 absent above the watermark; about 35 disjoint and 109 absent among
    the newly read short numbers; and the 34 residue cases. Absent numbers first need their register page, to split
    never-issued from dissolved.
- **Unit 1.** `452-census/census.py` is committed. It imports 448's matcher (`core`, `matches`) from
  `448-campaign/altid_cases.py`. Its inputs are committed beside it: `gb-orgs-2026-09-30.jsonl` (the box's
  `/root/gb-orgs.jsonl`, 58,938 rows) and `watermark.json` (`max_org_id` 31,590,759, snapshot 2026-09-01). Both
  snapshots, 2026-09-01 and 2026-10-01, are in the box's `/data/archive/companies-house/`. The script runs on the box
  in 18 s.
  - **Reconstructed and pinned.** Over the 2026-09-01 snapshot and the 09-30 input, the script reproduces
    `counts.json` exactly: 28,545 / 1,303 / 1,009, from 30,857 orgs and 30,844 numbers. Its 1,009 absent rows equal
    `absent-from-live-register-2026-09-01.json` row for row.
  - The company-number shape had to be recovered from the totals: `GBCOH` plus any 8 of [A-Z0-9], or a bare 8 digits,
    or a bare 2 letters + 6 digits. It is the only tried rule that gives both 30,857 and 30,844. It also admits the
    `0C415849` and `X338EBHC` 452 named.
  - **Live-name-disjoint: 355 against 452's 354.** 452 left no rule, so its split was reconstructed by sweeping
    variants. The best has 2 more and 1 fewer:
    - extra: Reef Cleaning Solutions under GSO LIMITED, and Milestone Infrastructure under M GROUP (SERVICES). Their
      heads share nothing with the register, so 452 most likely judged them on mention names, which the census input
      does not carry;
    - missing: DRPG (UK) under DRP (UK), a 3-letter prefix the rule counts as a spelling.
    - For a list of review candidates this is close enough, and the rule is now written down (`related_spelling`).
Was: ready-for-agent — filed 2026-10-01
Kind: data quality (identifiers)
Relates to: 452 (the census and its 685 verdicts), 453 (the rekey arm, `wrong` verdicts only), 454 (name-denial review),
342 (the FTS backfill chunks that minted the new orgs), 448 (the matcher and `ch_fetch.py`; its delta review covers
COH↔PPON pairs only), 456 (a stray mention heads an org; one of 452's disputed cases has that shape)

## What is wrong

### 1. The census was taken once, and the corpus kept growing after it

452's census input is `/root/gb-orgs.jsonl` on the box. It has 58,938 rows, was written 2026-09-30 09:50 UTC, and its
highest org id is **31,590,759** (read 2026-10-01). The census commit `9f68699` is from 09:53 UTC. By then FTS
backfill chunks 1–2 had landed (project 1689 finished at 05:10 UTC). The following were folded after it (finish times
from `GET /admin/jobs`, 2026-10-01):

| chunk | months | jobs | project finished (UTC) |
|---|---|---|---|
| 3 | 2025-01 → 2025-05 | 1719–1725 | 2026-09-30 16:11 |
| 4 | 2024-05 → 2024-12 | 1728–1737 | 2026-09-30 18:37 |
| 5 | 2023-09 → 2024-04 | 1740–1749 | 2026-09-30 20:10 |
| 6 | 2023-01 → 2023-08 | 1750–1759 | 2026-09-30 21:41 |
| 7 | 2022-05 → 2022-12 | 1776–1785 | 2026-10-01 09:18 |
| 8 | 2021-09 → 2022-04 | 1789–1798 | 2026-10-01 11:34 |
| 9 | 2021-01 → 2021-08 | 1800–1809 | 2026-10-01 13:12 (read 13:3x; at 12:09 fetch 1802 was still running) |

The 2026-10-01 07:35 UTC daily tick ran in the same window.

The public listing above the watermark, read 2026-10-01 12:0x UTC
(`/v1/organizations?country=GB&kind=national&limit=1000&cursor=31590759`, 9 pages), returns **8,057** GB national
orgs, ids 31,591,419 → 31,634,553:
- **6,259** carry a `GBCOH…` identifier, which is the census's own company-number shape. None is a bare 8-digit number
  or two letters + 6 digits.
- All 8,057 serve `"identifier_status": null`.
- **298** of the 6,259 cannot be a company number at all. They are neither 6–8 digits nor two letters + 6 digits:
  `GBCOHCOMPANYNUMBER822508` on St Mungo Community Housing Association (31591846), `GBCOHDN2722` on Surrey County
  Council (31593234), `GBCOHHOUSING21` on HOUSING 21 (31593664).

Re-read 2026-10-01 13:3x UTC, after project 1809 finished: the same 8,057 / 6,259 / 298 over the same id range. No GB
national org stands above 31,634,553.

452 measured about 1.2% wrong and 0.3% related numbers. Applied to 6,259 orgs, that is roughly 75 wrong and 20
related. This is an extrapolation: the 2021–2024 suppliers may not have the same rate.

### 2. Nothing protects an org whose number was never reviewed

A 452 verdict applies to the exact (identifier, kind, country) triple it names. An org minted under a number that no
verdict names is not withheld from R2/E0/R3 or the altid arm, and it is not flagged. 452's resolver guard covers only
another spelling of a number that already has a verdict.

The only register reading since the census is 448's delta review of new COH↔PPON pairs (dry 1786 / wet 1787, 36
pairs). Notices from before the Procurement Act carry no PPON. For chunk 7, project 1785 armed the alias and asked
0 (its job row carries no alias suffix, which `alias_suffix` leaves empty only when nothing was asked,
`crates/app/src/supervisor.rs:2388`). After chunk 8, the altid dry run 1799 planned 0. So for 2021–2022 that path checked nothing.

No code in `crates/` or `ops/` reads a company register. The weekly Sunday tick (`run_report_tick`,
`crates/app/src/supervisor.rs:11811`: disk-census, ghost-census, member-twin-census, registry-contiguity,
data-quality, rehash probe, build-org-match-keys, org-merge-health, scan-org-match-keys dry + wet) does not consult
one either.

### 3. The census cannot be re-run: its join script and its snapshot are gone

`.scratch/tender-db/452-census/` holds the following:
- `page_orgs.sh`, the `/v1/sql` pager;
- `verdict_post.py`;
- `rubric.md`;
- the census outputs (`counts.json`, `live-name-disjoint-candidates.json`, `absent-from-live-register-2026-09-01.json`,
  `absent-register-pages-2026-09-30.json`, the cases and the verdicts).

It does not hold the script that joined the register snapshot against `gb-orgs.jsonl` and split the orgs into
match / mismatch / absent. Commit `9f68699` added only the two JSON outputs, `counts.json`, `page_orgs.sh` and the
issue text.

The snapshot `BasicCompanyDataAsOneFile-2026-09-01.zip` went to "the session scratchpad, not the box" (452). It is
in neither place now:
- On the box, a `find` of `/`, `/data`, `/tmp`, `/var/tmp` and `/home` on 2026-10-01 found no `*BasicCompany*` file.
- `/data/archive` holds `doe`, `fts`, `rates` and `ted`, and no `companies-house` directory. 452's own plan, step 1,
  was to put it there.
- This container has no copy either.

These pieces survive:
- the census input, `/root/gb-orgs.jsonl` (box only);
- the matcher, `core`/`matches` in `448-campaign/altid_cases.py`;
- the register-page fetcher, `448-campaign/ch_fetch.py`;
- 453's `ch_check.py`.

Companies House still publishes the 2026-09-01 snapshot (492.6 MB). The 2026-10-01 one is also out (494.0 MB,
last-modified 2026-10-01 08:10 GMT).

### 4. 452's 35 unclear and disputed verdicts were never settled

`verdicts-2026-09-30.json` holds 720 verdicts. `verdict_post.py` skips 35 of them by design ("Not posted: unclear and
every disputed verdict"). 452's done Status names them as a follow-up, and no issue picked them up:

| verdict | count |
|---|---|
| `unclear` | 22 (21 low, 1 medium) |
| `disputed:right-number` | 6 |
| `disputed:related-company` | 4 |
| `disputed:unclear` | 3 |

The label after `disputed:` is the challenger's reading against the reviewer's. Westmorland and Furness Council
(31535203, `GBCOH03628053`) is an example:
- reviewer: "a statutory body under a random company's number";
- challenger: COURTS NOMINEES LTD is the council's own dormant nominee company, with the council as PSC since
  2023-04-01.

On 2026-10-01 all 35 orgs were live, each carried the reviewed number, and each served `identifier_status: null`. So
their numbers are still trusted for matching.

One of the 35 is not a number question: 8805068, headed "Fujitsu Services Limited" on 01920623. 448's altid verdict
`01920623~PTYH4442XPRH` (merge/high) reads 01920623 as the FCA's number and the Fujitsu mentions as strays. That makes
it a head-election problem (456's shape), which leaves **34** open.

### 5. 83 `related` verdicts carry a right number that no arm reads, and no decision covers them

The posted body (`identifier-verdicts-2026-09-30.json`) holds 102 `related` verdicts. **83** of them have a
`correct_identifier`: 53 high and 30 medium. Live on 2026-10-01, all 83 orgs stand, carry the reviewed number and
serve `related_entity`. A bounded identifier lookup of each right number, in both the bare and the `GBCOH` spelling,
found these owners:

| confidence | one standing org already carries the right number | none does |
|---|---|---|
| high | 40 | 13 |
| medium | 16 | 14 |

The rekey planner reads only `wrong` rows: `match_org_rekey` selects
`WHERE verdict = 'wrong' AND correct_identifier IS NOT NULL` (`crates/store/src/canonical.rs:17581–17583`). The other
readers of the column are the POST validation (`crates/app/src/admin.rs:475`) and the case-reviews readback.

452 decided that a related number is "flagged, not withheld" and that `correct_identifier` is "recorded, never applied
automatically". 453 noted "83 of the 102 related verdicts carry one too. Nothing reads it", then built the arm for
`wrong` only. Neither issue says whether the related ones should be re-keyed, reviewed or left alone.

Widening the filter is not enough. The resolver alias reads every stamped row, whatever its verdict
(`SELECT … FROM org_identifier_verdicts WHERE applied_literal IS NOT NULL`, `canonical.rs:9872–9873`). For a `wrong`
number that is right. A `related` number, though, is a real company's own number, so aliasing it would send that
company's later mentions to the named org.

## Proposed fix

The root cause: the census was a one-shot procedure that lived in a session. Its script and its snapshot went with
the session, and it recorded no watermark. So nothing can re-run it, and nothing shows how far behind it is. The fix
makes it a committed, incremental procedure.

1. **Tooling (first unit).** Commit `.scratch/tender-db/452-census/census.py`.
   - Input: a snapshot zip plus org rows (`[identifier, id, name]`). Output: the match / mismatch / absent split, the
     live-disjoint list and the absent list.
   - It imports the matcher from `448-campaign/altid_cases.py` instead of copying it.
   - Commit `/root/gb-orgs.jsonl` beside it as the 2026-09-30 input, plus a `watermark.json`
     (`max_org_id` 31590759, snapshot 2026-09-01).
   - **Pin:** over the 2026-09-01 snapshot and that input, the script must reproduce `counts.json` exactly
     (28,545 / 1,303 / 1,009) and the 354 rows of `live-name-disjoint-candidates.json`. Record any difference before
     going on.
   - Put the snapshots in the box's `/data/archive/companies-house/`.
2. **Decide the 83 related numbers.** This is independent of the census. Either:
   - (a) re-key the 53 high ones through `rekey` and keep them out of the alias, so the alias query reads
     `verdict = 'wrong'` only. Pin it with `a_related_verdict_re_keys_but_its_number_is_never_aliased` in
     `crates/store/tests/rekey.rs`: the org merges or moves, and a later mention of the related literal under the
     related company's own name does not reach the named org; or
   - (b) keep them as a flag, and write down why in `docs/operations.md` beside the rekey rule.

   Record the reasoning on this issue either way.
3. **Incremental run, after project 1809.**
   - Page the orgs above the watermark through the public listing (`cursor=<watermark>`, no `/v1/sql`).
   - Skip triples that already carry a verdict.
   - Join against the 2026-10-01 snapshot. For absent numbers, fetch the register pages (`ch_fetch.py`).
   - Write the new watermark.
4. **Review, settle the residue, post.**
   - Build the cases as 452 did: `rubric.md`, plus mention names by bounded reads.
   - Run reviewer + challenger, and add the 34 open residue cases to the same run. For the 13 disputed ones, a third
     adjudicator decides, because the first two already disagree.
   - Each case ends as `wrong`, `related` or `right`, or stays unposted with its reason recorded.
   - POST under cohort `466-census-<date>` with `verdict_post.py`.
   - Then dry-plan `rekey` for any new `wrong` verdict that has a right number. 453's arm and review shape cover it
     unchanged.
   - Record the measured wrong rate, and set the next run against a signal rather than a date: the count above the
     watermark is one public GET, and Companies House publishes a snapshot on the 1st of each month. No supervisor
     arm is proposed: the join is cheap, but every flagged case needs a reader, which a job cannot supply.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh '/admin/case-reviews?table=identifier&limit=5000'" | python3 -c 'import json,sys,collections; r=json.load(sys.stdin)["rows"]; print(len(r), sum(x["org_id"]>31590759 for x in r), sorted(collections.Counter(x["cohort"] for x in r).items()))'

This reads unit 4, the last one.

- **open** (2026-10-01 12:1x UTC, again 13:3x): `685 0 [('452-census-2026-09-30', 683), ('453-rekey-review-2026-09-30', 2)]`. No
  verdict names an org above the census watermark.
- **done**: the second figure is above 0 and a `466-census-…` cohort is in the list, meaning the incremental census's
  verdicts are posted. The settled residue rides the same cohort on ids at or below 31,590,759, so the first figure
  grows by those too.
