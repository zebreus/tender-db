# 572 unit 2: independent re-score of R1 and R2

Date: 2026-10-24. Reads only the saved files in `572-triage/u2/`: `q8_rows.out.json`, `q8_rows.sql`,
`q8_rows.plan.txt`, and the 54 bodies under `bodies/`. It also uses the 10-19 references `q4_class_refs.summary.json`,
`q6_current.sql`, `q6_current.summary.json`, `q9a_principal_orgs.out.json`, `triage_frozen_lists.json` and
`measure_a.py` (only for ASSIGN, OWN_CLASS and AGENTS). It made no box statement and no GET.
The re-derivation is my own code, not a re-run of the reader's `classify.py`, `sample.py`, `score.py` or
`r2_score.py`. Session-local copies are in the scratchpad: `rescore/rescore.py` and `rescore/r2.py`.

**Verdict: the reader's conclusion stands.** R1 is 40 R / 0 W / 0 replaced N, R2 is 6 of 6 with 0 W, and the stop
rule does not fire.

## Inputs checked

- **The q8 query is q6, ungrouped.** All 22 `WHERE` clauses in `q8_rows.sql` are byte-identical to `q6_current.sql`,
  arm for arm. The FROM/JOIN clause is the same. The SELECT list is exactly the decision's 7 columns: `tender_id`,
  `p.seq = t.current_seq`, `organization_id`, `role`, `notice_id`, `section_id`, `name`.
- **The plan is SEARCH only.** `q8_rows.plan.txt` has 132 SEARCH lines, which is 66 for each of its two probes, and 0
  SCAN. That is the same as `q6_current.plan.txt`.
- **The output is complete.** `q8_rows.out.json` has 263 rows, `row_count` 263, and `truncated` false.

## R1

### Class rows

These use analyze_q6.py's exclusions: OWN_CLASS V/U/T, and a P@A name matching its agent's pattern. ASSIGN carries
30926924 → NWSSP A@P.

| | current | older version |
|---|---|---|
| P@A | 99 (10-19: 97) | 56 (56) |
| A@P | 13 (13) | 25 (25) |

- There are 193 class rows, each on a distinct (notice, section). All 191 of q4's 10-19 mentions are present, so
  q4 minus q8 is empty. Two are new, and both are current P@A rows:
  - 8853907 / 47418946: "COMMUNITY GATEWAY ASSOCIATION LIMITED" on NHC 30953692. It is not in
    `triage_frozen_lists.json` list (a); "community gateway" occurs 0 times in the file. It is therefore an unreviewed
    key for monitor M.
  - 8858460 / 47428982: "Cardiff and Vale University Health Board" on NWSSP 30913738. List (a) already has this key
    (`cardiff and vale university health board`, 9 mentions, first notice 46731877).
- The 5 transferred mentions, keyed by (notice, section) from q4, are still bound as on 10-19:
  - Riverside 8619331, current, Tenderer;
  - Hywel Dda 8628060, current, Tenderer;
  - NWSSP on Swansea University 8640032, older version;
  - Pagabo 8678254 and 8678255, both current.

  All 5 are excluded. Excluding by notice id alone gives the same eligible set.

### Sample (decision.md "Read bars", R1)

- **(a) Exemplars.** Each of the 10 current exemplars has exactly 1 current class row. The 3 v1-only exemplars
  (8654651, 8621643, 8613154) have 5 class rows between them (1, 1 and 3), all on older versions. None is carried by
  today's `parties[]`:
  - 8654651 serves Aneurin Bevan under its own org;
  - 8621643 serves Cardiff Council and WLGA;
  - 8613154 serves Maidstone BC.

  None of the three has a version published since 2026-10-19; their latest versions are 07-14, 08-07 and 06-19. So
  they are the decision's "read and expected N". They are not version-gain Ns, and they use none of the 5 replacement
  slots.
