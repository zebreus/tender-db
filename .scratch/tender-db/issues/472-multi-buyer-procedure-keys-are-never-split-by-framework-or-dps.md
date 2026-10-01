# 472 — multi-buyer procedure keys are never split by what their notices declare (framework / DPS), so weld 430681 is still served

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the census, not a parser change: BT-765/BT-766 are already parsed into `notice_codes`, so the census reads the declaration per published BT-04 key and splits the multi-buyer keys by it.
Kind: data quality (identity / grouping): 377's one open direction, which no issue carried
Relates to: 377 (DONE 2026-09-11; its 2026-09-27 comment names this direction), 369 (the key gate whose pre-filter
this would widen), 364 (the weld gauge), ADR-0011 (previous-notice edges, which pull other procedures into a weld),
ADR-0002 (the field checklist)

## What is wrong

377 decided on 2026-09-10 that there would be no gate, because no buyer statistic separates a weld from a Dynamic
Purchasing System: both accumulate buyers across notices. Its 2026-09-27 comment named the one direction still open,
"a rule keyed on what the NOTICE DECLARES (a DPS or framework, BT-765 / BT-766)", and said it needs a census first.
377 is DONE and no open issue carries that census. Meanwhile the weld it convicted is still served. Live
`GET /v1/tenders/430681` on 2026-10-01 returned 789 versions, 246 parties, 446 lot_results and 391 contracts under
`Tervakosken koulu- ja monitoimikeskus, Vaihe 1`, with `procedure_key` `5e001394-80da-44e1-8955-e9fe17674c90` and
country `[DK032, FI1C2, DK013, DK011, FI1B1]`.

### The declaration is already parsed, and nothing reads it

- **Checklist status.** `sdk::decide` (`crates/ingest/src/eforms/sdk.rs`) decides BT-765-Lot/-Part and
  BT-766-Lot/-Part as `Codes` in every vendored inventory: EU SDK 1.0–1.15 and SDK-DE 2.0/2.1 (1.0 has no
  BT-766-Part). Their type is `code`,
  their codelists are `framework-agreement` and `dps-usage`, and the 1.9+ `-List` fields are `Attribute`. The national
  dialects file the same element as `DE1-…-ContractingSystemTypeCode` and `SDK01-…-ContractingSystemTypeCode` codes. So
  every notice's declaration is already a `notice_codes` row (`field_id`, `list_name`, `code`), and
  `/v1/notices/{id}/content` serves it. All 789 notices of 430681 were read that way on 2026-10-01.
- **Nothing downstream reads it.** No `.rs` file under `crates/` names BT-765, BT-766, `ContractingSystem` or
  `dps-usage`. Those names appear only in the vendored SDK JSON and in test fixtures. The planner's identity row
  (`Ident::read`, `crates/ingest/src/project.rs:4109`, written to `plan_notice`) carries `key_shaped` and
  `buyer_key` but not the declaration. 369's refusal (`plan_refused_key`, `crates/store/src/canonical.rs:9124`) is
  gated on `key_shaped = 1` alone.

### What the declaration shows on 377's own four tenders

Every notice of 377's four "spread" tenders (430681, 333104, 1012301, 769785) was walked through
`/v1/notices/{id}/content` on 2026-10-01 (1,479 GETs) and grouped by the BT-04 each notice publishes. The table lists
the five keys with more than 70 notices; 1012301 has none, and it is covered further down. A notice "declares a DPS"
when any of its `dps-usage` codes is not `none`, and "declares a framework" likewise for `framework-agreement`.

| tender | published BT-04 | notices | distinct titles (BT-21) | distinct buyer names | buyer countries | DPS | framework | neither | no row |
|---|---|---|---|---|---|---|---|---|---|
| 430681 | `aff2863e-…` | 777 | 705 | 416 | CHE 379, DNK 24, FIN 12, FRA 1 | 9 | 2 | **761** | 5 |
| 769785 | `85e08737-…` | 128 | 79 | 48 | ITA 48 | 0 | 1 | **127** | 0 |
| 769785 | `e4a15545-…` | 74 | 63 | 35 | ITA 34, LUX 1 | 0 | 0 | **74** | 0 |
| 333104 | `4860afa4-…` | 182 | 1 | 105 | LVA 105 | **176** | 0 | 0 | 6 |
| 769785 | `f3814684-…` | 95 | 1 | 1 | FIN 1 | **93** | 0 | 2 | 0 |

The Latvian DPS (333104) has 105 buyers under one title and declares itself a DPS on 176 of its 182 notices. The
constant-key welds have hundreds of titles and declare neither on 98–100 % of their notices. Buyers-per-version, 377's
discriminator, scores 4860afa4 and aff2863e alike. This is an exhibit of five keys, not a calibration. Sizing the
pattern is the census's job.

### Corrections to the record 377 left

- **The constant BT-04 is `aff2863e-b4cc-4e91-baba-b3b85f709117`** (777 of 789 notices), not `5e001394-…`. The other
  12 notices publish 8 other keys. They are Danish municipal procedures (Esbjerg, Fredensborg, Aalborg) joined by 8
  ADR-0011 previous-notice edges, and 6 of those edges are cited from an aff2863e notice. `5e001394` is a single
  Aalborg notice, `00084518-2024`. It wins the label because a key's rank is the earliest publication it is seen
  carrying *in an edge row* (`canonical.rs:9492`). aff2863e's earliest edge row is 2024-05-08, but its first notice is
  2023-09-12. The comment there says the representative is "the procedure's first appearance", and that is not what
  the code computes. This only affects the label; membership is unaffected. It means a census keyed on
  `tenders.procedure_key` would miss this weld. The census must key on the published BT-04.
