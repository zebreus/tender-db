# 470 — a letter O typed for a zero (or a zero for the O of `OC`) splits a GB company number from its organization, and no arm joins them

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The measurement 454 asked for is below (taken 2026-10-01, through FTS chunk 8), so the first unit is the decision it feeds: which shapes fold, at which tier and behind which name gate, recorded here with its reasoning before any code.
Kind: data quality (identifiers)
Relates to: 454 (its option 2, never filed), 453 (the re-key arm, which fixed the 13 `GBCOH` exhibits), 452 (the census; its
shapes left out the bare spellings), 362 (merge verdicts for groups R2's name gate holds), 466 (the census re-run; same
listing walk), 460 (identifier lookups after a merge), 300 (the E1/E2 rule)

## What is wrong

### The GB arm reads the register format literally

The GB arm of `canonical_key` (`crates/ingest/src/crosswalk.rs:365–392`) keys a company number only when it is eight
digits or two letters plus six digits (`letters_then_six`, :383). A lookalike gets no key, or a key of its own:

| literal | key today | the number meant |
|---|---|---|
| `SCO55775` | none | `SC055775` |
| `O6611251` | none | `06611251` |
| `0C301540` | none | `OC301540` |
| `OO688424` | E1 `GB:coh OO688424` (it passes as two letters) | `00688424` |

Without a shared key, nothing joins the typo to the correct spelling:
- The resolver binds a mention by its exact triple or by an E1 key (`resolver_canon_key`,
  `crates/store/src/canonical.rs:6176`). So a new lookalike spelling mints a new org.
- R2 groups orgs by the same key (`match_org_identifiers_r2`, :11919). So it never sees the pair.

### 454's exhibits are fixed, but the census never looked at the bare spellings

The 13 lookalike orgs that 452 posted as `wrong` with a right number are gone, re-keyed by 453 (`GBCOHOO688424`,
`GBCOHO2O84294`, `GBCOHSCO28565`, and others). All 13 return no items on `?identifier=` (2026-10-01), and all 13 were
`GBCOH…` spellings.

452's census took only three shapes: `GBCOH…`, 8 digits, and two letters plus six digits (452, "sized"). A bare
`SCO55775`, `FCO35527` or `O6611251` matches none of them, and no 452 verdict names one. So this is not only prevention:
split pairs stand today.

### Measured 2026-10-01

Method. No `/v1/sql` and no data pages:
- **Ids ≤ 31,590,759.** 452's census input `/root/gb-orgs.jsonl`, read with `cat` over ssh. It has 58,938 rows: every GB
  `national` org as of 2026-09-30 09:50 UTC.
- **Ids above that.** The public listing `/v1/organizations?country=GB&kind=national&limit=1000&cursor=31590759`. That is
  9 pages and 8,057 orgs, up to id 31,634,553. Chunk 9's project 1809 is still queued.
- **The test.** Each identifier is normalised the way `canonical_key` does it: alphanumerics only, upper case, PPON
  skipped, `GBCOH`/`COH`/`GB` stripped. It is flagged when an 8-character body has a letter O at positions 3–8, an O
  next to a digit or another O at positions 1–2, or a digit 0 next to a letter at positions 1–2.
- **Live check.** Each hit, and each standing org under its folded number, was read live through `?identifier=` and
  `/v1/organizations/{id}` (12:2x–12:3x UTC).

The sample contains the thing being measured: the scan finds all 13 exhibits that 453 fixed.

Re-take it with the same two reads. Once project 1809 lands, also read the listing from `cursor=31634553`.

Of 66,995 GB `national` orgs:

| shape | standing | with a twin (a standing org under the folded number) | gone since 2026-09-30 (453) |
|---|---|---|---|
| letter O where a digit belongs (`SCO55775`, `FCO35527`, `NIO41488`, `O6611251`) | 25 | 13 | 11 |
| digit 0 where the `OC` prefix has its O (`0C301540`) | 5 | 3 | 2 |

- 8 of the 30 standing lookalike orgs sit above the census watermark. The FTS backfill keeps minting them.
- Two of the 25 end in an O (`9694399O`, `9979037O`). No standing org carries their fold, and this measurement cannot
  show that they are lookalikes at all.
- The 0-next-to-a-letter test also flags `I0097973` and `GBCOHN0790518` (452's challenger reads the second as 07905187's
  digits behind a stray N), plus six `R0…` rows (see below). None of these is an `OC` lookalike, and none is counted.

The 16 pairs (org id, mentions in brackets):

