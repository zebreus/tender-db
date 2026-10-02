# 470 — a letter O typed for a zero (or a zero for the O of `OC`) splits a GB company number from its organization, and no arm joins them

Status: ready-for-agent — UNIT 2 (the code) COMMITTED 2026-10-02 on top of a5c7666. Gate GATE-EXIT=0 (879 s), run before the commit on a dirty tree, so no gate marker was written; re-gate at the shipped rev. NOT pushed, NOT deployed (prod is busy on a5c7666). NEXT: deploy it in a queue gap, then the rollout reads (step 4) and the verdict re-post (step 5), written out step by step under "Unit 2 (2026-10-02)" → "NEXT". No job has run on prod for this issue.
Was status: ready-for-agent — DECIDED 2026-10-02 (owner; decision below, under "Decision"). The proposed fold is accepted with three refinements. NEXT: unit 2, the code (crosswalk GB arm + R2 survivor + the resolver's guarded bind + tests), then the rollout in step 4. Sequence it after 481 unit 2 lands, because both edit crates/store/src/canonical.rs and one gate at a time fits the container's disk.
Was status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The measurement 454 asked for is below (taken 2026-10-01, through FTS chunk 8), so the first unit is the decision it feeds: which shapes fold, at which tier and behind which name gate, recorded here with its reasoning before any code.
Kind: data quality (identifiers)
Relates to: 454 (its option 2, never filed), 453 (the re-key arm, which fixed the 13 `GBCOH` exhibits), 452 (the census; its
shapes left out the bare spellings), 362 (merge verdicts for groups R2's name gate holds), 466 (the census re-run; same
listing walk), 460 (identifier lookups after a merge), 300 (the E1/E2 rule)

## What is wrong

### The GB arm reads the register format literally

The GB arm of `canonical_key` (`crates/ingest/src/crosswalk.rs:365–395`) keys a company number only when it is eight
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
   wet run). In 4 of the 15 one-entity pairs the lookalike has the lower id: `SCO55775` 10312649, `SCO46129` 12802561,
   `NIO41488` 23693680 and `O6611251` 15336604 (the Aberdeen pair's `SCO13683` 9725105 is lower too, but the gate keeps
   that pair apart). The merged org would then serve the typo as its `identifier`, and while 460 is open
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

## Decision (owner, 2026-10-02)

Accepted: the proposed fold, its name gate, the survivor rank `(provisional, folded, id)`, the `R0` E1 key and the
re-posted verdicts for the three withheld lookalikes. Reasons: 15 of 16 standing pairs name one entity, 8 are confirmed
by the register, and the backfill keeps minting new ones. The 16th (Aberdeen / Net Zero) is exactly what the name gate
is for, so a key-only fold is refused.

Refinements:
1. **Real prefixes that contain an O are never folded.** `OC` (LLP), `SO` (Scottish LLP) and `OE` (overseas entity)
   have a letter beside their O, so the "other character is a digit or another O" condition already spares them. Pin
   that in the crosswalk test: `SO300123`, `OE012345` and `OC301540` key unfolded and unflagged.
2. **A fold never mints a key a standing org must then match by key alone.** A folded key that no unfolded org
   carries, such as `9694399O` → `96943990` with no standing twin, binds nothing new. That holds because every bind
   and merge of a folded key goes through the name gate. The test names that case, so the trailing-O rotations stay
   inert rather than becoming wrong numbers.
3. **`OO688424` loses its E1 key today and gains a folded one.** That is a re-key of any org currently keyed `OO…`.
   The rollout's R2 dry plan must list those orgs (expected: the 453-era exhibits are gone, so a handful at most), and
   the altid and rekey dry plans are read for movement before any wet run, per step 4.

## Unit 2 (2026-10-02) — the fold, its flag, and every gate that reads it

Steps 1–3 of "Proposed fix" as decided, with the three refinements. Line numbers are at this commit.

### What landed

- **The crosswalk** (`crates/ingest/src/crosswalk.rs`). `gb_coh_fold` (:451) folds an 8-character body: O→0 at positions
  3–8; an O at positions 1–2 only when the other character there is a digit or another O; a 0 becomes an O only in a
  `0C` head. `R0…` is never touched, and `R` + 7 digits now keys E1 as itself (the GB arm, :404 on). A folded key carries
  `CanonKey::folded = true`. `canonical_key_flat` (:841) returns `(scheme, key, is_e1, folded)`, and so do `e0_key_flat`,
  `mention_key` and `altid_pair_key`. The flat tuple grew a field on purpose: the compiler then names every consumer,
  and each one now says what it does with a folded key (below). Pads (6–7 characters) are never folded.