- **The class is no longer "4 tenders, 1 convicted".** 377 read 769785 as single-country Italian works "consistent
  with legitimate systems". Read per key, it holds two Italian constant-key welds (85e08737 and e4a15545 above) plus a
  Finnish DPS. That DPS (Lohja, f3814684) supplies the Tender's current title.

### Found while verifying: a separate defect, not this issue's fix

Tender 1012301 (377's Swedish "DIS") is welded by placeholder previous-notice references, not by a key. Its 184
notices publish 166 distinct BT-04 keys, 153 titles and 63 buyer names. 166 of their OPP-090 references read
`123456-2024` (122), `00123456-2024` (3), `123456-2025` (35) or `12345-2025` (6). Each of those numbers is a real
TED publication: `00123456-2024` (subtype 16, Bulgarian, the Tender's seq 1), `00123456-2025` (subtype 17) and
`00012345-2025` (subtype 29). ADR-0011's "target must exist" guard therefore passes, and `PREV_EDGE_JOIN_SQL`
(`canonical.rs:1227`) joins all of them. The Finnish DPS entered 769785 the same way: Italian notice `00638216-2026`
(key e4a15545) cites `00003-2026`, which normalises to the Lohja notice `00000003-2026`. ADR-0011 has no guard on
the shape of the cited number. A declaration rule cannot catch this, because no single key is involved, so it needs
its own issue.

## Proposed fix

**Unit 1: census (dry, report only).**
- Read the declaration where the eventual rule will read it. `Ident::read` gains a per-notice `contracting_system`
  with four values:
  - `dps`: some `dps-usage` code is not `none`.
  - `framework`: some `framework-agreement` code is not `none`.
  - `neither`: both lists are present and every code is `none`.
  - `undeclared`: no row.

  It keys on `list_name`, not on field id, so EU eForms, SDK-DE, eForms-DE 1.x (`DE1-…`) and DÖE sdk-0.1 (`SDK01-…`)
  read the same way.
- Add a census `Spec` arm with its body in `Box::pin(async move { … }).await` (CLAUDE.md, issue 467). It streams
  notices through `Ident::read` in id-ordered chunks, as projection pass 1 does. It aggregates per published BT-04 key
  with ≥ 3 distinct `buyer_key` sets, which is 369's predicate, so a joint procurement that repeats one buyer set
  drops out. Per key it records:
  - notices and distinct buyer sets;
  - the widest single notice's buyer count;
  - buyer countries;
  - the four-way declaration split;
  - whether the key's Tender absorbed other keys through ADR-0011 (the 430681 shape).

  The report gives band counts per declaration bucket (≥ 3 / ≥ 10 / ≥ 50 buyer sets) and 30 sampled keys per bucket
  with titles, for a reader. It must not run as a `/v1/sql` loop: 377 unit 1 hit the cap at 250,000-id windows.
- Pinned by `the_contracting_system_census_splits_multi_buyer_keys_by_declaration`. A fixture DB holds three keys: a
  DPS-declaring key across many buyers, a `neither` key across many buyers, and a joint procurement naming every buyer
  in one notice. The test asserts that the first two land in their buckets and the third is not listed.

**Unit 2: decide the rule, on the census.** The candidate widens 369's pre-filter. Today 369's insert into
`plan_refused_key` (`canonical.rs:9124`; 386's FTS insert beside it is separate) is
`key_shaped = 1 … HAVING COUNT(DISTINCT buyer_key) >= 3`. Under the candidate, a key also qualifies when its notices
declare `neither`. That needs `contracting_system` as an additive `plan_notice` column, the same shape as `key_shaped`
and `buyer_key`. The share threshold comes from the census, not from this exhibit. aff2863e declares a DPS on 9
notices, so "every notice says neither" would miss it. Weigh the rule against 369's measured over-split, where refused
notices fall to islands. Pinned by `an_undeclared_multi_buyer_key_is_refused_and_a_declared_dps_key_is_not`.

**Unit 3: repair through the rule, not by hand.** The next projection retires the welded Tender through
`retire_regrouped_nonlegacy_tenders`, as 369's did. No denylist of the UUID: 377 rejected that on 2026-09-27, and it
would not catch the platform's next key. If unit 2 records "no rule", 430681 is a convicted weld all the same, and
splitting it is this unit's remaining work.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/430681 | jq -c '[.procedure_key, (.versions|length), ([.versions[]?.publication_id] | index("00550462-2023") != null)]'

- **open** (2026-10-01 13:02 UTC): `["5e001394-80da-44e1-8955-e9fe17674c90",789,true]`. This is the weld, and the
  first Swiss notice (`00550462-2023`, BT-04 aff2863e) is still inside it.
- **done** (unit 3, the last unit): the third element reads `false`. A retired Tender prints `[null,0,false]` (a
  retired id answers 404; checked on tender 1 the same day). A Danish remainder kept under the label prints a short
  count with `false`. Units 1 and 2 are recorded in the body as they land.
