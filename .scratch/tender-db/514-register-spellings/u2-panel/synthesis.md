# Issue 514, unit 2: decision draft

**Basis.** Both judges picked the **minimal** design, so this draft is built on it. From architecture it takes the rule that a wet run executes only what the dry run listed, the 1/5 floor for stray names, and its guard and end-to-end tests. From registry it takes the register check of every mover, merged identifiers that keep old spellings findable, a capped first wet run, and its two findings, which are filed as unit-3 prerequisites.

**What I re-checked.** I re-verified every file:line below against HEAD `bdbb8ed`. `crates/ingest/src/project.rs` has another agent's uncommitted hunks in the working tree, so its line numbers are HEAD's.

**Reach, re-run over census `raw/001-004`.**
- I re-ran minimal's rules: **54 rewrites**, 25 of which land on a standing twin. The other 29 are 27 lone rows plus 1122831's two rows, which reunite with each other.
- **All 54 satisfy I1** (§1.2).
- All 54 movers carry only their polluted spelling as mention raws (raw/018-029). So the 345 witness sampler's `LIMIT 5` (canonical.rs:31437-31441) cannot miss any of them.

### 0. How the judges' flaws are removed

| Flaw named by the judges | How this draft removes it |
|---|---|
| E0 wet runs are bound only by parity of max(2%, 50) (canonical.rs:21462-21471). The 345 wet run is bound only by max(2%, 5) (canonical.rs:31514-31524). | **G1.** E0 dry runs also store a complete listing under a new report, `e0-merge-listing`. A wet run merges only live groups whose key **and** member set match that listing, the rekey arm's "live ∩ stored" rule (supervisor.rs:9750-9782; canonical.rs:29257-29263). **G2.** The 345 wet run moves only the `(org, from, to)` rows of the stored dry plan, and refuses a plan with `plan_truncated`. The first E0 wet run is capped, always. |
| 27 lone rows are moved in place with no name check, including 8 IP+R rows. | **G3.** Before the gate, all 54 movers are checked against the register record for their canonical number. A mismatch puts that canonical number on a hold list, `REGISTER_SPELLING_HOLDS`, in `register_spelling`. The rewrite never produces a held number, and a test pins each entry. |
| The window between re-key and merge, when two rows share a triple and the alias is refused as `Shared` (canonical.rs:17899-17906), rests on discipline alone. | **G4.** The exposure is measured: exactly 5 canonical holders are e2-altid register targets (Clarion, Metropolitan, Creative Support, Hallam, Cambridge; `469-registry-pairs/u6/verdicts-confirmed.json`). Each is the lower id, so each survives (canonical.rs:21358-21362). Mentions resolve only in queued `project` jobs, run one at a time (ADR 0007:4). So the re-key wet run, the E0 dry run and the E0 wet runs go into one daily-pipeline gap (the scheduler fires at 09:35 Berlin, supervisor.rs:14459), with verdicts posted beforehand. The 448 "stand on two or more rows" log line (canonical.rs:17401-17418) is a post-check. |
| Polluted literals stop answering `?identifier=`, because an E0 merge of identical triples writes no merged identifier (canonical.rs:4778-4781). | **G5.** A 345 move records its pre-image through `record_alias_identifier_as(…, "renormalise")`, with loser = holder. That shape is already established by 460 D5 and 513 U4 (canonical.rs:4914-4955). The E0 merge then carries the row to the survivor (canonical.rs:4723-4729, 4774). |
| The undo claim is overstated: ADR 0003:59-61 is about the link ledger, and E0 merges are physical repoints (canonical.rs:4617-4624). | §5.4: moves are undone from their stored pre-images. Merges are undone from a pre-merge snapshot of each loser's mention primary keys (canonical.rs:452), with an unwind specified in 513 U4's shape (canonical.rs:27228). |
| W6 changes the E0 wall in every country, and it can tighten as well as loosen. | W6 is scoped to **GB register identities, rule `e0`**. The injected strip is `crosswalk::registry_mention_key` (crosswalk.rs:1242-1252), so it cannot act outside the five tokens. Its footprint in both directions is a stop rule (S9a). |
| The E0 name rule's denials are invisible (canonical.rs:21224-21227 lists nothing). | E0 dry runs list name and wall denials, in the shape of R2's 4c listing (canonical.rs:21253-21277). |

What is **not** grafted, and why:
- **Registry's normaliser-level unifications** (NI strip, `-0`, generic labels, IP0/RS, leading-zero G rules): each judge called them name-blind and able to reach traps.
- **Architecture's widened rule-3b names and its rule-4b scoping**: they loosen 470's live gate.
- **Any move of an RS or NIC alias target**: this would orphan the Connexus alias.