- **R2** (`crates/store/src/canonical.rs`, `match_org_identifiers_r2`). New rule 3b, the fold gate (:14734). It runs after
  the consortium veto and before the legal-form veto, so a refused lookalike neither merges nor vetoes the rest of its
  group. A folded member stays only when one of its names (head or satellite) agrees with one of its group's under
  `altid_keys_agree(altid_name_key(a), altid_name_key(b))`. "The group's" means the unfolded members, or the survivor when
  every member is folded. The gate is member-scoped: a remainder below two is `denied_fold`. An excluded group is listed
  whole in `denied_fold_listing`, R2's review queue for a 362 verdict (same shape and round trip as
  `denied_names_listing`). A HIGH merge verdict on exactly the live member set stands in for the gate, as it does for
  the name rule. The survivor is `(provisional, folded, id)` in the dry plan (:15069) and the wet merge (:15230) alike.
  `R2MergeArgs` gained `name_key` and `names_agree`; the supervisor passes the altid pair.
- **R2's dry plan** also records `keyed_folded` and `fold_listing` (:15180). The listing names every key a row reaches
  through the fold, with every row on that key, singletons included. That makes the decision's refinement 3 visible: a
  row still carrying `OO688424` appears as key `00688424` with that one row. These fields are in the stored
  `r2-merge-plan` report and in the job's summary line.
- **The resolver** (`mention_resolver` / `resolve_one_mention`). `resolver_canon_key` returns the flag. At open, a folded
  row goes into `MentionResolver::folded` and never into `canon_of` (:12145). So a key that only folded rows carry binds
  nothing new on the key alone (refinement 2). A mention whose own literal needed the fold never takes the `canon_of`
  hit. It goes through 452's guarded path (:12916): the owners are the key's `canon_of` owner plus its folded rows, and
  the mention binds to the ONE owner whose names match under `norm`. The bind is never cached. A poisoned key binds it
  to nobody. A folded mint claims nothing and joins the folded rows (:13297). A withheld key's guarded owners now
  include its folded rows. Counted in the fold's diag lines as `[issue 470] folded keys: N bound by name, M minted`.
- **453's re-key alias at open** (:12207). A lookalike re-keyed onto the number its own fold proposes guards nothing.
  Guarding the right number would put the entity's own spellings behind a name match.
- **The buyer guard tokens** (481 unit 2b, `crates/ingest/src/project.rs` `buyer_guard_tokens`). A folded key is not
  used: a lookalike buyer keeps its raw-literal token, as every lookalike did before the fold.
- **R3** and the r3-census keep folded rows out of the target map (canonical.rs:17599, supervisor's census beside
  it). R3 corroborates against ONE owner, and a lookalike row is not a register owner. GB has no checksum anchor today,
  so this changes no current output.

### The altid and rekey arms' own name gates still guard a folded key (verified)

- **altid** (`match_org_altid_pairs`, :18096).
  - A folded published company number pairs as `Side::E1` (:18230). A folded row owns its key in the owner map (:18368).
  - Beside the register spelling's row, the folded row makes the key `multi_target` (:18517–18520, refused).
  - Alone, the folded row is a target, but the PPON org folds into it only past step 6: `altid_corroborates` on
    witness-free names (:18799), the generic wall, or a HIGH reviewer verdict on the exact pair (:18742). Only then does
    `report.plan_pairs += 1` run (:18897).
  - The alias half (`altid_alias_bind`) reads `canon_of` only, and a folded row is never in it.
- **rekey** (`match_org_rekey`, :20346).
  - A folded row owns the right number in the owner walk (:20470). With the register spelling's row beside it, the
    candidate is `multi-target` (:20639).
  - Alone, a folded row is a merge target, so the consortium, legal-family and names tests apply: `names_agree`
    over `name_key`, head and satellites (:20613; `names` denial :20620; HIGH 454 verdict `admits` :20615).
  - A folded owner is "found", so it never becomes a move (the move arm is the `[]` case, :20552).