- **(b) Salted rows.** Eligible rows are current, not transferred, and not on any of the 13 exemplar tenders. There
  are 98: 9 A@P on 9 tenders, and 89 P@A on 89 tenders.
  - Each stratum is sorted by `sha256('572-u2-' + tender_id + '/' + notice_id + '/' + section_id)` hex, ascending. Each
    tender keeps its first row only. The first 8 A@P are taken (9 are available), then the first 22 P@A.
  - Tender 8626810 is in both strata: A@P at 0-based position 3, which is picked, and P@A at position 63, which is
    outside the first 22. Picking P@A first gives the identical 30.
  - My 8 + 22 match the reader's `r1_sample.json` picks exactly, in the same order, with the same tender, notice,
    section, role, org and name.
  - The reader wrote "P@A 88". That is the P@A tender count after 8626810 is removed for the A@P pick; it is 89 before.
    This is wording only.

### Scoring

The rubric was applied to each `bodies/v1_tenders_{id}.json`:
- **R** needs a `parties[]` entry with `role` equal to the recorded role, `organization_id` equal to the recorded
  holder, and `mention_name` non-null and UTF-8 byte-equal to q8's `name`.
- **N** applies only if a version was published on or after 2026-10-19.
- **W** is anything else.

None of the 40 names has leading or trailing whitespace.

| set | tender | dir | holder | printed name (q8) | notice | score |
|---|---|---|---|---|---|---|
| ex | 8641682 | P@A | 30953692 | Birmingham City Council | 46795453 | R |
| ex | 8636304 | P@A | 30953692 | Darlington Borough Council | 46789661 | R |
| ex | 8677647 | P@A | 30953692 | Karbon Homes (54North Homes) | 46794092 | R |
| ex | 8698064 | P@A | 30913738 | Health Education and Improvement Wales (HEIW) | 46805032 | R |
| ex | 8655231 | A@P | 30917275 | Star Procurement | 46817944 | R |
| ex | 8677040 | P@A | 30913798 | Rutland County Council | 46845134 | R |
| ex | 8639202 | P@A | 30968739 | The Howard Partnership Trust | 46737636 | R |
| ex | 8580124 | A@P | 30914436 | GLEEDS COST MANAGEMENT LIMITED | 46698168 | R |
| ex | 8596944 | P@A | 30913722 | English Rural Housing Association | 46757368 | R |
| ex | 8687157 | P@A | 30914820 | Remarkable Autism | 46869370 | R |
| A@P | 8677564 | A@P | 30928312 | NHS Wales Shared Services Partnership-Procurement Services (hosted by Velindre University NHS Trust) | 46823552 | R |
| A@P | 8610192 | A@P | 30926924 | (same NWSSP long form) | 46751168 | R |
| A@P | 8614798 | A@P | 30926924 | (same NWSSP long form) | 46758368 | R |
| A@P | 8626810 | A@P | 30919377 | NHS WALES SHARED SERVICES PARTNERSHIP | 46819472 | R |
| A@P | 8655040 | A@P | 30917275 | Star procurement | 46817583 | R |
| A@P | 8678156 | A@P | 30929723 | (same NWSSP long form) | 46697948 | R |
| A@P | 8636214 | A@P | 30967593 | TMT Procurement Partnership | 47290364 | R |
| A@P | 8648971 | A@P | 30974616 | Scape Group Limited (trading as SCAPE) | 46807157 | R |
| P@A | 8678350 | P@A | 30953692 | Ongo Homes Limited | 46848785 | R |
| P@A | 8678872 | P@A | 30953692 | Rochdale Borough Council | 46857436 | R |
| P@A | 8595312 | P@A | 30913722 | Hull Churches Housing Association | 46755465 | R |
| P@A | 8668300 | P@A | 30953692 | City of Lincoln Council | 46841445 | R |
| P@A | 8858460 | P@A | 30913738 | Cardiff and Vale University Health Board | 47428982 | R |
| P@A | 8653366 | P@A | 30953692 | London Borough of Newham | 46814746 | R |
| P@A | 8673075 | P@A | 30953692 | Carmarthenshire County Council | 46850020 | R |
| P@A | 8668723 | P@A | 30953692 | Stockton on Tees Borough Council | 46842261 | R |
| P@A | 8677488 | P@A | 30968739 | The Keys Academy Trust | 46785018 | R |
| P@A | 8678322 | P@A | 30953692 | HOUSING 21 | 46843775 | R |
| P@A | 8678054 | P@A | 30953692 | THE GUINNESS PARTNERSHIP LIMITED | 46818155 | R |
| P@A | 8639888 | P@A | 30913738 | Hywel Dda University Health Board | 46821071 | R |
| P@A | 8678310 | P@A | 30953692 | ONWARD HOMES LIMITED | 46844733 | R |
| P@A | 8678131 | P@A | 30953692 | Moat Homes Limited | 46823501 | R |
| P@A | 8643140 | P@A | 30913738 | Cardiff and Vale University Health Board | 46828632 | R |
| P@A | 8649052 | P@A | 30953692 | Lancaster City Council | 46806856 | R |
| P@A | 8635466 | P@A | 30913738 | Public Health Wales NHS Trust | 46788140 | R |
| P@A | 8627364 | P@A | 30913722 | YMCA Saint Pauls Group | 46801228 | R |
| P@A | 8668115 | P@A | 30953692 | Bristol City Council | 46841024 | R |
| P@A | 8678133 | P@A | 30953692 | ISLINGTON & SHOREDITCH HOUSING ASSOCIATION LIMITED | 46823485 | R |
| P@A | 8582562 | P@A | 30953692 | Stoke on Trent City Council | 46697938 | R |
| P@A | 8609020 | P@A | 30913738 | Hywel Dda University Health Board | 46822958 | R |