| lookalike | twin | names |
|---|---|---|
| `SCO55775` 10312649 (9) | `SC055775` 10312664 (3) | Galliford Try Infrastructure Ltd, both |
| `SCO41252` 14857251 (1) | `SC041252` 7616138 (31) | Caledonian Modular Ltd, both |
| `SCO46129` 12802561 (4) | `SC046129` 18043531 (15) | Morris and Spottiswood / Morris & Spottiswood Ltd |
| `SCO79486` 22729025 (1) | `SC079486` 14115827 (4) | Emtelle UK / Emtelle UK Ltd |
| `SCO93579` 12861132 (3) | `SC093579` 12861122 (2) | Hypostyle Designs Limited t/a Hypostyle Architects, both |
| `GBCOHSCO53003` 31593022 (1) | `SC053003` 16243927 (9) | Farid Hillend Engineering Ltd, both |
| `FCO35527` 23405148 (1) | `FC035527` 16866599 (5) | Sony Europe B.V / Sony Europe BV |
| `GBCOHFCO12665` 31615837 (1) | `GBCOHFC012665` 31598423 (1) | McKinsey & Company, Inc United Kingdom, both (case aside) |
| `NIO41488` 23693680 (1) | `NI041488` 23747306 (3) | Bayview Contracts Limited, both |
| `O6611251` 15336604 (1) | `06611251` 16866854 (1) | Roythornes Ltd / Roythornes Solicitors Ltd |
| `GBCOHO7687679` 31607363 (1) | `GBCOH07687679` 30919575 (9) | Aim2Learn / AIM 2 LEARN LTD |
| `GBCOHIPO30808` 31536131 (1, withheld) | `GBCOHIP030808` 31534918 (9) | Co-Op Funeral Care / Funeral Services Limited T/A Co-op Funeralcare |
| `GBCOH0C301540` 31612159 (1) | `OC301540` 10695280 (192) | KPMG LLP, both |
| `GBCOH0C305934` 31597793 (1) | `OC305934` 18614141 (25) | Knight Frank LLP / Knight Frank |
| `GBCOH0C429964` 31602487 (1) | `GBCOHOC429964` 31602449 (1) | Shakespeare Clinic / Shakespeare Clinic LLP |
| `SCO13683` 9725105 (1) | `GBCOHSC013683` 31633948 (1) | University of Aberdeen (via Centre for Energy Law) / **Net Zero Technology Centre Limited** |

15 of the 16 pairs name one entity. The register confirms the twin's number for 8 of them (448's cache, or the public
page on 2026-10-01):

| number | register name |
|---|---|
| `SC055775` | GALLIFORD TRY INFRASTRUCTURE LIMITED |
| `SC041252` | REALISATIONS (CM) LIMITED, named CALEDONIAN MODULAR LIMITED 2013–2022 |
| `SC046129` | MORRIS & SPOTTISWOOD LIMITED |
| `06611251` | ROYTHORNES HOLDINGS LIMITED, named ROYTHORNES LIMITED 2013–2025 |
| `07687679` | AIM 2 LEARN LTD |
| `IP030808` | FUNERAL SERVICES LIMITED |
| `OC301540` | KPMG LLP |
| `OC305934` | KNIGHT FRANK LLP |

The 16th pair is not one entity:
- `SCO13683` is the University of Aberdeen's Scottish charity number (OSCR `SC013683`) typed with an O.
- Companies House has no `SC013683` (404, 2026-10-01).
- The twin, org 31633948, is Net Zero Technology Centre Limited, a different body that carries the same number.
- R2's name gate (`name_cores_disjoint`, canonical.rs:26679) denies a pair only when the two names share no core token.
  These two share `centre`, so a key-only fold would let R2 merge them.

### The `R0` series keys nothing, and a fold must not touch it

Six orgs carry `R0` followed by six digits. On the register (2026-10-01), `R0000568` is NORTHERN BANK LIMITED and
`R0000273` is H.& J. MARTIN LIMITED. `letters_then_six` rejects that shape, so two numbers stand as two orgs each:

| number | org with `GBCOH` prefix | org with bare number |
|---|---|---|
| `R0000273` | 30914553 | 16866909 |
| `R0000524` | 31573122 | 21985296 |

A fold that reads the `0` in `R0` as an O would turn real numbers into wrong ones.

### Three withheld lookalikes have a known right number that nothing reads

452 posted three lookalikes as `wrong`/high with no `correct_identifier`. In each rationale, the challenger named the
right number:

| literal | org | right number |
|---|---|---|
| `GBCOHCEO19319` | 31574614 | CE019319 |
| `GBCOHIPO30808` | 31536131 | IP030808 |
| `GBCOHNIO18750` | 31581165 | NI018750 |

All three are withheld and serve `register_mismatch`. R2 skips withheld orgs (canonical.rs:11962), and the rekey arm
reads only verdicts that have a `correct_identifier`. So neither a fold nor the existing arms reach them.

### 454's "`org_match_keys` epoch bump" does not apply