## 1. Canonical form per scheme

### 1.1 Where the rewrite acts

The rewrite is `fn register_spelling(value: &str) -> Option<String>`, in `crates/ingest/src/project.rs`, called from `normalise_identifier_with` (project.rs:8961). It fires only when all of these hold:
- **`folds == true`.** `normalise_identifier_before_folds` (project.rs:8952-8959) then still reproduces the stored value, which is the witness the 345 job needs.
- **Country is GB.** `country` after `register_jurisdiction` (project.rs:8969) must be `Some("GB")`.
- **Position in the pipeline.** It runs after the ASCII filter (project.rs:8996-9000) and the filler checks (project.rs:9002-9012), and before the 328 label block (project.rs:9014-9078). That block has no `G` entry (crosswalk.rs:384-386), so the reorder changes no other value.
- **Kind stays national.** The output goes through the existing VAT/national test, and the letter run after `GB` keeps it national (project.rs:9139-9157).

### 1.2 Invariants, checked inside the function and pinned by tests

- **I1.** The number after the token fails that token's `REGISTRIES` shape on input and passes it on output (crosswalk.rs:1208-1227). The shape closures are exposed as `pub(crate) fn registry_number_shaped(token, number)`, so there is one table.
  - Every 469 ledger identity passed that shape (469 U4).
  - So **no e2-altid register target can move**. I checked both non-canonical targets, `GBMPRRS008376~PZTX9645HPZJ` and `GBNICNI100770~PLPQ1249DXWG`: both pass the shape, so I1 leaves them alone.
- **I2.** The output keeps its scheme token. The GB arm strips a bare `GB` (crosswalk.rs:400-404). It then keys 8 digits as `GB:coh` E1 (crosswalk.rs:422-425) and pads 6-7 digits to E2 `GB:coh` (crosswalk.rs:428). So a token-less output would become a company number.
- **I3.** No digit is added, removed or changed, and no `R` or `RS` is added or removed.
- **I4 (new).** The output is not on `REGISTER_SPELLING_HOLDS`. The list is expected to be empty (§3.3).

### 1.3 Rules

Each rule acts on the upper-cased alphanumeric value. For each scheme, any input not matched stays as published.

**UKPRN**
- **Rule:** `^GBUKPRN(?:(?:GB)?UKPRN)+(1\d{7})$` → `GBUKPRN$1`.
- **Refuses:**
  - a dropped or doubled zero: `100007158`, `100007167`, `100000747`, `100009085`, `1007141`, `1000457`, `100026620`, with or without the label;
  - 11-digit values, 6-digit URNs, a PPON, 0-led company numbers;
  - any other label word.

**CHC**
- **Rule:** `^GBCHC(L)(\d+)$`, with L tried longest first from {`REGISTEREDCHARITYNUMBER`, `REGISTEREDCHARITYNO`, `REGISTEREDCHARITY`, `CHARITYREGISTRATIONNUMBER`, `CHARITYREGISTRATIONNO`, `CHARITYCOMMISSIONNO`, `CHARITYREGNO`, `CHARITYNUMBER`, `CHARITYNO`}. `$2` must be CHC-shaped. Output `GBCHC$2`.
- **Refuses:**
  - **generic labels:** `NO`, `REGISTRATIONNO`, `REGISTRATIONNUMBER`, `REGCHARITY`. A company "registration number" that lost its zero passes the 7-digit shape;
  - **compounds:** anything after the number;
  - **leading zero:** `0518801`;
  - **glued `-N`:** a 6-digit number plus `-N` gives 7 digits starting 2-9, and a 7-digit number plus `-N` gives 8 digits. Both fail the shape (crosswalk.rs:1210-1212), so no raw literal is needed;
  - **HMRC references:** `X1089`, `XC1089`, `XN85208`, `XR6326`;
  - **other numbers under GB-CHC:** `RC…`, `RSA007814`, FCA numbers (`IP14044R`, `30224R`, `IP031693`, `17434R`), `SC`/`OC`/`CE` company numbers, 8-digit company numbers, a PPON.

**NIC**
- **Rule:** `^GBNIC(CHARITYCOMMISSIONNO|CHARITYNUMBER|CHARITYNO)((?:NIC|NI)?1\d{5})$` → `GBNIC$2`. The published `NIC`/`NI`/bare spelling is kept.
- **Refuses:**
  - unifying `NIC`, `NI` and bare (U3 keeps three identities, crosswalk.rs:1205-1207, and no split was measured);
  - `101486-0` (Triangle, a finding for unit 3);
  - CHC- or FCA-shaped values (`216250`, `1014860`, `32427R`);
  - `FH363`;
  - a generic `NO`.

