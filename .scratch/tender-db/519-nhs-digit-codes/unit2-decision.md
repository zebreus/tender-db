# 519 unit 2: decision draft (synthesis; based on the extend-513 design)

**Path short names:**
- `canonical.rs` = crates/store/src/canonical.rs
- `nhs_ods.rs` = crates/ingest/src/nhs_ods.rs
- `crosswalk.rs` = crates/ingest/src/crosswalk.rs
- `project.rs` = crates/ingest/src/project.rs
- `read.rs` = crates/store/src/read.rs
- `supervisor.rs` and `admin.rs` = crates/app/src/…
- `nhs_body.rs` = crates/store/tests/nhs_body.rs

I re-read every line I cite at HEAD 1530967 on 2026-10-12.

**Where the counts come from.** I joined `519-nhs-digit-codes/sub-icb.json` with the closing 513 dry plan `513-nhs/u5/plan-u6-dry-after.json` (171 codes applied, ledger 273, survivors read from the holders that carry `GBNHS<P>` as a merged identifier). Mention names are the census's top five per three-character tail (`off_allowlist`). The names of the five-character children are not read yet (M1).

## 0. The design in brief, and what changed from the three designs

**The design.** There is no new verdict scheme and nothing is minted.
- **Child code.** An ODS code that ODS dates under a body's code:
  - an RO98 sub-ICB location under an RO261 ICB (relationship RE5);
  - a measured RO198 trust site under its RO197 trust (relationship RE6).
- **The map.** A frozen sibling table, `nhs-ods-children.tsv`, holds the dated map.
- **The census.** The `nhs-body` census lists the holders of every child code dated under an allowlisted code P, in any period, as holders of P.
- **The verdict.** A new reviewed nhs-body cohort on P (HIGH, members-exact) names P's live survivor plus the eligible child holders.
- **The fold.** `merge_nhs_bodies` and `nhs_body_fold` fold them. The only change there is an identity-loser branch in the evidence.
- **The lookup.** The child literal lands on the survivor as a merged identifier (`canonical.rs:4782-4795`).

Only three mechanics are new:
- (a) an exact-triple arm in the nhs-body alias, gated by name and consulted before `org_of`;
- (b) a cohort scope on `unwind_nhs_body`;
- (c) for holders whose names are mixed, the stray mentions are rehomed first through issue 317's existing per-mention verdicts, and then the holder folds whole.

**Kept from extend-513:**
- a reviewed verdict, with eligibility derived from structure;
- a strict name gate on every name, applied at fold time and again at bind time;
- the triple arm placed before `org_of`;
- empty name keys on identity losers;
- S4's `identity` / `identity-stray` labels;
- the `child-into-anchor` and `child-carries-ppon` holds;
- QHL15E handled through a 452 verdict plus a PPON rekey.

**Flaws removed, and how:**