`org_match_keys` holds name keys only (`key_kind IN ('n2','n3','n3s')`, canonical.rs:697). The identifier key is
computed again on every R2 run and on every fold, which builds the resolver. A change to the GB arm therefore acts at the
next fold and the next R2 run, with nothing to rebuild.

The altid and rekey arms key through the same function (`canonical_key_flat`: canonical.rs:2431 and :3160, and
`mention_key` → `altid_pair_key` in crosswalk.rs:1003–1035). Their next dry plans can move too.

## Proposed fix

The root cause: the arm treats a format violation as "not a company number". For an O/0 lookalike, though, the format
itself says which character was meant. The fold belongs in the arm, behind a name gate, because one pair in sixteen is
two different bodies.

1. **Decide (first unit), and record the decision here.** The table argues for a fold: 16 standing pairs, 15 of them one
   entity, and new ones still being minted. Recommended shape, for the decider to accept or reverse:
   - Fold O→0 at positions 3–8. Fold it at positions 1–2 when the other character there is a digit or another O.
   - Fold 0→O only for a `0C` head, giving `OC`, the LLP prefix. All five observed are LLPs by name.
   - Never fold `R0`. In the same change, key `R0` plus six digits as E1, which joins the two pairs above.
   - Mark a folded key as folded (a flag on `CanonKey`, carried through `canonical_key_flat`) and gate it by name
     wherever it binds or merges:
     - **In R2:** a folded member merges only when its names agree with the group's under `altid_keys_agree`, the gate
       the rekey arm already uses. A denial goes to R2's review listing for a 362 merge verdict. Aim2Learn / AIM 2 LEARN
       LTD is 454's spacing shape and will likely need one.
     - **In the resolver:** a mention whose literal needed the fold binds through 452's guarded path, that is, only to
       an owner whose names its names match. It never binds through `canon_of` on the key alone.

     That stops the Aberdeen pair and keeps the prevention.
2. **The survivor.** R2 keeps `min_by_key(|m| (m.provisional, m.id))` (canonical.rs:12305 in the dry run, :12445 in the
   wet run). In 4 of the 16 pairs the lookalike has the lower id: `SCO55775` 10312649, `SCO46129` 12802561, `NIO41488`
   23693680 and `O6611251` 15336604. The merged org would then serve the typo as its `identifier`, and while 460 is open
   `?identifier=SC055775` would find nothing. Rank by `(provisional, folded, id)` instead.
3. **Tests.**
   - `gb_coh_folds_an_o_where_the_register_format_has_a_digit` (crosswalk.rs tests):
     - `SCO55775`, `O6611251`, `OO688424` and `0C301540` key to their numbers, marked folded;
     - `SC055775` and `OC301540` key unfolded;
     - `R0000568` keys as itself, never as `RO…`.
   - `a_fold_joined_r2_group_merges_agreeing_names_into_the_unfolded_literal` (`crates/store/tests/r2_merge.rs`):
     - a Galliford-shaped pair merges into the `SC055775` org, although the typo org has the lower id;
     - an Aberdeen/Net-Zero-shaped pair stays apart and is listed.
   - A resolver test beside 452's guarded-key tests (`crates/store/tests/identifier_verdicts.rs`):
     - a new `GBCOHSCO55775` mention named Galliford Try binds to the `SC055775` org;
     - the same literal under another name mints a new org.
4. **Roll out.**
   - Run the gate (`ops/check.sh`), then deploy.
   - Dry-run R2 (`{"kind":"match-org-identifiers","rule":"r2"}`, which is dry by default).
   - Check its fold-joined groups against the 16 pairs and the two `R0` pairs, and read the altid and rekey dry plans
     for movement.
   - Run R2 wet against the dry plan, then the Verify.
5. **The three withheld lookalikes.** Re-post their verdicts with the challenger's numbers under a new cohort, as 453
   corrected Cochlear and Montel. The rekey arm then acts on them:
   - `IP030808` has a standing org (31534918) to merge into;
   - `CE019319` and `NI018750` have none (looked up 2026-10-01), so they move in place.

## Verify

    curl -s https://tenders.zebreus.click/v1/organizations/10312649 | jq -c '[.id, .identifier, .merged_into]'

- **open** (2026-10-01 12:3x UTC): `[10312649,"SCO55775",null]`. Galliford Try Infrastructure Ltd stands under the typo,
  beside org 10312664 (`SC055775`, the register's GALLIFORD TRY INFRASTRUCTURE LIMITED).
- **done:** `[null,null,10312664]`, meaning it was merged into the org that carries the register's spelling.
  - The fold landed but the survivor rule did not: the line still reads `[10312649,"SCO55775",null]` and org 10312664
    answers `merged_into` 10312649.
  - The decision is not to fold: the closure goes on line 3 with its reason, and this line stays open by design.
