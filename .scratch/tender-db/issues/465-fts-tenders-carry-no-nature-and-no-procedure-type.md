# 465 — FTS tenders carry no contract nature and FTS notices no procedure type; the profile's 20 `owed:` paths have no issue

Status: ready-for-agent — DEPLOYED 2026-10-01 at `b629d0b` (built `59668ee`, review fixes `b4f2bba`; gated in the batch). The FTS re-parse is ENQUEUED as job 1813 (`{"kind":"reparse","profiles":["fts:ocds-1.1"]}`, ~314k notices) with its trailing fold 1814. NEXT: read 1813 (expect 0 unmatched, 0 now-failing) and 1814, then the Verify (tender 8576017: `["services"]`, then `BT-23-Procedure=services`, `BT-105-Procedure=open`).
Was status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the parser: emit `mainProcurementCategory` as BT-23 (OCDS `goods` → eForms `supplies`) and `procurementMethodDetails` as BT-105-Procedure in `crates/ingest/src/fts/parse.rs`, pinned by a fixture test in `crates/ingest/tests/fts.rs`, then gate, deploy, and run one `fts:ocds-1.1` re-parse and fold after 342's last backfill chunk (jobs 1800–1809) has drained.
Kind: coverage (ingest: the FTS profile `fts:ocds-1.1`, i.e. `crates/ingest/src/fts/parse.rs` and the ADR-0004 checklist `crates/ingest/src/fts/checklist.rs`)
Relates to: 397 (contract nature as the cross-era `nature` classification; FTS is the source that never got it), 386 (unit 2b built the checklist and wrote the `owed:` list down, lines 489–506; done), 437 (the same re-parse-and-fold path, and the award-scope rule for award items), 342 (the FTS backfill the re-parse waits for), ADR-0004 (mapped-or-ignored)

## What is wrong

**The profile parses neither field.** `checklist.rs` records both as debts:
`tender.mainProcurementCategory` (:110) and `awards[].mainProcurementCategory` (:154) are
`Ignored("owed: the nature (BT-23) is not folded for FTS")`, and `tender.procurementMethod` /
`procurementMethodDetails` (:111–112) are `owed:` for BT-105. No Rust source under `crates/` other
than the checklist names either key. `struct Tender` (parse.rs:740) and `struct Award` (:843) do not
deserialize them, so serde drops them without a trace.

**The fold is already waiting for the nature.** The issue-397 pre-arm (project.rs:3265 onward) turns a
`Code` under `BT-23-*` into `Fact::Classification { scheme: "nature" }` at the scope it was published:
procedure → Tender, lot → Lot. It goes through `contract_nature` (project.rs:5242), whose BT-23 arm
accepts only `works`, `supplies` and `services`. OCDS publishes `goods` (fixtures 028961-2025,
052408-2025, 083645-2026). Passed through unchanged, `goods` would fold to nothing.

**Live, read 2026-10-01 11:59–12:05 UTC:**