| # | Flaw | Fix |
|---|---|---|
| 1 | A 10th column and RC2 rows in `nhs-ods.tsv` would change 513's parse (`nhs_ods.rs:376-382`), its VERSION stamp (`supervisor.rs:10466`) and the rival set. Every row is a rival (`nhs_ods.rs:18-19`, `298-307`). | A sibling file with its own `CHILDREN_VERSION`. `nhs-ods.tsv` stays byte-identical, and so do 513's 233/235 and 116/116 tallies. |
| 2 | The strict gate was never run against names the census already had. 13T, 26A, 15E, 15N and 93C all carry a refused name (719 mentions), so phase A was overstated and S1's halt would trip on day one. | Rehome-then-fold for strays that belong to another body (§1.4). S1 halts on *false* refusals (the body's own misspellings), not on strays from other bodies. |
| 3 | "13T is the CSU, never a member" (and separate-arm's "hosted" fold into 0AR). | 481 of 13T's 504 mentions are named NENC: 464 + 11 + 4 + 2 in the top five. 15 are named CSU. 13T is NENC publishing under its own sub-ICB code. Rehome the CSU-named mentions, then fold into QHM (ruling 3). |
| 4 | 15C held because "QUY is held". | QUY was applied in 513 U6 (wet 2286; PPON survivor 31536069), so 15C is phase A. |
| 5 | 03K listed as "no RO261 parent". | `ods-table-codes/03K.json` is `{"http":"000"}`, a failed fetch. Its table row is "NHS HUMBER AND NORTH YORKSHIRE ICB - 03K", Active. Refetch it, and the generator exits on any status other than 200 or 404. |
| 6 | One verdict lineage per body: a `keep` on P drops every alias row of P (`canonical.rs:17524-17540`; pinned by `nhs_body.rs:2077`). | Holds are made by leaving a holder out of `members`, never by `keep`. The census counts `keep_over_applied`, and S0 requires 0. |
| 7 | S4's `triple_standing = 0` contradicts the design itself: refused names mint strays. | Replaced by `identity` (halt) versus `identity-stray` (listed) (§3.7). |
| 8 | The unwind leaves identifier-less mentions on the survivor. | New denial `child-impure`: every mention of an identity child must carry the holder's own triple, so each one goes back through `Dest::Identity` (`canonical.rs:27421-27425`). |
| 9 | Triples claimed by two codes are poisoned. | Arms are routed by name (from separate-arm): exactly one admitting arm binds. |
| 10 | The rivals in the name gate. An RO98 row's current name is "<ICB> - code" and declares the ICB role (`nhs_ods.rs:109`), so it beats P for a name that carries the code. | `child_check` drops Active RO98 rows from the rival set and adds an NHS England refusal (from separate-arm). |

**Not adopted, because it is fatal elsewhere:**
- **identifier-verdicts:**
  - Its rekey route lets a 454 pair verdict admit past the names gate (`canonical.rs:29543-29550`), so the ODS gate becomes overridable; 13T → 0AR would file about 481 NENC mentions under the CSU.
  - Its name-blind "open" tier is cached.
  - Its fallback runs only after an `org_of` miss.
  - It folds canonical rows into provisional anchors without a 300 amendment.
- **separate-arm:**
  - its fuzzy alias shapes (`hosted`, `transposition`, `embedded`);
  - a second planner, ledger and alias lineage for each body;
  - exact parity: 513's parity already folds only the live ∩ stored set, and defers unforeseen codes (`canonical.rs:26977-27002`). The parity code is shared, so it stays as it is.

## 1. The rule: which org a child identity belongs to

### 1.1 A reviewed verdict, with eligibility derived from structure

The act is a reviewer + challenger verdict (300's ladder: two canonical rows with different identifiers merge only by review, issue 362). A holder h of child code C may be a member of P only if **every** structural condition below holds. No verdict overrides them.

- **E1. Map.** The children table dates C under P (RE5→RO261 or RE6→RO197). P is allowlisted, has an applied 513 body, and its live survivor is a PPON org. That last part is phase A; anchors are §1.5 and digit parents are U6.
- **E2. Shape.** h is one of:
  - an identity holder whose own identifier is exactly (GB, national, `GBNHS<C>`) (`child-triple-shape`);
  - a provisional cluster that coded parties of a zero-only child tail bound by name (00P, 00N, 00L: the all-zero rule mints nothing).

  513's `minted-identity` denial (`canonical.rs:26547-26550`, `26575-26577`) is lifted for exactly that triple and nothing else.
- **E3. Purity and completeness.**
  - Every mention read carries h's own triple (`child-impure`).
  - A capped read denies (`child-names-unread`; caps at `canonical.rs:7663-7672` and `7715-7718`).
- **E4.** No PPON, whether own or merged (`child-carries-ppon`). Not withheld by a 452 verdict (`child-withheld`).
- **E5.** The name gate (§1.3) admits the head name and every distinct mention name.
- **E6.** 513's remaining D6 denials still apply: `gone`, `generic`, `consortium`, `successor-name`, `contested`.

Why a verdict rather than a derived "fold into the name parent" rule:
- 13T and other mixed holders need a notice-level read.
- These are canonical rows.
- The mass is one batch (at most about 52 + 25 holders).

### 1.2 Dated parents

The name chooses among the dated parents. It never means "today's parent".

| child | dated parents | chosen | note |
|---|---|---|---|
| 26A | QMM (2019-10-23 → 2026-03-31), then T6Y0W | QMM | T6Y0W has five characters and is not allowlisted |
| D4U1Y | QNQ, then QRL | QNQ | Both are allowlisted, but "NHS Frimley ICB" does not fit QRL |
| 93C | QMJ, then Z9B2Z | QMJ | |
| 06H | QUE, then S1Y5D | QUE | QUE has an anchor survivor: held |
| 10Q | QU9, then S0E4D | QU9 | A digit parent: U6 |

- **Two allowlisted parents.** A child dated under two allowlisted parents is listed under both. The gate admits at most one of them: `Admitted` requires strictly beating every rival, and the other parent is a rival unless it is a predecessor (`nhs_ods.rs:298-312`). Two verdicts naming the same holder make it `contested` (`canonical.rs:26439-26447`).
- **No date fence**, as in 513. Instead the census reports, per name, the counts before P's start, inside P's period and after P's end. Post-merger mentions named after the new body fail the gate under P (at fold and at bind time).

### 1.3 The name gate: `child_check(P, name)`

This is `check` (`nhs_ods.rs:275-327`) with two changes:

1. **Own names and rivals.**
   - Own names stay as they are: P plus P's predecessors (`nhs_ods.rs:289-291`).
   - Rivals become every row except own and except **Active RO98** rows. Since 2026-04 those rows read "<current ICB> - code" and duplicate an RO261 row already in the table.
   - Inactive RO98 rows (names of abolished CCGs) stay rivals.
2. **NHS England refusal (`nhse`).** A name with the token `england` that declares no ICB, trust or CSU role is refused. NHS England's regions (RO209/RO210) are not in the table.

Only `Admitted` passes; a `tie` refuses. The same fn gates the bind (§3.5), so prevention is never looser than the reviewed merge.

Expected answers (pinned by tests, §6):

| name | under | answer |
|---|---|---|
| "NHS Norfolk and Suffolk ICB" | QMM | `better{T6Y0W}` (as `nhs_ods.rs:854`) |
| 13T's "NORTH OF ENGLAND COMMISSIONING SUPPORT" | QHM | `role` |
| 04Y | QNC | `better{QOC}` |
| 03Q | QOQ | `nhse` |
| W2U3Z | QRV | `tie{Z9B2Z}` (QRV's own name ties, `nhs_ods.rs:859`) |
| a B2M3M name with or without "- B2M3M" | QWU | admitted |

### 1.4 Mixed holders: rehome, then fold

When a phase-A holder carries names the gate refuses:
1. The reviewer and challenger read each refused mention's notice.
2. If the mention is another body's, they post a 317 rehoming verdict (`POST /admin/rehoming`, `admin.rs:48`, `93-151`; recorded by `canonical.rs:28346`). It is `rehome`, HIGH, and its target is an existing org.
3. `apply-rehoming` (`supervisor.rs:1674-1681`; `canonical.rs:28460`; the wet run is tuple-exact) moves the mention, stamps the touched tenders stale and requeues their notices.
4. The re-census sees all remaining names admitted.

A refused name that is the body's own misspelling cannot be rehomed: the holder is held (S1, ruling 2).

The rehome candidates seen so far (about 22 mentions, top-five names only):

| holder | refused mentions | proposed target |
|---|---|---|
| 13T | 15 CSU-named | 0AR survivor 30929531 (ruling 3) |
| 26A | James Paget FT, Norfolk & Norwich FT | the RGP and RM1 bodies |
| 15E | "College Road Surgery", "NHS Black Country ICB" | Black Country → QUA 31542353 |
| 15N | BNSSG ×2 | QUY 31536069 |
| 93C | "N E L Commissioning Support Unit" | to be read; also "North Central London Integrated Care System" |

### 1.5 The partition (52 orgs / 1,844 mentions)

| class | orgs / mentions | unit 2 |
|---|---|---|
| A1, clean, PPON survivor: B2M3M 600, D4U1Y 65, 15C 31, D2P2L 16, D9Y0V 10, 16C 8, 99C 6, 03R 5, M2L0M 4, 11J 3, 15F 3, 42D 2, 78H 2, 01H, 84H, 02Y, 36J, 18C, 92A (1 each) | 19 / 761 | fold |
| A1m, mixed, PPON survivor: 13T 504, 26A 115, 15E 46, 15N 31, 93C 23 | 5 / 719 | rehome strays, then fold |
| zero-only clusters under QHM: 00P (6 holders), 00N (1), 00L (1); 31 parties | — | name members (513 D2's provisional→canonical scope) |
| A2, anchor survivor: QOX, QKK, QKS, QWE, QSL, QMF, QHG, QUE, QNC, QNX, QYG (92G 80, 72Q 47, 91Q 41, …), plus 04Y (names QOC) | 14 / 291 | hold `child-into-anchor`; U7 per ruling 5 |
| A3, digit-identity parent: QF7, QU9, QK1, QE1, QH8, QJ2, QM7 (03N, 10Q, 03W, 01A, 06Q, 15M, 06K, 06N); plus 00X (2 holders) | 8 / 30 | U6, identity bodies |
| rest: 15A 23 (S0E4D has no org; 15A *is* Thames Valley's only org, so leave it); 03K 12 (refetch; the HNY name points to QOQ, PPON 31573365, probably A1); W2U3Z 3 (tie); 14J 2 and 14N 2 (RO210 parents only); 03Q 1 (NHS England) | 6 / 43 | listed only |

**Trust sites** (class B, 25 orgs / 36 mentions; about 10 single-mention orgs named after their trust) follow the same rule through RE6. Site-named orgs fail the gate, which keeps 513's "hosted sites stay separate". Posting them is ruling 7.

## 2. The data

### 2.1 The table

New file `crates/ingest/sdk/nhs-ods-children.tsv`, header `# version: ods-children-2026-10-<dd>`. A test pins `nhs_ods::CHILDREN_VERSION` to the header.

Columns:
`child, child_status, child_rc, child_role (RO98|RO198), rel (RE5|RE6|-), parent, parent_role, op_start, op_end (empty = open), legal_end, child_name`

- **One row per (child, parent, Operational interval).** Adjacent intervals with the same parent are merged; for example, 06K's S1Y5D periods 04-01..04-07 and 04-08.. become one row.
- **No RO261 or RO197 parent** (14J, 14N): exactly one `rel -` row. The dropped RO210/RO173 relations are counted in the header.
- **RO198 rows** only for the measured site codes (`MEASURED_RO198`).
- **`nhs-ods.tsv` and `VERSION` do not change.**

### 2.2 The generator

One generator and one `raw/` cache: extend `.scratch/tender-db/513-nhs/u1-table/gen_nhs_ods_table.py`.
- `LIST_ROLES` (:52) stays unchanged. Add `MEASURED_RO198` beside `MEASURED_RO172` (:61).
- Call `org_record` (:154) for all 344 RO98 rows (A3 123, A5 42, I3 174, I5 5) and for each measured RO198 code.
- Seed `raw/` from `519-nhs-digit-codes/ods-table-codes/` (66 records; 03K's failed fetch is refetched, never seeded) and from `ods/`.
- Keep `Rels.Rel` with `id == RE5` and a target whose primary role is RO261. For sites, keep RE6 → RO197. Record Operational dates.
- Exit non-zero on any of:
  - a fetch status other than 200 or 404;
  - a parent that is not a row of `nhs-ods.tsv`;
  - overlapping intervals for one child;
  - a site whose RE6 target is not RO197;
  - `nhs-ods.tsv` coming out not byte-identical.
- `--offline` rebuilds both files byte for byte.
- Print the children per allowlisted parent and the codes with no parent.

### 2.3 Reader and types

**`nhs_ods.rs`:**
- `CHILDREN_VERSION`;
- `Parent { code, rel, start, end: Option }`;
- `parents(child)` and the reverse index `children(parent)`;
- `child_check(parent, name)`;
- census adapters `census_children(parent) -> Vec<store::NhsChildCode>` and `census_child_check(parent, name) -> store::NhsNameCheck`.

**Store:**
- `NhsOdsParent` and `NhsChildCode`, beside `NhsOdsCode` (`canonical.rs:5833-5846`);
- `NhsBodyArgs` (`canonical.rs:5904-5934`) gains `children` and `child_check`, wired at `supervisor.rs:10461-10466`.

### 2.4 What the census lists for child codes

A new child pass in `plan_nhs_bodies_on`, after step 2 (`canonical.rs:26337-26353`), under `Box::pin`. For each P in `by_code` and each C in `children(P)`, in any period:

**Finding the holders:**
- **Identity holders.** `orgs_with_triple(GBNHS<C>, national, GB)` (`canonical.rs:8502-8520`, a seek on `organizations_identifier_id`). This is the only way five-character children (B2M3M, D4U1Y, D2P2L, D9Y0V, M2L0M) are seen, because `nhs_tail` takes three characters only (`nhs_ods.rs:159-163`; used at `canonical.rs:26251-26252`).
- **Coded holders.** For a three-character C, the orgs the harvest bound for tail C. Today these are listed under `off_allowlist` (`canonical.rs:26761-26798`).
- Both kinds join P's `holder_ids` and `holder_of`.

**What is listed:**
- **Per code**, `children[]`: `{code, role, ods_name, periods_here, other_parents, holder_ids, coded_parties}`.
- **Per holder:**
  - `child_of`;
  - `child_names[]`: `{name, name_norm, n, before_start, in_period, after_end, check_here, check_other_parents[]}`. To build it, `nhs_org_facts` (`canonical.rs:7618-7737`) keeps, per name_norm, the publication instants of the mentions it reads. It already seeks `published_at` per notice (`7698`).
  - `via` gains `child`. Today `(false, false, _)` reads "ppon" (`canonical.rs:26605-26613`).
  - the denials from §1.1.
- **Report-wide:** `child_codes_seen`, `child_holders`, `child_mentions`, `child_denied{reason}`, `child_unanchored` (every dated parent is off the allowlist), `child_no_parent`, `keep_over_applied`.

**Status changes** (`canonical.rs:26659-26716`):
- The D4 test (`26692-26710`) also exempts identity child members, because their name key is never armed.
- New status `child-into-anchor`, after `member-denied`: the verdict names an identity child, but `nhs_body_survivor` would not pick a PPON member. Such a verdict is never admitted.

**Why widening is safe.** `applied` is decided before members-exact (`canonical.rs:26663-26666`), and every allowlisted ICB code is applied (171 of 174; DQF, RFF and RVY are held and are not ICBs).

## 3. Merge mechanics

### 3.1 The verdict

- **Key and row.** The key stays (GB, GB:nhs-body, P), so `validate` is untouched (`canonical.rs:8446-8452`). The new cohort is its own row, because the primary key includes the cohort (`canonical.rs:628-643`). The newest merge is the standing verdict (`canonical.rs:29123-29125`).
- **`members`** = {P's live survivor} ∪ eligible child holders ∪ zero-only clusters.
- **`reviewed_holders`** = P's full live, widened holder set. That is the 513 holders plus the child identity and coded holders, including strays and rehome targets. A newcomer makes the verdict `stale`.
- **Holds** are made by leaving a holder out of `members`, never by `keep`.

Why not a key per child:
- It duplicates 513's U2–U4.
- It splits members-exact across two keys.
- A code-wide 513 unwind would then strand child mentions. Under one lineage, that unwind dissolves both cohorts (§3.6).

### 3.2 The fold

`merge_nhs_bodies` and `nhs_body_fold` stay as they are, except:
- `nhs_body_survivor` (`canonical.rs:6608-6643`) leaves identity members out of the anchor tier (`6634`). A wet hold `child-into-anchor` backs up the census status.
- For an identity loser, the evidence (§3.4) is written with empty name keys, and the fold's key set is emptied too. Today it always inserts `loser.name_norm` (`canonical.rs:27148-27155`, line `27150`).
- `repoint_org_references` (`canonical.rs:27175` → `4617`) writes `GBNHS<C>` as a merged identifier under rule `nhs-body` (`4782-4795`). The row is deleted, the ledger row is written and `organization removed` is announced (`27181-27188`).
- `GBNHS<P>` is already standing on the survivor, so re-recording it is a no-op (`27190-27192`; `4930-4937`).
- The new cohort is stamped (`27201-27206`).
- For a folded zero-only child, the fold also records `GBNHS<00x>` (rule `nhs-body`). This is lookup only and is lost on a rebuild, like a one-member body's code.
- `loser-ppon-unaliased` is unchanged (`canonical.rs:27040-27048`).

### 3.3 The loser's `GBNHS<C>`

- `?identifier=GBNHS<C>` answers the survivor through the merged leg (`read.rs:4335-4357`).
- The loser's id answers 308 (`read.rs:4841-4868`).
- A from-archive rebuild empties the merged table; the triple arm re-records the identifier on each bind.

### 3.4 Ledger evidence

`nhs_body_ledger_evidence` (`canonical.rs:6528-6577`) writes, for an identity loser:
- `loser_kind: "identity"` (today `ppon|name`, `6567`);
- `loser_id = GBNHS<C>` (already the loser's literal, `6566`);
- a new `loser_identifier_kind`;
- `loser_country` `GB`;
- `loser_name` and `loser_names` kept;
- `loser_name_norm: ""` and `loser_name_norms: []`;
- new `child_code`, `child_rel` and `child_periods`.

Because the keys are empty, every existing name path ignores these rows with no code change:
- the alias skips them (`canonical.rs:17496-17499`);
- the unwind's key set drops empty norms (`27389-27398`);
- the S4 name probe is skipped (`26893`).

### 3.5 The alias

**Today a merged identifier routes nothing:**
- `org_of` is loaded from `organizations.identifier` only (`canonical.rs:17044-17091`);
- `merged_hit` applies only to O/0-folded keys (`18720-18737`);
- GB-NHS keys nothing (`crosswalk.rs:429-433`);
- the altid alias is asked only for `GB:ppon` keys (`canonical.rs:8015-8017`, `19071-19088`).

So the next child-coded mention mints a new org (`canonical.rs:19133-19174`).

**The new triple arm:**
- **Arming.**
  - `NHS_BODY_ALIAS_LEDGER_SQL` (`canonical.rs:6270-6276`) also reads `$.loser_kind`, `$.loser_id` and `$.loser_identifier_kind`.
  - In `arm_nhs_body_alias_on` (`17487-17625`), identity rows go into `triples: HashMap<triple, Vec<NhsBodyArm{code, target, keep_name, at}>>`. They are diverted before the name-key `continue` at `17496`.
  - The arming rules match the name keys: the `keep` drop applies (`17524-17540`), the newest row per (triple, code) wins, and anchor chains are followed (`17553-17582`).
  - Withheld triples refuse (`17606-17610`).
  - Country-less triples are never armed.
- **The check.** `AltIdAliasRules` (`canonical.rs:8012-8034`) gains `nhs_child_check: Option<fn(&str, &str) -> NhsNameCheck>`.
  - `project::altid_alias_rules` (`project.rs:2053-2065`) sets it to `Some(nhs_ods::census_child_check)`.
  - The test literals (`nhs_body.rs:1205-1206`, `crates/store/tests/altid_merge.rs:1699-1700`) set `None`.
  - With `None` no triple is armed, and that is logged.
- **The bind.** At the head of the identified branch, before `org_of.get` (`canonical.rs:18633-18634`), under `Box::pin`, and only for an armed triple:
  1. Collect the arms whose code's `child_check` admits `m.name`, memoized per (code, name_norm).
  2. If exactly one arm admits, resolve its target with the target half of `nhs_body_alias_bind` (`17671-17768`), factored into a shared, behaviour-preserving helper. 513's alias tests (`nhs_body.rs:1587-1798`) guard the refactor.
  3. Record `GBNHS<code>` and `GBNHS<C>` on the target once per (literal, org) per run.
  4. Never cache the result in `org_of`.
- **Refusals.** None admitting → `refused_name`; two or more → `refused_ambiguous`; withheld → `refused_withheld`. Each falls through unchanged to `org_of` and then the mint, so a mis-publisher stands on its own `GBNHS<C>` row.
- **Why the arm must precede `org_of`.** Otherwise the first refused mint claims the triple (`canonical.rs:19151-19153`), and that row then catches every later mention, admitted names included.
- **Counts** on `NhsBodyAliasCounts` (`canonical.rs:6464-6489`): `triples`, `triple_arms`, `triple_asked`, `triple_bound`, `triple_minted`, `triple_refused_name`, `triple_refused_ambiguous`, `triple_refused_withheld`, `triple_recorded`. They go on an `[issue 519]` diag line beside 513's (`18557-18566`), zeros included.

### 3.6 The unwind

Identity losers already dissolve in `unwind_nhs_body` (`canonical.rs:27244-27652`):
- the literal set (`27399`) routes them to `Dest::Identity` (`27421-27425`);
- they land on the standing row or a minted one (`27455-27463`, `27514-27520`);
- the old loser redirects by identity (`27585-27604`);
- its literal is dropped off the survivor (`27612-27619`).

Additions:
1. **Cohort scope.** `NhsBodyUnwindArgs` (`canonical.rs:6354-6365`) gains `cohort: Option<&str>`, which filters rows by `$.cohort`.
   - The Spec variant (`supervisor.rs:426`) and the request parse (`1574-1577`) take `cohort`, refused for other rules the way `code` is (`1590-1593`).
   - The unwind's body moves behind `Box::pin`, like the canary's. `run_match_identifiers_spec` sits about 40 bytes under its future-size budget (`supervisor.rs:6210-6211`).
2. **Keep `GBNHS<P>`** when the unwind is scoped and other rows of the code remain. Today it is dropped unconditionally (`canonical.rs:27606-27611`).
3. **Drop the alias-recorded `GBNHS<C>` rows** (rule `nhs-body`, loser = org = survivor). The delete at `27612-27619` matches only the old loser.
4. **Last in, first out.** A scoped unwind refuses while a later cohort's un-retired rows for the same code stand.

Survivor agreement (`canonical.rs:27366-27375`) is unchanged. `cohort: None` dissolves 513 and 519 together.

### 3.7 S4

In `nhs_body_ledger_listing` (`canonical.rs:26824-26936`), the literal-regrown probe (`26913-26928`, labelled "ppon" today) labels identity rows:
- `identity` (halt) when the regrown row's head name or any mention name passes `child_check(P)`: the arm should have bound it;
- `identity-stray` (listed, no halt) otherwise.

The listing takes `child_check` as a new argument.

## 4. Typo codes and foreign values

| values | path |
|---|---|
| QHL15E (10 mentions) | 452 `wrong`, with `correct_identifier` set to QHL survivor 30916123's GB-PPON. The rekey merges it **today**: the right number keys `GB:ppon` E1 (`canonical.rs:29348-29352`), the owner walk finds it (`29385-29418`), and the names gate applies (`29543-29554`). The literal is not carried (`4775-4777`), so `?identifier=GBNHSQHL15E` answers nobody, as intended. Repeats are routed by the rekey alias (`17126-17169`). No 454 pair verdicts. |
| Companies House numbers under GB-NHS (61) | 452 `wrong` with a GB-COH correction. Merge into the owner, or a move. A move writes the bare 8-digit number (`canonical.rs:29486-29490`), which still keys `GB:coh` E1 (`crosswalk.rs:400-404`, `423`). Accepted as it is. |
| PPONs under GB-NHS (16) | 452 `wrong` with a GB-PPON correction; merges work. **Hold every PPON *move*:** it writes the bare 12-character serial (`canonical.rs:29486-29490`), which keys nothing, because `GB:ppon` needs the `GBPPON`/`PPON` prefix (`crosswalk.rs:392-395`). Re-post such verdicts at medium until U8. |
| NHS→NHS typos: RF1→R1F (31542853), T6YOW→T6Y0W (30919647), typed RY2/RW4/RY7/RP5DR (about 9 mentions in all) | 452 `wrong` with a correction now. This withholds and flags them; the exact triple still binds. Rekey answers `unkeyed` (`canonical.rs:29348-29352`). Optional U8: a `GB:nhs` key used only by the rekey arm; merge shape only, never a move; owners are own-identifier carriers only; plus `check(right, every loser name)` admitted, with no pair-verdict override. |
| Junk, CQC and OCDS ids, glued pharmacy codes, 404 codes | 452 `wrong` without a correction: withheld, and the org stays. |
| Real codes on the wrong holder (RJ7, Y54/Y61/Y62, RH8C3, D8D1G); NHS England class C | Not unit 2. 317 rehoming or leave; class C becomes its own issue. |

Typos are not folded as members: they are outside the dated map, and a fuzzy relation is exactly what 513 limited to the unique O/0 fold.

## 5. Dry plan, stop rules, canary

**The dry plan** is the same job, `match-org-identifiers {"rule":"nhs-body"}` → `nhs-body-plan`, plus:
- `children[]` per code;
- `child_names` per holder, with dated counts and checks;
- the child denials;
- `child_unanchored`, `child_no_parent`, `keep_over_applied`;
- the status `child-into-anchor`;
- `nhs_body_regrown` kinds `identity` and `identity-stray`.

**Stop rules:**
- **S0.**
  - Posting only from the live census: `reviewed_holders` equals the widened set.
  - 0 members with any denial.
  - `keep_over_applied` is 0.
  - Every rehome verdict is applied before the cohort is posted.
- **S1.**
  - Reviewer and challenger agree HIGH on every member and every rehome.
  - 13T joins only under ruling 3; 03Q, 04Y, W2U3Z and 14N never join.
  - Halt for ruling 2 if more than 3 phase-A holders, or more than 5% of phase-A mentions, are held *only* by refusals of the body's own misspellings.
- **S2.** 513's parity (`canonical.rs:26975-26991`), unchanged.
- **S3.** A wet run capped at 5 bodies (`max_groups`): QWU (B2M3M), QMM (26A, moved), QNQ (D4U1Y), QHM (16C, 99C, 01H, 84H and the 00P clusters), QHL (15E, after rehome). Read for each:
  - each loser answers 308;
  - `?identifier=GBNHS<C>` answers exactly the survivor, and `GBNHS<P>` still does;
  - survivor mentions ≥ pre-image + members;
  - the 513-cohort ledger rows are unchanged;
  - non-members are untouched.

  This is a new `s3check_children.py` beside `513-nhs/u5/s3check*.py`.
- **S4.**
  - On every fold, read the `[issue 519]` lines; list every refused name.
  - On every dry run: regrown kind `identity` = 0; `identity-stray` listed; 513's `name` and `ppon` kinds still [].
- **S5. Canary** (`nhs-body-canary`, `canonical.rs:27668`; it arms through `arm_altid_alias` at `27679`):
  - 3 child-coded notices each (oldest, middle, newest) for B2M3M, 26A and D4U1Y, plus 13T if it is folded;
  - the newest post-2026-04 notices of 26A and D4U1Y, if any exist;
  - 3 notices of QWU (uncoded, PPON-first, coded) as a 513 regression check.
  - Pass: 100% agreement and `triple_bound` > 0. Rehomed mentions are excluded: by design they re-resolve to a stray.
- **S6.** The scoped unwind is store-tested. A dry `nhs-body-unwind {code: "QWU", cohort}` on prod after the capped run must list only child moves, with `GBNHSQWU` kept. No rollback past U4 without it.

**Run order:**
1. Measure (M1–M7).
2. U2 dry deploy, then the census.
3. Rehome: dry, then wet.
4. Re-census, then post the cohort.
5. Dry run.
6. Capped wet run (5), then the S3 read.
7. Canary, then the dry scoped unwind.
8. Full wet run, then a closing dry run (`admitted_codes` [], regrown `identity` []).

## 6. Tests

**`nhs_ods.rs`:**
- `CHILDREN_VERSION` equals the header.
- The main table, `VERSION` and the 233/235 and 116/116 tally are unchanged.
- `parents`: 26A is [QMM 2019-10-23..2026-03-31, T6Y0W 2026-04-01..]; 06K's intervals are merged; 14J and 14N give `rel -`; 03K has a parent; RBT20 → RBT by RE6.
- `child_check`, every row of §1.3's table, plus:
  - "James Paget…" under QMM is refused;
  - "College Road Surgery" under QHL gives `zero`;
  - "(Leeds Place)" under QWO is admitted.
- An ignored tally harness over the M1 names.

**`nhs_body.rs`** (the fixture `TABLE` at `26` gains RO98 and RO198 rows and a `children` stub):
- **Census:**
  - `a_five_character_child_is_found_by_its_identity_seek`
  - `a_child_holder_is_listed_under_every_dated_parent`
  - `a_child_identity_holder_is_not_denied_minted_identity_but_a_foreign_identity_still_is` (beside `640`)
  - `a_child_name_another_body_wins_is_child_name_mixed`
  - `an_uncoded_mention_makes_a_child_impure`
  - `a_merged_ppon_denies_a_child`
  - `a_moved_child_is_eligible_only_where_its_names_fit`
  - `a_child_newcomer_stales_the_new_cohort_never_an_applied_verdict`
  - `a_child_without_a_ppon_survivor_is_child_into_anchor`
  - `a_zero_only_child_cluster_joins_as_a_name_member`
  - `the_d4_gate_skips_identity_child_members` (beside `922`)
  - `a_keep_on_an_applied_code_is_counted`
  - `via_labels_child`
  - the child seek in the plan guard (extend `1094`)
- **Wet:**
  - `a_reviewed_child_folds_into_the_ppon_survivor_and_its_code_answers_it` (308, merged identifier, `loser_kind identity`, empty keys)
  - `a_second_cohort_on_an_applied_code_folds_only_the_children`
  - `a_rehomed_stray_leaves_its_holder_foldable`
- **Alias:**
  - `the_next_child_coded_mention_named_as_the_body_binds_the_survivor`
  - `a_refused_name_stands_on_its_own_row_and_a_later_admitted_one_still_binds_the_survivor` (precedence)
  - `the_child_arm_survives_a_rebuild_in_notice_order`
  - `two_arms_route_by_name`
  - `a_withheld_child_triple_binds_nobody`
  - `a_country_less_child_mention_is_untouched`
  - `no_child_check_arms_no_triple`
  - `a_keep_on_the_code_drops_its_triples` (beside `2077`)
  - extend `an_unarmed_resolver_is_byte_identical` (`1779`)
- **Unwind:**
  - `a_cohort_scoped_unwind_returns_the_children_through_e0_and_keeps_the_513_fold`
  - `a_scoped_unwind_under_a_later_cohort_is_refused`
  - `a_code_wide_unwind_still_unwinds_every_cohort` (beside `1834` and `1983`)
- **S4:** `a_regrown_child_literal_named_as_the_body_halts_and_a_stray_is_listed`
- **Canary:** extend `2181` with a child-coded notice.

**`supervisor.rs`:**
- `cohort` parsing and its refusal for other rules;
- the boxed unwind arm;
- re-run `an_execute_without_an_expected_count_is_refused` (CLAUDE.md stack note).

**Gate.** Everything runs through `ops/check.sh`. Focused runs use the gate's flags and package set (`--features tender-db/server`).

## 7. Units in build order

- **U0, docs, no code.**
  - The decision in issue 519.
  - A note beside 513's D2 amendment on 300 §10: an nhs-body verdict may name canonical holders of child codes dated under the body's code, never into a provisional survivor.
  - The campaign steps in `docs/operations.md:2222`.
  - Corrections to record:
    - 03K is a failed fetch;
    - 13T is NENC-named (481 of 504);
    - `sub-icb.json` predates wet 2281 and wet 2286 (QKK, QMF and QYG are now anchors; QMJ and QUY have PPON survivors).
- **U1, pure, no deploy.**
  - the generator;
  - `crates/ingest/sdk/nhs-ods-children.tsv`;
  - `nhs_ods.rs` (§2.3, plus its tests);
  - `canonical.rs`: the `NhsOdsParent` and `NhsChildCode` types only.
- **U2, census, dry deploy.**
  - `canonical.rs`: the `NhsBodyArgs` fields; the holder, code and report fields; per-name instants in `nhs_org_facts`; the child pass; the denials; `via`; the statuses; D4; `keep_over_applied`.
  - `supervisor.rs`: the wiring at `10461` and the summary line.
  - `nhs_body.rs`: the census tests.
  - Before it ships, M1–M3 must be read.
- **U3, adjudication** (`.scratch/tender-db/519-nhs-digit-codes/u3/`).
  - A reviewer and a challenger per batch.
  - Rehoming verdicts → `apply-rehoming` dry → wet.
  - Re-census, then post cohort `nhs-body-children-<date>`.
  - The 452 cohort for QHL15E and the COH and PPON corrections, after a dry `rekey-plan`.
- **U4, one deploy.**
  - `canonical.rs`:
    - the evidence at `6528`;
    - the identity-loser branch of the fold (`27140-27175`);
    - the survivor tier at `6608`;
    - the alias SQL at `6270`, the arming at `17487`, the bind helper factored from `17671-17768`, the hook before `18634`, the counts at `6464` and the diag line at `18557`;
    - the S4 listing at `26824`;
    - the unwind at `27244` and `6354`;
    - `AltIdAliasRules` at `8012`.
  - `project.rs:2053`.
  - `supervisor.rs`: the Spec at `426`, the parse at `1574-1593`, and the boxed arm.
  - The tests.
- **U5, the run** in §5's order.
- **U6, identity bodies** (A3, 8 orgs / 30 mentions plus 00X). Planned after U5's read.
  - Census entries for non-allowlisted three-character RO261 parents with exactly one standing, non-withheld `GBNHS<P>` org.
  - That org becomes the survivor, with target `Ppon{GB, national, GBNHS<P>, ppon: None}`. That target is already handled: the exact triple, else an error, in `canonical.rs:6795-6798`; `org_of` then a mint in `17698-17730`.
  - The fold skips recording `GBNHS<P>` when it is the keep's own identifier.
- **U7, conditional:** anchor parents (A2) per ruling 5.
- **U8, optional:** the `GB:nhs` rekey key for NHS→NHS typos, and the PPON move spelling (`GBPPON{k}`).
- **Then 469 U7.** It must deny any register pair whose GB-NHS value is a code in the child map; otherwise its "keep the register org" rule inverts a held child.

## 8. Risks

(What to measure first is the final section.)

1. **E0 precedence changes for armed triples only:** the name check runs before exact equality. Mitigated by the precedence test, S4's `identity` halt, the canary, and an unarmed resolver staying byte-identical.
2. **The strict gate refuses noise.** Misspelled own names hold holders at fold time and mint strays at bind time. Both are visible; neither is a wrong merge.
3. **Rehomes do not survive a rebuild.** The resolver does not read `org_mention_rehoming` (513 review, finding 4). After a from-archive rebuild, a rehomed mention lands on an `identity-stray` row, not on its reviewed home.
4. **Coupling through one lineage.**
   - A `keep` on P disarms 513's alias for P (`canonical.rs:17524-17540`).
   - Any future 513 cohort on P must now list child holders in `reviewed_holders`.
5. **Moved children have no date fence.** Old-name mentions published after 2026-04 join the legally-ended body (513's accepted trade-off). New-name mentions become strays beside the 2026 five-character ICB orgs.
6. **469 U7 inversion** for any child that is held or a stray.
7. **Stack.** `plan_nhs_bodies_on`, `resolve_one_mention` and `run_match_identifiers_spec` (40 bytes of headroom, `supervisor.rs:6210-6211`) all grow. Box every new arm.
8. **The shared bind-helper refactor.** 513's alias tests guard it.
9. **Purity may hold more than expected.** Country-less same-literal mentions that R3 merged in would make a holder impure (M3).
10. **After strays regrow, `?identifier=GBNHS<C>` answers the survivor plus each stray** (the direct leg plus the merged leg, `read.rs:4335-4357`). That is truthful for a code two bodies publish, but S3 holds only at fold time.

## Owner rulings needed

1. Adopt the one-lineage extension: nhs-body verdicts may name holders of child codes; the census widens to P's child holders; holds are made by omission, never by `keep`.
2. The strict all-names gate, with noise holding the holder (default). The alternative is a narrow reviewer attestation, allowed only for `zero` or `coverage` refusals where no rival fits. Decide after M1.
3. 13T: rehome the 15 CSU-named mentions to 0AR's survivor 30929531 and fold the 481 NENC-named ones into QHM, or hold 13T whole. M7 decides whether the CSU acted as agent.
4. Under a child code, names the gate admits as a predecessor-CCG name fold into the ICB (default: yes, per 513's predecessor rule). Moved children get no date fence (default: yes).
5. Anchor parents (14 orgs / 291 mentions):
   - hold (default);
   - the identity survivor: the child's `GBNHS` row absorbs the anchor. This stays within D2, but it flips 13 applied survivors and needs an anchor-chain follow in `nhs_body_target_org`;
   - D2′: canonical into provisional, which needs a new 300 amendment.
6. Build U6, identity bodies for digit parents (8 orgs / 30 mentions plus 00X), after U5, or skip it.
7. Trust sites: post the gate-admitted, trust-named sites under PPON-survivor trusts in the same cohort (default), or defer all of them.
8. NHS→NHS typos: 452 `wrong` only (default), or U8's `GB:nhs` rekey key. PPON-correction moves: hold (default), or fix the spelling in U8.
9. Record `GBNHS<00x>` on the survivor for zero-only children (lookup only; lost on a rebuild).
10. The scoped unwind is the default for the child cohort, with last-in-first-out refusal; a code-wide unwind is the full rollback.
11. File NHS England (class C: X24, Y56, Q4x/Q7x, Y5x/Y6x, 03Q, 14N) as its own issue.

## Measure first

All of these are bounded `/v1/sql` reads or offline work.

- **M1.** For each of the 52 sub-ICB orgs, the 25 sites and the zero-only clusters: every distinct mention name × `notices.published_at`, scored with a Python port of `child_check` (extend `513-nhs/u1-table/check_sim.py`) under each dated parent. Split the results into all-admitted, other-body strays (rehome candidates) and own-name noise. Before U2. This decides ruling 2 and the S1 numbers.
- **M2.** Run the generator: refetch 03K first, then all 344 RO98 rows and the measured RO198 codes. Count children per allowlisted parent, overlapping intervals and codes with no parent.
- **M3.** Purity and flags per child holder:
  - raw identifiers (country-less or foreign);
  - merged identifiers (PPON);
  - `e2-altid` and `r2` ledger rows;
  - 452, 317 or case flags.

  This gives the counts for `child-impure`, `child-carries-ppon` and `child-withheld`.
- **M4.** Child-coded mentions per month over the last 12 months, and the share published after P's end: the alias load and the expected stray rate.
- **M5.** The zero-only tails' holders (00P 6, 00N 1, 00L 1, 00X 2): whether each is standing and provisional, its names, and whether a 513 alias key already binds it.
- **M6.** A dry `rekey-plan` after posting QHL15E's PPON correction: the shape must be `merge` into 30916123. Also list every COH and PPON correction's shape, and hold the PPON moves.
- **M7.** A notice read of 13T's 15 CSU-named mentions (agent or buyer), and of each rehome candidate's notice, to fix its target org.
---

## Owner rulings (2026-10-12)

The design panel was wf_98dc79c8-57c: three designs (extend-513, separate-arm, identifier-verdicts), two judges
(adversarial safety; evidence and coverage), then a synthesis. **Both judges picked extend-513.** It is the only design
whose partition of the 52 sub-ICB orgs adds up and matches the census (22/945 folded, 13/290 anchor-held, 8/30
digit-held, 9/579 rest), and the only one whose 13T call matches the mention names (481 NENC, 15 CSU). The
identifier-verdicts design had fatal flaws: it rekeys 13T whole into the CSU, and it folds canonical GBNHS rows into
provisional anchors with no 300 §10 amendment.

1. **Adopted:** the one-lineage extension. Holds are made by omission, never by `keep`.
2. The strict all-names gate is the default, and noise holds the holder. Revisit after M1 with its numbers.
3. 13T: decided after M7. If the 15 CSU-named mentions are the CSU acting as its own party, rehome them to 0AR's
   survivor 30929531 and fold the rest into QHM. Otherwise hold 13T whole.
4. Yes: predecessor-CCG names the gate admits fold, and moved children get no date fence.
5. Anchor parents are held (the default).
6. Identity bodies for digit parents (8 orgs / 30 mentions plus 00X) come after U5, as a separate decision.
7. Trust sites: the gate-admitted, trust-named sites under PPON-survivor trusts go in the same cohort.
8. NHS→NHS typos get a 452 `wrong` only. PPON-correction moves are held.
9. Yes: `GBNHS<00x>` is recorded on the survivor for zero-only children.
10. Yes: a scoped unwind is the default, with last-in-first-out refusal.
11. NHS England (class C) is filed as its own issue: 523.

NEXT: measure first, M1–M7 (offline work or bounded reads; M1 and M7 decide rulings 2 and 3), then the units in §7.

## Rulings 2 and 3, decided after measure-first (2026-10-12; `measure/results.md`, wf_45bc31fa-ae6)

- **Ruling 2: strict all-names gate.** The attestation alternative is dropped. In phase A the strict gate holds
  1 holder / 1 mention (31615636, "…North Cumbira ICB", a `tie{QMF}` the attestation could not reach either), and
  both options fold the same holders. S1 does not trip.
- **Ruling 3: first branch.** M7: NECS appears as its own party in the buyer slot, with NENC beside it, in 14 of 15
  notices. So:
  - rehome the 15 CSU-named 13T mentions to 0AR's survivor 30929531;
  - fold the 488 NENC-named mentions into QHM.
  This is conditional on two things settling before 13T joins: the 16th mention (46991199), and the head-name clause
  (13T's head name is the CSU's, which E5's head-name gate checks). If either stays open, 13T is held whole. The
  cost is 488 mentions and nothing else.
- **Plan changes the measurements force** (folded into the build):
  - E5's head-name clause for identity holders;
  - the two wrong pins (04Y answers `zero`; College Road answers `non-nhs`);
  - zero-only members drop to 3 holders / 3 mentions (00P/00N already arrived through 513's folds);
  - U6 needs a PPON tier (QE1 has a standing PPON org 31569664 beside 30915463);
  - M6 moves into U3.
- **Phase-A fold, measured:** 34 holders / 1,475 mentions after 26 rehomes, or 33 / 987 without 13T. The child map is
  M2's `measure/child-map.json`: 0 overlaps, 03K under QOQ, 162 codes with no RO261 parent.