- **Two rekey refinements were needed for step 5. Without them the three re-posted verdicts would never act:**
  - `same_key` compares only an unfolded wrong key (:20422). `GBCOHIPO30808` now folds to `IP030808`, the right number
    itself. That is what the verdict corrects, not a no-op.
  - The destination flag set skips folded keys (:20504). Otherwise the verdict on `GBCOHCEO19319` would flag `CE019319`
    and refuse its own move.
  - 460's spelling test treats a flagged literal the same way (`flagged_spelling_key`): a lookalike `wrong` drops only
    its own literal, never spellings of the right number its fold proposes.
  - Pinned by `rekey.rs::a_lookalike_whose_fold_is_the_right_number_is_re_keyed_onto_it` (:578): one move, one merge.

### Tests

- `crosswalk.rs::gb_coh_folds_an_o_where_the_register_format_has_a_digit` (:757).
  - `SCO55775`, `O6611251`, `OO688424`, `O2O84294` and `0C301540` fold, flagged. The prefixed forms fold the same.
  - `SC055775`, `OC301540`, `SO300123`, `OE012345` and `06611251` key unfolded and unflagged.
  - `R0000568` keys as itself. `I0097973` and `N0790518` key nothing.
  - `9694399O` gets a flagged `96943990`. A fold that misses the register shape keys nothing, and no pad folds.
- `r2_merge.rs::a_fold_joined_r2_group_merges_agreeing_names_into_the_unfolded_literal` (:403).
  - The Galliford-shaped pair (typo id 1, register spelling id 2) merges INTO 2.
  - The Aberdeen/Net-Zero pair stays apart and is listed whole. A HIGH merge verdict on `[3, 4]` then admits it.
  - The `R0` `GBCOH`/bare pair merges. A `0C`/`OC` pair merges into the `OC` row.
  - The lone `OO688424` row is in `fold_listing` as `00688424`.
- `identifier_verdicts.rs::a_folded_mention_binds_only_to_the_owner_whose_name_matches` (:345).
  - `GBCOHSCO55775` named Galliford Try binds to the `SC055775` org.
  - The same literal under another name mints, and the mint claims nothing (the next Galliford spelling still finds
    org 1).
  - `96943990` published as itself mints rather than binding to the `9694399O` org.

### Expected on the first dry plan (read against it, not taken on trust)

Applying `altid_keys_agree` to the measured names (the "16 pairs" table):