**MPR (a)**
- **Rule:** `^GBMPRIP([1-9]\d{4}R)$` → `GBMPR$1`. This is Companies House's exact 8-character society form.
- **Refuses:**
  - `ip6243r` (7 characters: Hastoe's typo of 16243R, org 31563025, 30 mentions);
  - `IPI6637R`, `IPR16337R`, `IPSNO27793`, letter O (`2744OR`);
  - `IP0NNNNN`, where the R would have to be inferred (Guinness Developments' `IP031693`);
  - `IP07522R`.

**MPR (b)**
- **Rule:** `^GBMPRREGNO([1-9]\d{2,4}R)$` → `GBMPR$1`.
- **Refuses:** `REGNO` before a bare or `RS` number, and other label words.

**MPR: deferred to unit 3.** No rule maps `RS00NNNN` to `NNNN`, and no rule maps bare to `R` (7522 is not 7522R). See §4.

**SC.** No rule. This refuses `GBSC`+digits (minted as a VAT), `SCSC…` company numbers and `SCO…` lookalikes.

### 1.4 Effect on the census (my replay)

**Rewrites.** 54 rows: UKPRN 6, CHC 28, NIC 1, MPR 19 (18 IP, 1 REGNO).

**Groups.** 26 E0 groups: 25 reunions onto a standing twin, plus 1122831.

**Lone moves.** 27 rows:
- 20 CHC label rows;
- 1 UKPRN (`GBUKPRNUKPRN10007339`);
- 8 IP+R rows, the ones G3 targets:
  - 30975000 `IP17704R` Your Housing;
  - 31567617 `IP21180R` Cadarn;
  - 31573433 `IP29840R` Local Space;
  - 31575842 `IP30178R` Incommunities;
  - 31600628 `IP27566R` NDTi;
  - 31601449 `IP19229R` Accent;
  - 31609584 `IP26511R` Two Saints;
  - 31613230 `IP16243R` Hastoe, the correct spelling.

**Closed.** 26 of 36 splits, and 743 of 852 mentions (minimal's figures, which judge 2's replay reproduced).

**Deferred.** 10 splits, 109 mentions:
- RS 4 (89 mentions, 75 of them Connexus);
- compounds 3;
- generic labels 2 (1003906, 1179472);
- leading zero 1.

**Untouched.** Every trap org: 31563025, 31552235, 30917450, 31547203, 31611321, 31600646, 31614503, 31605441, 31623228, 31536226, 31613164, 31587933, 31550331, 31596036.

## 2. Where it runs, and why

### 2.1 Prevention: the ingest normaliser

- **Register identities have no canonical key.** The GB arm keys none of them (crosswalk.rs:429-433, pinned at crosswalk.rs:747-756), so they bind by exact triple only (canonical.rs:18630-18634). The normaliser is therefore the only place that makes a spelling exact: CONTEXT.md:66-68 says mentions auto-merge on exact identifiers.
- **470's key-level route** (`CanonKey::folded`, crosswalk.rs:45-60; "proposes … never proves", crosswalk.rs:456-457) would make registers E1. That would move every exact-duplicate register group into R2, and push register keys into `mention_key` (crosswalk.rs:1153-1156) and `canon_of`. Both 448 and 469 refused that.
- **Binding without a name check adds no new risk.** By I3, a rewritten literal binds exactly where the publisher's bare spelling of the same characters would bind.

### 2.2 Stock repair: the 345 job, with two changes

**Why the job is needed at all.**
- Recorded mentions never re-resolve: `publishes()` compares raw strings (canonical.rs:11676-11681), and resolution returns early on a match (canonical.rs:18624-18625).
- `Spec::RepairRenormalisedIdentifiers` (supervisor.rs:7823) already pairs `before_folds` with the live normaliser (supervisor.rs:7854-7862).
- The witness, "the old rules reproduce the stored triple", holds for every polluted row (canonical.rs:31448-31455), because the rewrite sits behind `folds`.

**Change G2: the wet run is bound to the stored plan.**
- Today the wet run reads only `rows` (supervisor.rs:7834-7851), recomputes the plan and moves it all (canonical.rs:31514-31575).
- New: `repair_renormalised_identifiers` takes `expect_plan: Option<&HashSet<(org, from triple, to triple)>>`, read from the stored dry plan (supervisor.rs:7874-7895). A recomputed row outside that set is counted `held_unplanned`.
- The supervisor refuses a stored plan with `plan_truncated`. `PLAN_CAP` is 400 (supervisor.rs:7874).

**Change G5: the pre-image stays findable.** After each guarded UPDATE (canonical.rs:31560-31575), the job calls `record_alias_identifier_as(&conn, &Identifier{from…}, org, "renormalise")` (canonical.rs:4929-4955). The new row has loser = holder.

**Why not the rekey arm (453).** It finds owners through `canonical_key_flat`, which keys no register identity. Its `wrong` verdicts withhold the org, and a moved rekey loser records no literal (canonical.rs:4775-4777).

### 2.3 The merge: R2's E0 rule, with three changes

E0 already gives:
- exact-triple groups through `e0_key_flat` (crosswalk.rs:876-893; single caller at supervisor.rs:9381);
- the 362 verdict lane (canonical.rs:21136-21160);
- `org_merge_log` rows with rule `e0` and both literals (canonical.rs:21577-21589).

**W6, the wall fix (verified).**
- **Why every reunion is denied today.** The wall infers kind `vat` for every raw with a two-letter lead (canonical.rs:21311-21316), and strips only a raw keyed equal to the group key (canonical.rs:21322-21327). All E0 evidence sits in one `"E0"` scheme bucket. So `vat:GBUKPRNUKPRN10007798` and `vat:GBUKPRN10007798` are disjoint, and the group is denied as `denied_group_vat` (canonical.rs:21339-21350). A verdict cannot lift this, because it replaces only the name rule (canonical.rs:21171-21173).
- **The fix.** `R2MergeArgs` (canonical.rs:5312-5351) gains `register_identity: Option<fn(Option<&str>, &str) -> Option<(&'static str, String)>>`.
  - The E0 arm passes `Some(ingest::crosswalk::registry_mention_key)`; every other constructor passes `None`.
  - In the wall, under rule `e0`, a raw is skipped when `registry_mention_key(country, raw)` returns the group's own identity, that is `national:<identity>` equals the group key.
  - This extends catch (a) (canonical.rs:21285-21290) to the publisher's other spellings of the same register key.
- **Effect.** It removes only evidence that agrees with the group. Two different PPONs on two members now **deny**, where today the shared register raw masks them. So the subsidiary shape is walled.

**G1, a bound wet run.**
- E0 dry runs write `e0-merge-listing`: every planned `(country, scheme, key, members)` and a `truncated` flag. Today's listing is dry-only and capped at 500 (canonical.rs:21401-21425, 9499).
- A wet run passes it as `expect_listing`. A group is merged only on an exact (key, sorted members) match; otherwise it is counted `held_unplanned`.
- A truncated listing falls back to today's parity, and the summary line says so.
- Wet re-records of `e0-merge-plan` (supervisor.rs:9417-9474) no longer clobber the listing, so a capped continuation stays bound.

**Denial listing.** E0 dry runs list name-rule and wall denials, capped, in R2's 4c shape (canonical.rs:21253-21277).

No new Spec arm, ledger rule or job is added. Both arms touched are already `Box::pin`ned (supervisor.rs:7823, 9338).

## 3. The name gate

### 3.1 Automatic backstop, unchanged

- E0's rule (canonical.rs:21174-21227) reads head names only, through `org_health_meta` (canonical.rs:20967, 35327-35340).
- The vetoes before the verdict lane stay as they are (canonical.rs:20939-21126): literal cap, gate-poison, consortium, legal form.
- On its own it would deny 11 of the 26 groups (minimal's estimate), which is the safe direction. With verdicts pre-posted, it decides only groups nobody foresaw.

### 3.2 The real gate: a reviewed verdict on every group

- **Shape.** One verdict per group, keyed `(GB, E0, national:<canonical>)`, members = the exact post-re-key set, ascending, cohort `514-spellings-<date>`. It can be posted before deploy: `record_merge_verdicts` validates shape only (canonical.rs:29045-29094, 8414-8438). A `keep` is read first (canonical.rs:29096-29124). A HIGH `merge` on exactly the live set admits the group (canonical.rs:21153-21156).
- **Inputs, per member:**
  - the head name and `organization_names`;
  - every mention name with its count, from one bounded `/v1/sql` call (`SELECT organization_id, name, COUNT(*) FROM organization_mentions WHERE organization_id IN (…) GROUP BY 1,2`, census call-18 shape, served by `organization_mentions_org`, canonical.rs:455);
  - the register record for the canonical number: FCA Mutuals, the CC extract (including `linked_charity_number`), UKRLP or CCNI, through 469 U2's fetchers. The cached `mpr.json` holds only 16243R, 28038R, 16337R, 27440R and 30938R, so the rest are fetched.
- **The rule a reviewer and a challenger each apply: merge, HIGH, only if all hold.**
  - (a) The register names one holder for the number.
  - (b) For every member, more than 50% of its mentions, weighted by count, name that holder, a former name on the register, or a sourced brand or service of it.
  - (c) No member has a separate legal entity's name ("… Trustees Limited", "… Developments Limited", a trading subsidiary) on 1/5 or more of its mentions. Below 1/5 a name is a stray: Hallam's single "University of Sheffield" out of 182 mentions, 10007157's single Hallam mention out of 16.
  - (d) The two readers agree.
- **Otherwise `keep`.** A tie also means `keep`.

### 3.3 The same check covers lone movers (G3)

- **Input.** Each of the 27 lone movers, and each mover inside a group, is read against the register record for its canonical number, using rule (b).
- **On a mismatch:**
  - the canonical number goes onto `REGISTER_SPELLING_HOLDS` before the gate;
  - the rewrite never produces it, so new polluted mentions stay as published;
  - a test pins the entry.
- **Escalation.** Two or more mismatches stop the unit and send the rule back for redesign. One mismatch means the hazard (a subsidiary on its parent's number) exists in-scheme, which the census measured at 0 in 36.

### 3.4 Why no code-level all-names gate in this unit

- **It is needed only once.** After deploy, every polluted spelling binds to the survivor's exact triple, so the gate runs once, over 26 groups.
- **Changing the code rule would move every country.** E0's head-name rule decides every E0 group in every country (issue 329's class), and widening it is that movement.
- **518's regrowth asymmetry cannot arise.** It needs an alias, and no alias is involved here.

## 4. The Companies House mirror (GBCOHIP/GBCOHRS against MPR): out of this unit

1. **`GBCOH…` values are E1 `GB:coh` keys** (crosswalk.rs:417-425). Rewriting them would take societies out of the Companies House series, and out of the reach of 448's `canon_of` alias (canonical.rs:17883-17893).
2. **Every subsidiary-on-parent trap is cross-prefix:** Guinness 31693R, Homes Plus `GBCHC30224R`, IDS Trustees `GBCHCIP14044R`, plus 29996R, 28985R, 24218R and 31218R. Unit 2's rules are token-scoped, so they never read them.
3. **`IP0NNNNN` → `NNNNNR` infers an R** that the publisher never wrote.
4. **`RS00NNNN` → bare is deferred even in-scheme.** `RS`+6 digits passes `REGISTRIES` (crosswalk.rs:1220). The alias keys its targets on ledger identities verbatim (canonical.rs:17330-17340) and binds them by exact `org_of` (canonical.rs:17899-17906). Moving Connexus 31536226 would read `NoOwner` and re-mint the PPON org that has 63 mentions.
5. **It is a different scale:** at least 3,238 more mentions over at least 85 groups (unit1-verify F3/F4).

**File as "FCA society ↔ Companies House mirror", a 469 T/515-shaped arm.** Prerequisites:
- (a) Canonicalise ledger identities when the alias is armed. Two U6 targets are non-canonical: `GBMPRRS008376~PZTX9645HPZJ` and `GBNICNI100770~PLPQ1249DXWG`, both verified.
- (b) Triangle publishes `GB-NIC-101486-0` (raw/028). That is a CCNI number with sub-number 0, not a CHC number.
- (c) FCA confirmation pair by pair.

## 5. Repair path, dry plans, stop rules

### 5.1 Sequence

Everything runs in-app, dry first, and each plan is read before its wet run.

1. **P0, on the current revision.**
   - `repair-renormalised-identifiers` dry: **S1**, `rows` must be 0.
   - `match-org-identifiers {"rule":"e0"}` dry: record B0. If B0 > 0, drain it under its own review first.
   - List the existing verdicts on the 26 keys. A standing `keep` from any cohort denies its group.
2. **Review (§3).** Post the 26 verdicts and fix the hold list. **S8:** every verdict is posted, and the hold list is frozen before the gate.
3. **Deploy U2.2 and U2.3 together, in a queue gap,** right after a daily pipeline completes.
4. **345 dry.** Read it against S2-S6. Then **E0 dry, giving B1.** S9a: B1 − B0 is W6's footprint before any re-key.
5. **The same pipeline gap (S7).**
   - Run the bound 345 wet, then E0 dry immediately, giving B2. Check S9b.
   - Snapshot every loser's mention keys: `SELECT notice_id, section_id, organization_id FROM organization_mentions WHERE organization_id IN (<26 loser ids>)`, bounded and index-served.
   - E0 wet with `max_groups: 5`, then the S10 read.
   - E0 wet in full, bound by `e0-merge-listing`.
6. **Verify.**
   - The 26 canonical values each answer exactly one org.
   - Each loser answers 308 to its survivor.
   - `?identifier=GBUKPRNUKPRN10007798` returns the survivor with `merged_into`, per docs.rs:272.
   - The next fold's 448 "register target(s) stand on two or more rows" count is 0 (canonical.rs:17401-17418).
   - The altid dry run's `registry_regrown` is empty (canonical.rs:24974).
   - Repeated dry runs plan 0 on both jobs.

### 5.2 Contents of the dry plans

**`renormalise-repair`** (fields from supervisor.rs:7875-7895):
- `walked`, `witnessed`, `unexplained`, `already_clean`;
- `now_refused` = 0;
- `rows` = 54 ± rows minted since 2026-10-11;
- `reunions` = 25;
- `plan_truncated` = false;
- `plan[{org, from, to}]`.

**`e0-merge-plan`, today's fields plus:**
- `plan[]` with each group's members, which must be the 26 groups minus the keeps;
- `admitted_verdict` = 26 minus the keeps;
- `denied_verdict` = the keeps;
- `verdict_stale` = 0;
- the new denial listing;
- `held_unplanned` on wet runs.

**`e0-merge-listing`:** complete, `truncated` false.

### 5.3 Stop rules

- **S1.** The 345 baseline (dry run on the current revision) plans 0 rows.
- **S2, scope.** Every `from` is GB, `national`, under `GBUKPRN`, `GBCHC`, `GBNIC` or `GBMPR`, and its `to` is the rule's output.
- **S3, I1.** Every `from` fails the shape and every `to` passes it.
- **S4, size.** At most 2 × 54 rows, and never more than 400.
- **S5, completeness.** Every expected mover appears in the plan. A census mover cannot miss its witness (measured), so a missing row is new drift: investigate it, and never add rows to the plan by hand.
- **S6, traps.** None of the trap org ids in §1.4 appears.
- **S7, window.** No `project` job runs between the 345 wet run and the full E0 wet run. If one does, re-run E0 dry, read the 448 log line, and only then go wet.
- **S8.** Verdicts are posted and the hold list is frozen before step 5.
- **S9a.** B1 − B0 contains only GB register E0 groups. Anything else is a defect in W6's scope.
- **S9b.** B2 − B1 equals the expected groups. Each shows `admitted_verdict` or `denied_verdict`, and each survivor is the lower id (canonical.rs:21521-21525).
- **S10, capped read.**
  - Each loser answers 308 to its listed survivor.
  - Each survivor holds the canonical value.
  - Each old spelling resolves through merged identifiers (read.rs:4275-4297).
  - Mentions are conserved.
  - One wrong merge halts the run, and the unwind gets built.
- **S11.** Two or more register mismatches in §3.3 stop the unit.

### 5.4 Reversibility

- **Moves.** Each move has a stored pre-image (the plan is never truncated), a change event (canonical.rs:31574) and a merged-identifier row.
- **Merges.** Each merge has an `org_merge_log` row with `keep_id`/`loser_id` literals (canonical.rs:21577-21589) and the pre-merge mention-key snapshot.
- **The unwind.** It re-mints the loser with its pre-image triple and moves exactly the snapshotted mentions back, in 513 U4's shape (canonical.rs:27228). It is specified now and built only if S10 trips.
- **What a code revert does.** On its own it undoes nothing: after a revert, the witness reads the moved rows as `unexplained`.

## 6. Tests that pin each trap

### `crates/ingest/src/project.rs`

- `register_spellings_fold_to_one_identity_per_scheme`: every census literal among the 54. Examples: `GB-UKPRN-UKPRN 10007798`, `GB-UKPRN-GB-UKPRN10004116`, `GB-CHC-registered charity (no. 1003906)`, `Charity Commission No NIC102575` → `GBNICNIC102575`, `GB-MPR-IP28038R`, `GB-MPR-Reg No 30938R`.
- `every_rewrite_moves_from_unshaped_to_shaped`: I1, I3, and idempotence (normalising the output again returns the same value).
- `typo_spellings_of_another_number_stay_as_published`:
  - `ip6243r`, `IPI6637R`, `IPR16337R`, `IPSNO27793`;
  - `RSA007814` under MPR and under CHC;
  - `RS0076173`;
  - the 7 UKPRN digit-count typos.
- `hmrc_references_under_gb_chc_are_not_charity_numbers`: `X1089`, `XC1089`, `XN85208`, `XR6326`, `Charity No X1089`.
- `subsidiary_literals_on_a_parents_number_are_untouched`: `GBMPRIP031693`, `GBCHCIP031693`, `GBCHC30224R`, `GBCHCIP14044R`, `GB-COH-IP031693`.
- `bare_and_r_suffixed_society_numbers_never_meet`: `7522` / `7522R` / `REGNO7522` / `IP07522R` / `RS007522`.
- `glued_linked_charity_suffixes_are_never_stripped`: `1099485-2`, `1169762-2`, `Registered Charity No. 216250-1`, `Charity Number 1099485-2`.
- `generic_labels_compounds_padding_rs_and_nic_spellings_are_deferred`: `NO`, `REGISTRATIONNO`, the 3 compounds, `0518801`, `RS008376`, `NI100770`, `101486-0`.
- `the_rewrite_keeps_the_token_so_no_company_key_appears`: `canonical_key_flat == None` for every output, which covers crosswalk.rs:428.
- `the_rewrite_never_reads_another_token_or_a_bare_legacy_literal`: `GBCOHIP16791R`, `IP16791R`, `RS007843`, `CHARITYNO298028`, `NIC100166`, `SC005336`, `SCO13683`.
- `the_register_fold_is_gb_only`: the same literals under `XI`, `IE` and `None`.
- `before_folds_keeps_the_published_register_spelling`: extends project.rs:12789-12791.
- `a_held_number_is_never_rewritten`: through `register_spelling_with(value, holds)`.

### `crates/ingest/src/crosswalk.rs`

- `label_spellings_are_refused_not_stripped` (crosswalk.rs:1660-1671) becomes `label_spellings_key_as_their_canonical_identity`. `reg. charity 1044521` stays `None`.
- `a_glued_linked_charity_suffix_fails_the_chc_shape` (crosswalk.rs:1648-1655) is unchanged.
- `gb_other_registers_and_vats_key_nothing` (crosswalk.rs:747) gains the canonical outputs.

### `crates/store/tests/e0_merge.rs`

- `a_register_spelling_reunion_is_walled_today_and_passes_on_its_own_identity`. With `None` the result is `denied_group_vat` = 1. **Write this half first.**
- `a_reunion_member_naming_another_number_still_hits_the_wall`: two PPONs, and a second UKPRN.
- `the_own_identity_strip_is_scoped_to_gb_registers_and_rule_e0`.
- `a_head_name_split_is_denied_and_a_high_verdict_admits_it`: the 10007157 shape, plus `keep` and stale-member cases.
- `a_wet_run_merges_only_listed_groups`: an unlisted group, member drift, and a truncated listing that falls back to parity.
- `e0_dry_lists_name_and_wall_denials`.

### `crates/store/tests/renormalise_repair.rs`

- `a_register_label_spelling_moves_onto_its_bare_twin`: the `ip6243r` and `RS008376` rows stay; a second pass plans nothing.
- `a_wet_run_moves_only_stored_plan_rows`.
- `a_moved_row_keeps_its_old_spelling_findable`: loser = self, and the row is carried to the E0 survivor.

### `crates/ingest/tests/fts.rs`

- `GB-UKPRN-10007798` and `GB-UKPRN-UKPRN 10007798` under one name leave one org.
- `GB-MPR-ip6243r` beside `GB-MPR-16243R` leaves two.

### Must stay green

- The stack canary `an_execute_without_an_expected_count_is_refused` (supervisor.rs:20162).
- `registry_pairs_never_change_the_coh_plan` (altid_merge.rs:2689).
- Every `R2MergeArgs` constructor: 2 in supervisor.rs, plus e0_merge, identifier_verdicts ×3, merged_org, r2_merge and r2_name_gate.

## 7. Units in build order

**U2.0: decision and measurements. No code.**
- Record this decision on issue 514.
- Run P0 (S1, B0, existing verdicts).
- Run the reads in "Measure first".

**U2.1: review campaign. Scratch only, under `.scratch/tender-db/514-register-spellings/unit2-review/`.**
- Mention-name reads.
- Register fetches for all 54 canonical numbers.
- Reviewer and challenger.
- `verdicts.json`, posted through `POST /admin/merge-verdicts`.
- The hold list.
- It must finish before U2.2 is gated, because the hold list is in code.

**U2.2: the rewrite (ingest).**
- `crates/ingest/src/project.rs`: `register_spelling`, `REGISTER_SPELLING_HOLDS`, the hook, the doc on `before_folds` (project.rs:8952-8956), and the tests.
- `crates/ingest/src/crosswalk.rs`: `registry_number_shaped`, the `REGISTRIES` doc (crosswalk.rs:1187-1207) and the tests.
- `crates/ingest/tests/fts.rs`.
- **Contention.** project.rs has another agent's uncommitted hunks. Run `git diff` before staging, or use a worktree.

**U2.3: the merge and the repair (store and app).**
- `crates/store/src/canonical.rs`:
  - `register_identity` and the wall strip;
  - `expect_listing` and `held_unplanned`;
  - the E0 denial listing;
  - 345's `expect_plan` and the `renormalise` pre-image record.
- `crates/app/src/supervisor.rs`:
  - the E0 arm: inject `registry_mention_key`; write and read `e0-merge-listing`; fix the stale comment at supervisor.rs:9382-9383;
  - the 345 arm: pass the plan set and refuse a truncated plan;
  - the other constructors get `None`.
- `crates/store/tests/e0_merge.rs`, `renormalise_repair.rs`, and the other R2 tests.
- `docs/operations.md` rows at 2174 and 2202.
- **Sequencing.** Order after other in-flight canonical.rs work (511 u4, 513 U6).

**Gate and deploy.** One `ops/check.sh` run covers U2.2 and U2.3, and `--features server` matters for supervisor code. Read `GATE-EXIT=`. Then one deploy in a queue gap.

**U2.4: rollout as in §5.**

**Filed for later:**
- unit 3, the mirror and RS work, with §4's prerequisites;
- CHC compounds and the leading zero;
- generic CHC labels, which need a register check;
- NIC/NI/bare unification;
- the SC `SCO` lookalikes;
- near-miss re-keys through 453 (Hastoe `ip6243r`, `RSA007814`, the UKPRN zero typos);
- XI-country register identities.

## 8. Risks

1. **W6's footprint.** It is confined to GB register E0 groups, and both directions show up in S9a. Its tightening of masked PPON conflicts is intended.
2. **The window.** Exposure is 5 PPON aliases for one pipeline gap. S7 holds the window shut, and the 448 log line and `registry_regrown` detect any leak. 469's drain re-merges a regrown pair.
3. **469 consumers move.** `registry_pair_key` now accepts the canonical spellings, so the next altid dry run may plan new register ↔ PPON pairs. They still pass 469's own gates. Diff against `/previous`.
4. **Future mentions bind by triple without a name check.** That is the same risk as a bare spelling. The rules mint no new number, and the hold list blocks disputed ones.
5. **Gaps in the label vocabulary** leave splits standing, which is the safe direction. Each new word is one line plus a re-key.
6. **Both 345 changes apply to future 345 runs too.** Binding and pre-image records are strictly safer. A from-archive rebuild empties merged identifiers (canonical.rs:4916-4918), so moved literals stop answering after a rebuild. This is accepted: a rebuild re-mints canonical spellings, so no split appears.
7. **A wrong verdict.** Mitigated by two readers, the register record, the 1/5 floor, keep on doubt, the capped first run, and the snapshot unwind.
8. **XI-country register identities are untouched**, and their number is not yet measured.
9. **Survivor head names may be the mover's** (for example 17437R keeps "GOLD HILL HA (B2G)"). This is cosmetic.

## Owner rulings needed

1. Adopt the normaliser rewrite, the 345 re-key and the E0 merge (minimal), over the folded E0 key (architecture) and a new arm (registry).
2. Keep NIC spellings as published, with no NIC/NI/bare unification in unit 2.
3. Defer `RS00NNNN` → bare and `IP0` to unit 3, leaving 4 splits and 89 mentions standing, with alias canonicalisation as a prerequisite.
4. Use a reviewed verdict on every group as the gate (majority of weighted mention names, a 1/5 floor for conflicting entity names, the register record), with no code-level all-names gate in this unit.
5. Scope W6 to rule `e0` and GB register identities through `registry_mention_key`.
6. Bind every E0 wet run to `e0-merge-listing` when the listing is complete (proposed), rather than as an opt-in.
7. Bind the 345 wet run to its stored plan, and record the pre-image under rule `renormalise` for all future 345 moves.
8. The hold-list mechanism for register mismatches, and S11's stop at two or more mismatches.
9. Defer generic CHC labels, compounds and the leading zero.
10. Leave the unwind specified but unbuilt, relying on the snapshot plus S10.
11. Drain an existing E0 backlog (B0 > 0) under its own review first.
12. Keep XI out of scope until measured.

## Measure first

1. **Write the failing E0 wall test.** If it does not fail, drop W6.
2. **P0 on prod:** 345 dry `rows` = 0, E0 dry B0, and the existing verdicts on the 26 E0 keys.
3. **Register records for all 54 canonical numbers**, through 469 U2's fetchers. Only 5 MPR numbers are cached. Include the 8 lone IP+R movers and CCNI 102575.
4. **A bounded mention-name read** for the 52 group members and the 27 lone movers: the review packet.
5. **Fresh bounded re-reads of the 5 token ranges just before deploy**, plus each new mover's raw mix (all 54 census movers are single-spelling).
6. **A bounded read of XI-country register identities** on `organizations_identity`.
7. **After deploy:** B1 − B0 (W6's footprint), and the 448 log line for registers on two or more rows.