| | what the source publishes | what tender-db serves |
|---|---|---|
| tender 8576017, UK4 089588-2026 "Manchester Piccadilly Strategic Regeneration Framework (SRF) Update", current notice 46697935 | public FTS API `ocdsReleasePackages/089588-2026`: `tender.mainProcurementCategory` `services`, `procurementMethod` `open`, `procurementMethodDetails` `Open procedure` | classifications: cpv `71400000` (main), cpv `71530000`, nuts `UKD33`, all on lot `1`; **no `nature`**. Notice content: no BT-23 and no BT-105 field |
| tender 7956308, UK7 028961-2025 "Route optimisation software" (a fixture, and 437's witness) | per the fixture copy of the release: `awards[0].mainProcurementCategory` `goods` on its one lot `1`, and **no** tender-level category | classifications: cpv `48000000`, nuts `UK`; **no `nature`** |
| TED eForms tender 2, notice 23555356, for comparison | | `nature` `services` at procedure and at `LOT-0001`. Notice content: `BT-23-Lot=services`, `BT-105-Procedure=neg-w-call` |

Some award releases publish the nature only on the award: 028961-2025 (UK7), 029664-2025 (UK6, on all
five of its awards, each naming lot `1`) and 083650-2026 (UK6) carry no tender-level category, the shape
437 found for CPV and region. Others (029615-2025, 052408-2025, 083468-2026) publish it on the tender.
So the award path is needed as well as the tender path.

**Procedure type is a notice-layer field in every source.** The tender response has the same 27 keys
for 8576017, 7956308 and TED tender 2, and none of them is a procedure type (`procedure_key` is the
procedure's identity). A jq read of `.procedure_type` prints `null` because the key is missing, not
because FTS left it empty. BT-105 lives only in `/v1/notices/{id}/content`. eForms notices carry it
there; FTS notices do not. A Tender-level procedure type for all sources is a separate model question:
the 2026-09-15 API/DQ review lists BT-105 among the eForms terms with "no place on the Tender"
(`.scratch/tender-db/api-dq-review-2026-09-15.md`, the dq-eforms row, marked "not recorded"). It is
out of scope here.

**No issue owns the rest.** `grep -c 'Ignored("owed'` on `checklist.rs` gives 20 entries. 386 listed
them ("Reclassifying one of these to `Mapped` is how the next unit records itself"), but 386 is done. A
grep of the board for `mainProcurementCategory`, `procurementMethod`, `BT-105` and `owed:` hits only
closed issues, plus two open ones (443, 448) whose hits are unrelated prose ("Still owed:",
"followed:", "allowed:"). Issue 342 (the FTS backfill) names none of these paths.

## The owed entries, and which are worth mapping

The eForms terms are from `crates/ingest/sdk/fields-1.15.0.json`. "In fixtures" counts the 10 member
releases under `crates/ingest/tests/fixtures/fts/members/` and the two pages.

| checklist paths (line) | eForms term | where it would land | in fixtures | verdict |
|---|---|---|---|---|
| `tender.mainProcurementCategory` (110), `awards[].mainProcurementCategory` (154) | BT-23 Main Nature, list `contract-nature` | the served `nature` classification (397) | 8 of 10 members, 3 of them award-only | **map now** (this issue) |
| `tender.procurementMethod`, `tender.procurementMethodDetails` (111–112) | BT-105-Procedure, list `procurement-procedure-type` | notice layer; no source serves it on the Tender | details in 7 of 10 members | **map now** (this issue): cheap, gives parity with eForms content, and a later Tender field then reads FTS like the rest |
| `tender.procurementMethodRationale` (113), `…RationaleClassifications` (114) | BT-135 (text), BT-136 (code, list `direct-award-justification`) | notice layer only | page p002 only; its codes are TED R2 (`TED_PT_AWARD_CONTRACT_WITHOUT_CALL` / `D_OUTSIDE_SCOPE`), which r209 keeps as markers (`r209/rules.rs:529`), and nothing maps them to BT-136 | later. The text is a one-line BT-135. The code needs a translation table and has no consumer |
| `tender.lots[].hasOptions` / `options` (94–95), `awards[].hasOptions` / `options` (141–142) | BT-54-Lot Options Description | notice layer only; the fold reads BT-54 for no source | 052408-2025, p002 | later, with a model field for all sources |
| `tender.lots[].hasRenewal` (96), `contracts[].hasRenewal` (178) | BT-57-Lot / BT-58-Lot (renewal description / maximum) | notice layer only; eForms content carries both (notice 23555356) and the fold reads neither | p001, `_noid` member | later, with options. A boolean fits neither term exactly |
| `…contractPeriod.maxExtentDate` on lots (90), awards (139), contracts (172) | none named | no destination: 386 unit 2b's schema question (`tender_version_contracts` has no duration columns) | p001, `_noid` member | leave owed until that schema question is decided |
| `tender.awardPeriod` (107) | none named | no destination | 083563-2026 | leave owed |
| `parties[].address.streetAddress` / `postalCode` / `locality` (59–61) | BT-510 / BT-512 / BT-513 | notice layer only. eForms content carries them (notice 23555356); `/v1/organizations/{id}` serves none (keys: country, id, identifier, identifier_kind, identifier_status, mentions, name, provisional) | all 10 members | later, and only when the resolver or an organization field reads them |
| `parties[].contactPoint` (62) | BT-502 / BT-503 / BT-506 | notice layer only | all 10 members (`email`, and in most a `name` and/or `telephone`) | do not map without a consumer: it is a contact person's name, e-mail and phone with no reader |

## Proposed fix

1. **Parser** (`crates/ingest/src/fts/parse.rs`). Deserialize `mainProcurementCategory` on `Tender`
   and `Award`, and `procurementMethod` / `procurementMethodDetails` on `Tender`. Translate at the
   profile boundary, the way `awards[].status` already becomes BT-142 codes (parse.rs ~:273–284). The
   fold and `contract_nature` stay in one vocabulary, eForms', and `contract_nature` does not learn
   `goods`.
   - **Nature.** `goods` → `supplies`, `works` → `works`, `services` → `services`, anything else emits
     nothing. Emit with list `contract-nature`. The tender's goes on ROOT as `BT-23-Procedure`. An
     award's goes as `BT-23-Lot` on the one lot it names, and as `BT-23-Procedure` when the award names
     several lots or none. It is not stated again when the release already stated that code at that
     scope, which is 437's rule for award items.
   - **Procedure type.** Emit `BT-105-Procedure` (list `procurement-procedure-type`) from
     `procurementMethodDetails` through a closed table of the published strings. These have an eForms
     code: `Open procedure` → `open`, `Negotiated procedure with prior call for competition` →
     `neg-w-call`, `Award procedure without prior publication of a call for competition` →
     `neg-wo-call`. These have none: `Below threshold - open competition` / `- limited competition` /
     `- without competition` / `- unknown`, `Award under framework`, and the Procurement Act's
     `Competitive flexible procedure` (uk-fts.md §4). For those the unit decides between emitting
     nothing (the profile's rule for `awards[].status` and the Procurement Act bid measures: "unmapped
     rather than guessed") and `oth-single` / `oth-mult`, and records the decision in the checklist
     entry. Do not derive the code from `procurementMethod` alone: OCDS `selective` covers restricted,
     negotiated-with-call and competitive dialogue. An unknown string emits nothing, so a string the
     table misses costs coverage, never a wrong code.
2. **Checklist.** Reclassify the four paths (110, 111, 112, 154) to `Mapped(...)` naming the field ids
   above. `every_published_fts_path_is_mapped_or_ignored_on_record` keeps the census honest. The other
   16 entries stay `owed:` with the verdicts above.
3. **Tests.**
   - In `crates/ingest/tests/fts.rs`, a test named for the behaviour, e.g.
     `an_fts_release_folds_its_category_as_the_contract_nature`. Fixture 052408-2025 (tender `goods`)
     → Tender nature `supplies`. 028961-2025 (award-only `goods`, lot `1`) → lot `1` nature `supplies`.
     029615-2025 (`services`) → `services`.
   - A parse-level test of the BT-105 table in parse.rs's tests: `Open procedure` → `open`,
     `Negotiated procedure with prior call for competition` → `neg-w-call`, an unknown string → nothing.
4. **Rollout.** Gate (`ops/check.sh`), then deploy on a hand-read idle queue (deploy.sh's own busy
   probe fails open, issue 459). Then run ONE `reparse` of `fts:ocds-1.1` and its fold, the pair 386
   ran as jobs 1585 / 1587. Run it after 342's last chunk drains: at 12:04 UTC on 2026-10-01 job 1801
   (`fts monthly 2021-02`) was running, with 1802–1808 (fetches) and 1809 (`project rebuild=false`)
   queued. That way one re-parse covers every FTS notice. No organization fact changes, so 434's
   mention refresh is not needed.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/8576017 https://tenders.zebreus.click/v1/notices/46697935/content | jq -c '[(.classifications // [])[] | select(.scheme == "nature") | .code] + [(.sections // [])[].values[] | select(.field_id | test("^BT-(23|105)-")) | "\(.field_id)=\(.code)"]'

The first line is tender 8576017's `nature` codes. The second is the BT-23 / BT-105 values of its
current notice (089588-2026).

- **open** (2026-10-01 12:02 UTC): `[]` then `[]`. No nature on the tender, and neither field in the
  notice.
- **done**: `["services"]` then `BT-23-Procedure=services` and `BT-105-Procedure=open`, in either
  order. That is what the release publishes. This reads the LAST unit, the re-parse and fold: a deploy
  alone leaves both lines empty, because the notice was parsed before it (unless a newer notice for
  this procedure lands first, which moves only the first line). The same command on TED tender 2 and
  notice 23555356 prints `["services","services"]` /
  `["BT-23-Lot=services","BT-105-Procedure=neg-w-call","BT-23-Lot=services"]`, which is the shape done
  takes.

## 2026-10-01 15:2x UTC — deployed; re-parse enqueued

- Parser: `mainProcurementCategory` maps to BT-23 nature (`goods` → `supplies`), at the procedure, or at the one lot
  an award names. `procurementMethodDetails` maps to BT-105 through a closed table of published labels. The act's
  `Open procedure` is `open`; `Competitive flexible procedure` and `Direct award` have no eForms code and emit
  nothing. 16 `owed:` checklist entries remain.
- Review fix (`b4f2bba`): the data-quality drop sieve now reads the nature (`NATURE_STEMS`), so no era's nature
  reads as dropped (an issue-397 gap).
- Prod: job 1813 reparses every `fts:ocds-1.1` notice, then project 1814.