| expected | pairs |
|---|---|
| merge | Galliford, Caledonian Modular, Morris & Spottiswood (`&` → `and`, which the key drops), Emtelle (one-sided `Ltd`), Hypostyle, Farid Hillend, McKinsey (case), Bayview, KPMG, Knight Frank (one-sided `LLP`), Shakespeare Clinic, the two `R0` pairs (unfolded, so the R2 name rule decides) |
| listed in `denied_fold_listing`, needs a 362 merge verdict | Aim2Learn / AIM 2 LEARN LTD (454's spacing shape, as forecast), Roythornes Ltd / Roythornes Solicitors Ltd, Aberdeen / Net Zero (keep: two bodies) |
| either way | Sony Europe B.V / Sony Europe BV — depends on how `n3_key` reads `B.V` |
| absent from R2 | the Co-op pair: `GBCOHIPO30808` (31536131) is withheld, so step 5 covers it |

Survivor caveat: the rank puts `provisional` first, as decided. A pair whose register-spelling row is provisional and
whose typo row is not keeps the typo. The dry plan's `plan` listing shows each keep. Check it per pair.

### Deviations

- The fold reads 8-character bodies only. "Positions 3–8" is defined only on the full form, and a pad is E2 anyway.
- The fold gate's denials get their own listing (`denied_fold_listing`) beside `denied_names_listing`. They are not
  mixed in, because the tuple carries no reason field and a reviewer must know which rule refused.
- The buyer guard, R3 and 460's flagged-spelling test also needed decisions the issue did not name; each is above.

### NEXT (owner, in order; nothing here has run)

1. **Deploy.** Gate at the shipped rev (`ops/check.sh`, read `GATE-EXIT=`), push by explicit ref, deploy in a queue
   gap. The resolver half acts from the next fold on: folded mentions bind by name only. Watch the fold's diag line
   `[issue 470] folded keys: N bound by name, M minted` and the open line `… carried through the GB O/0 fold by N
   row(s)`.
2. **R2 dry.** `POST /admin/jobs {"kind":"match-org-identifiers","rule":"r2"}` (dry by default), then
   `GET /admin/reports/r2-merge-plan`. Read:
   - `keyed_folded` (≈30 standing lookalikes, fewer the withheld ones);
   - `fold_listing` against the 16 pairs: the keys `SC055775 SC041252 SC046129 SC079486 SC093579 SC053003 FC035527
     FC012665 NI041488 06611251 07687679 OC301540 OC305934 OC429964 SC013683`, plus the two trailing-O singletons
     (`96943990`, `99790370`), plus any row still carrying an `OO…` literal (refinement 3: expected none or a handful);
   - `denied_fold_listing` against the table above;
   - `plan` for the two `R0` pairs (`R0000273` 30914553/16866909, `R0000524` 31573122/21985296) and each fold-joined
     group's keep (the register spelling, unless the provisional caveat applies);
   - `plan_groups` against `GET /admin/reports/r2-merge-plan/previous`. Anything beyond the fold-joined groups and
     the R0 pairs is movement to explain before the wet run.
3. **Movement in the other arms.** Run `{"kind":"match-org-identifiers","rule":"altid"}` and `{"kind":…,"rule":"rekey"}`
   dry. Diff each report against its `/previous`: altid `pairs`, `multi_target`, `no_target_*`; rekey `keys`,
   `same_key`, `destination_verdict`, `multi_target`. Expected from the code: a few altid pairs move into
   `multi_target` where a lookalike and its twin both stand (refused until R2 merges them); rekey changes nothing
   before step 5.
4. **R2 wet** against the reviewed dry plan: `{"kind":"match-org-identifiers","rule":"r2","dry_run":false}`. Then run
   the Verify below (`[null,null,10312664]`).
5. **362 merge verdicts** for the listed pairs the register confirms. `POST /admin/merge-verdicts` with `country`
   `GB`, `scheme` `GB:coh`, `key` and `members` exactly as listed, `action` `merge`, `confidence` `high`:
   - Roythornes `06611251`: the register name 2013–2025 is ROYTHORNES LIMITED;
   - Aim2Learn `07687679`: the register is AIM 2 LEARN LTD.

   Post `keep` for Aberdeen/Net Zero `SC013683` so it leaves the queue. Then a dry plan, and a wet run with the new
   plan.
6. **Step 5, the three withheld lookalikes.** `POST /admin/identifier-verdicts`, cohort `470-lookalikes-2026-10-02`.
   Each verdict is `wrong`, confidence `high`, with the challenger's number as `correct_identifier`:
   - org 31574614 `GBCOHCEO19319` → `CE019319`;
   - org 31536131 `GBCOHIPO30808` → `IP030808`;
   - org 31581165 `GBCOHNIO18750` → `NI018750`.

   Expect `recorded 3, stale 0`. Then the rekey dry plan should read:
   - `GB/national/GBCOHCEO19319>move:GBCOHCE019319`;
   - `GB/national/GBCOHNIO18750>move:GBCOHNI018750`;
   - a merge of 31536131 into 31534918 (`GBCOHIP030808`). Its names ("Co-Op Funeral Care" against "Funeral Services
     Limited T/A Co-op Funeralcare", trimmed at `T/A`) will NOT agree, so expect a `names` denial. It needs a 454
     verdict: `scheme` `GB:rekey`, `key` `GBCOHIPO30808~IP030808`, `members` `[31534918, 31536131]`, `merge`, `high`.
     The register name is FUNERAL SERVICES LIMITED.

   Then run rekey wet with the dry plan's `keys`.

## Verify

    curl -s https://tenders.zebreus.click/v1/organizations/10312649 | jq -c '[.id, .identifier, .merged_into]'

- **open** (2026-10-01 12:3x UTC): `[10312649,"SCO55775",null]`. Galliford Try Infrastructure Ltd stands under the typo,
  beside org 10312664 (`SC055775`, the register's GALLIFORD TRY INFRASTRUCTURE LIMITED).
- **done**: `[null,null,10312664]`, meaning it was merged into the org that carries the register's spelling.
  - The fold landed but the survivor rule did not: the line still reads `[10312649,"SCO55775",null]` and org 10312664
    answers `merged_into` 10312649.
  - The decision is not to fold: the closure goes on line 3 with its reason, and this line stays open by design.