- All 40 rows are Procedure-Buyer, and each body has exactly 1 matching party.
- Only one tender has a version since 10-19: 8858460, whose single version is the new notice 47428982. It carries the
  row, so it scores R, not N. No other scored tender has a version since 10-19.
- **R1 = 40 R / 0 W / 0 N (0 replacements).** This matches the reader's `r1.json` tally (R 40, N 0, W 0, v1-only N 5).
  The bar is met.

## R2

All 11 `?buyer=…&limit=1000` bodies are single pages, each with `more` false, `next_cursor` null and
`ignored_filters` empty. None has a duplicate id, and the largest list is NWSSP's 693, under 1000.

| tender | must include | present | must exclude | present | result |
|---|---|---|---|---|---|
| 8641682 | 30953692 NHC (75) | yes | 30914707 Birmingham (199) | no | pass |
| 8636304 | 30953692 NHC | yes | 30914393 Darlington (172) | no | pass |
| 8677040 | 30913798 Welland (121) | yes | 30915335 Rutland (76) | no | pass |
| 8698064 | 30913738 NWSSP (693) | yes | 30917293 HEIW (32) | no | pass |
| 8596944 | 30913722 Gibbs Laidler (84) | yes | 2291108 English Rural HA (6) | no | pass |
| 8655231 | 30917275 Trafford (63) and 30916230 Rochdale (85) | yes, yes | none | n/a | pass |

- **R2 = 6 of 6, 0 W**, as the reader reported.
- **Sizes against `q9a_principal_orgs.out.json` (10-19).**
  - These agree with the reader: NHC 74 → 75, Birmingham 198 → 199, HEIW 31 → 32, Darlington 172 → 172.
  - Rutland 76 and Rochdale 85 are also unchanged.
  - Trafford has no 10-19 count in q9a, so I did not check the reader's "Trafford unchanged".

## Request log (read only)

- **Counts.** `requests.log` has 1 SQL line and 54 GET lines tagged `(572 u2 …)`:
  - 13 R1ex, 30 R1s and 11 R2;
  - all `rc=0` or `rc=200`, with no retries;
  - the 43 tender GETs are exactly the 13 exemplars plus the 30 picks.
- **Timestamps.** The log writes each time after curl returns, so it records completion times. `get.sh` spaces request
  starts by 2.2 s using `.last_get`, which it writes before curl runs. The one 1-second gap, lines 139 → 140, follows
  the 693-item NWSSP list, which was slow to download. It does not show a start gap under 2.1 s.
- **Not verifiable here.** The "job queue idle" checks cannot be confirmed from the saved files, and this rescore
  made no box reads.

## Discrepancies

None that affect the conclusion. There are two wording points:
- "P@A 88" is the eligible P@A tender count after the cross-stratum dedupe; it is 89 before.
- "Trafford unchanged" has no 10-19 count to check against in q9a.
