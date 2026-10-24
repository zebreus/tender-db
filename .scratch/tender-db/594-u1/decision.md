# 594 unit 2 — decision D594: pin the Junta's NIF rows to "Junta de Andalucía"; TARC's review mentions stay on the row

Decided 2026-10-24 ~23:10Z by the decision author (velocity: decisive). Inputs: issue 594 (Status: unit 1 sized), `reads.json`,
`read.md` and `summary.json` (the reader's full read of org 2660), the precedents 584 (`584-u2/guard-spec.md`), 572
(`572-triage/decision.md`), 528 U3, 524 (`crates/store/src/head_election.rs`), 513/519, 545/584 rehoming, 456 design 3, D576-1
and 587 D587-2. Code is cited at **HEAD `f482124`**, because deploy #5's build is editing the working tree.

Box reads: **one more `/v1/sql` statement, s7** (`raw/sql/s7_pinned_current.*`, 0.55 s). It was plan-probed first, and the plan uses
only index seeks: `tender_version_parties_org` plus the `tenders` primary key. **SQL now stands at 9 of 10, and GETs at 6 of 15** (I
made none). There was no 408 and no retry, and everything is logged in `requests.log`. Nothing was written on the box, nothing was
POSTed, no cargo was run and no git command was run.

## Decision

- **D594-1, identity: unchanged.** S4111001F is the NIF of the Junta de Andalucía, which is one legal person. Its Consejerías,
  Delegaciones, Direcciones Generales and the TARC are organs of that legal person, with no legal personality and no NIF of their
  own. So every mention that prints the NIF keeps binding the row that holds it. This is 572-L, and 572's invariant I1 (no rule may
  move mentions on name disagreement alone) holds. No mention moves under D594-1 and no party row is re-derived.
- **D594-2, the served head: a reviewed head pin.** It is a new, compiled, reviewed table `REVIEWED_HEADS`, keyed by the identity
  triple. The head election reads it as rule **P0**, ahead of every vote rule. ~~The mint reads it too, so a from-archive rebuild
  keeps the pin.~~ *(Struck by Challenge A2: the mint is not touched. After a rebuild the pin returns through the full
  refresh-org-names run that the 524 runbook already requires.)* It is written once, by the existing `refresh-org-names` wet path,
  with its ledger and its revert. The six rows (five after Challenge A3 drops 22525033):

  | org | triple (country, kind, identifier) | head today | pinned head |
  |---|---|---|---|
  | **2660** | ES, national, `S4111001F` | Tribunal Administrativo de Recursos Contractuales de Andalucía | **Junta de Andalucía** |
  | 22341455 | ES, vat, `ESS4111001F` | Consejería de la Presidencia y Administración Local de la Junta de Andalucía | Junta de Andalucía |
  | 22419088 | ES, national, `S4111001` (check letter lost; the only valid letter is F) | TRIBUNAL ADMINISTRATIVO DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCIA | Junta de Andalucía |
  | ~~22525033~~ | ~~ES, vat, `ES4111001F` (S lost)~~ | Tribunal Administrativo de Recursos Contractuales de la Junta de Andalucía | ~~Junta de Andalucía~~ **not pinned (A3)** |
  | 11044446 | ES, national, `4111001F` (S lost) | Junta de Andalucía. Consejería de Cultura. Secretaría General Técnica | Junta de Andalucía |
  | 22235590 | ES, national, `A01034437` (TARC's own DIR3) | Tribunal Administrativo de Recursos Contractuales | Tribunal Administrativo de Recursos Contractuales de la Junta de Andalucía |

  - **Row 22525033 is DROPPED (Challenge A3), so five rows remain.** Its 4 mentions are all TARC spellings with raw `ES4111001F`
    (s8), and its party rows are 9 current rows, all review. Pinning it would move 9 R rows to N and fix no W, and it fails admission
    (d). It keeps its TARC head and goes to F1 with the other TARC rows.
  - Row 6 can be dropped on its own. It is admitted on the reader's evidence: all 300 of its mentions are TARC spellings with raw
    A01034437, and notice 47417709 prints that code on its TARC section. It turns a generic tribunal name, which also fits
    Castilla y León's TARC, into TARC Andalucía's own name. If the U2 reviewer finds that attribution unproven, U2 drops the row
    and nothing else changes.
  - The NIF typo rows are **not** pinned: 656 `S411101F`, 23201016, 22671835, 22541509, 22803632, 22635873, 22522700 and 22720082
    are TARC-headed rows over TARC spellings. They go to F1.
- **D594-3, TARC's review mentions stay on 2660.** TARC's 1,976 mentions (65 spellings) stay where they are.
  - Their review rows serve organization 2660, now named "Junta de Andalucía". `mention_name` keeps the printed TARC name (456
    design 3, `read.rs:3344`).
  - This is D576-1's accepted N: a body's organ served under its parent, with its own printed name. *(Challenge R6: the precedent
    is 587's "one administration" N and D587-2, not D576-1's text, which accepts the reverse split.)*
  - It is **not** 530's AOK W. There, a different legal person's review body was served as the buyer. Here the buyer's department
    and the tribunal are organs of the same legal person, and `mention_name` tells them apart.
  - No TARC rehoming cohort. Rejected alternative (c) gives the reasons.
- **D594-4, the tails: a 545/584 cohort (U4) for the mentions whose body has a legal personality of its own.** That is 46 mentions,
  frozen in `summary.json` / `scripts/classify.py`:
  - **agency, 26:** Consejo Audiovisual de Andalucía ×22 and Consejo de Transparencia y Protección de Datos ×4;
  - **third, 8:** companies, a foundation and associations, 7 of them as Tenderer;
  - **review, 12:** the TSJA contentious-administrative chamber, a court.

  The cohort rules follow 584:
  - `rehome` at HIGH confidence only to an existing row keyed by the body's own register number (NIF/CIF). For the TSJA, which
    prints no number of its own, the target is its largest existing exact-name ES row.
  - Otherwise `keep`, counted as `missing_target`; nothing is minted (`apply_rehoming`, `canonical.rs:33779`).
  - ~~A kept **agency** mention reads N under the new head.~~ *(Amended by Challenge A1.)* A kept **agency** mention stays a known W,
    like third and TSJA. The CAA and the CTPDA are public entities with legal personality of their own (Ley 1/2004 for the CAA, Ley
    1/2014 for the CTPDA), so D594-1's legal-person test makes them W under the Junta's name.
  - A kept **agency**, **third** or **TSJA** mention stays a known W. It is listed in the cohort file and carried as F4.

  These stay on 2660 with no cohort: the Junta-unit mentions (9, residences) and the procurement titles printed as names (8). Under
  the new head the organization is right, so they read N.
- **D594-5, F3 (new issue):** a buyer printed in a Tenderer or winner slot. That is 28 rows on 2660 (Tenderer: Junta 24, TARC 2;
  winner: Junta 2). It is the role-slip shape of 584 §5, outside 594, and no head can fix it.
- **D594-6, the pin's bar and its invariant.**
  - **Admission:**
    - (a) the identifier's registered owner is established;
    - (b) at least 95 % of the row's mentions are that owner or organs without legal personality of their own;
    - (c) a decision file names the row;
    - (d) *(added by Challenge A4)* no single organ makes up essentially all of the row, that is, no organ holds at least 95 % of
      the mentions or at least 99 % review roles. Where one organ does, the head stays that organ, as with 584's KIO on UZP's NIP.
      S4111001F passes (d): Junta organs 77.6 %, TARC 21.7 %, review rows 43 %.

    S4111001F meets (b): 7,064 Junta + 1,976 TARC + 9 units + 8 titles = 9,057 of 9,103, which is 99.5 %. The other 46 are D594-4's
    cohort.
  - **Changes:** an edit to the table bumps `HEAD_ELECTION_EPOCH` in the same commit. A test pins the table's digest beside the
    epoch, so an edit without a bump fails.
  - **Invariant I594 (binding on 584's guard and on any future name-agreement rule):** a triple listed in `REVIEWED_HEADS` is never
    refused by a name-agreement guard. Its holder's identity is reviewed, and name disagreement there is an organ under its parent
    by decision.

## Why not the other mechanisms

- **(a) refresh-org-names as it stands** (`supervisor.rs:5121`, `elect_head` at `head_election.rs:795`) keeps the head, with
  verdict KEEP `bar`.
  - The votes are V 9,103, T 1,073 (11.8 %) and H 66, so neither R1 nor R2 clears its bar.
  - The "Junta de Andalucía" cluster holds 118 votes (1.3 %). The Junta's 7,064 mentions split over 570 clusters, against 36 for
    TARC.
  - So every count-based rule serves TARC, which is the reader's §4. Nothing but a reviewed pin can serve the Junta.
- **(b) Rehoming the 7,064 Junta mentions out** breaks 572-L. The only Junta-named target is 3231, an identifier-less name row. And
  every new notice would recur through the E0 exact-triple bind (`canonical.rs:23199`). Rejected.
- **(c) Rehoming TARC's 1,976 mentions to 22235590 or 3601.** Rejected:
  - Every new TARC section that prints S4111001F binds straight back to 2660 through E0, so a one-shot cohort decays with each
    daily fold. Holding it would take a standing re-cohort, or a resolver rule keyed on TARC's name, which I1 forbids for exactly
    this reason.
  - The gain is N → R on review rows.
  - TARC is already split across 10 or more rows, so the move would make one more fragment. TARC's identity belongs to F1.
  - A rehomed row has no rebuild parity: a from-archive rebuild re-mints it onto 2660.
- **(d) Alias, re-key or identifier verdict.** S4111001F is the Junta's real number, so a re-key has no right target. A `related`
  verdict only freezes the head (E2), and the exact literal still binds (`canonical.rs:21273`). Rejected.
- **(e) 584's name-agreement guard.** Rejected:
  - It is PL only.
  - Its mandatory role condition needs at least 99 % review rows. 2660 has 43 % (17,488 of 40,265), and 41 % of its current rows
    (5,375 of 13,208).
  - Its premise is an *unrelated* body printing the number. Organs of the same legal person are not unrelated.
- **(f) Leave it and document it, as 572-L did.** Rejected. 572 accepted that because its served organization was usually the right
  buyer. Here the served buyer is never the buyer: 3,334 current buyer rows serve the review tribunal.
- **(g) A one-off `UPDATE organizations SET name`.** No such path exists: the only name writer is `ORG_NAME_REFRESH_CAS_SQL`
  (`head_election.rs:2053`). The mint writes the first mention's name (`canonical.rs:23700`), so a rebuild would re-serve TARC.
  ~~Hence P0 must sit in both the election and the mint.~~ *(Amended by Challenge A2.)* A rebuild re-serves first-seen heads for
  every 524 rename, and the runbook already requires a fresh dry plan and a full run after one (`docs/operations.md`, the 524
  Undo note). P0 in the election re-pins in that run, so the mint stays untouched.

## Expected served change

The pin writes `organizations.name` and `name_norm` on 5 rows (6 before Challenge A3). No `organization_id` changes, no mention moves, and nothing is
refolded. Parties read the head live through `JOIN organizations` (`PARTIES_SQL`, `read.rs:3343–3350`), so the change is served the
moment the wet run commits.

s7 counts current party rows (`p.seq = t.current_seq`). Tenders are distinct per row of the table.

| org | party rows (all versions) / Tenders | **current rows / Tenders** | current buyer rows (Procedure-Buyer + buyer) / Tenders | current review rows / Tenders |
|---|---|---|---|---|
| 2660 | 40,265 / 3,937 | **13,208 / 3,887** | **3,334 / 3,333** | 5,375 / 1,237 |
| 22341455 | 109 / 30 | 57 / 30 | 8 / 8 | 38 / 24 |
| 22419088 | 342 / 76 | 153 / 75 | 8 / 8 | 132 / 67 |
| ~~22525033~~ (not pinned, A3) | 13 / 3 | 9 / 3 | 0 | 9 / 3 |
| 11044446 | 207 / 31 | 88 / 26 | 17 / 17 | 29 / 10 |
| **5 NIF rows** | **40,936** | **13,515** (≤ 4,021 Tenders) | **3,367** | 5,583 |
| **4 pinned NIF rows (after A3)** | **40,923** | **13,506** | **3,367** | 5,574 |
| 22235590 (row 6) | 626 / 141 | 286 / 139 | 0 | 285 / 139 |

On 2660's current rows, the grades move as follows:
- **Buyer, W → N.** 3,334 rows, of which 1,046 Procedure-Buyer and 2,267 buyer rows are Junta-named, now serve "Junta de Andalucía"
  with the Consejería as `mention_name`. The other 21 are 11 agency (in U4), 4 title, 2 unit and 4 TARC-named Procedure-Buyer rows.
- **Other buyer-side, W → N.** About 4,499 rows (AddInfo, DocProvider, TenderReceipt, Signatory, Paying and the rest, less the
  current Tenderer/winner slips). By the all-version shares they are about 99 % Junta-named.
- **Review, W → R/N and R → N.**
  - Of 5,375 current rows, about 1,880 (35 %, the all-version share) are the buyer's own section. They go W → R/N.
  - About 3,490 are TARC-named. They go R → N, the accepted cost.
- **In net,** about 9,600 current rows stop serving the review tribunal for a Junta body (W → R/N): 3,313 buyer + about 4,450
  other + about 1,880 review. About 3,500 TARC rows take the parent's name (R → N). The 46 tail mentions are U4's.
- **Siblings:** 298 current rows after A3 (307 before), including 22419088's 8 Procedure-Buyer Tenders that serve TARC today. They
  read like 2660. s8 read their names: 11044446 is 53 of 53 Junta organs, 22341455 is 51 Junta and 6 TARC, and 22419088 is 143 TARC
  and 12 Junta.
- **Row 6:** 286 current review rows gain TARC Andalucía's full name, N → R.

**A correction to the inputs.** `read.md` §2's total row gives "other buyer-shaped rows" as **21,907**. The per-role sums in
`summary.json` give **15,907**, and only 6,835 + 17,488 + 15,907 + 35 = 40,265 adds up to the total. Every figure here uses 15,907.

## Mechanism (U2 build, S)

1. **`crates/store/src/reviewed_heads.rs` (new).**
   - `pub struct ReviewedHead { country, kind, identifier, head, decision: &'static str }`.
   - `pub const REVIEWED_HEADS: &[ReviewedHead]`, holding D594-2's 5 rows (A3 drops 22525033), each with decision `"594 D594-2"`.
   - `pub fn reviewed_head(country: Option<&str>, kind: Option<&str>, identifier: Option<&str>) -> Option<&'static str>`, an exact
     byte match on the stored triple. All 9,103 raw spellings on 2660 already normalise to `S4111001F`.
   - The table lives in store, not in ingest (unlike 528's), because its reader (the election; the mint is no longer one, per A2)
     is in store and it is a fact about an organization, not a parse.
2. **`head_election.rs`.**
   - `HeadElectOrg` gains `country`. The planner already reads it (`HeadRefreshEntry::country`).
   - In `elect_head_counted`, right after the E1 return: if `reviewed_head(..)` is `Some(pin)`, return `Keep(HeadKeep::P0)` when the
     head equals the pin, and `Change { rule: HeadRule::P0, new: pin }` otherwise.
   - P0 asks no window, residue, `undated`, serve, `head_current`, `top_stale` or bind hold, and comes **before E2**: a reviewed pin
     outranks a verdict freeze.
   - The plan counts `pinned`.
   - `HEAD_ELECTION_EPOCH` becomes `524u2-v15`, with a test pinning the table digest beside it.
   - The wet path applies P0 changes. `refresh_wet_filters` accepts `P0` in `exclude_rules`, and the standing filters (`R2`, `R0w`,
     `unfit`) do not touch it.
3. ~~**Mint parity** (`canonical.rs:23700`, and every `INSERT INTO organizations` that carries an identifier triple, including 452's
   guarded-arm mint).~~ **Dropped by Challenge A2.** The rebuild path is the 524 runbook's full re-run. Routing every mint through
   one helper would touch about 15 INSERT sites, including restore re-mints that must stay byte-exact (`guid_twins.rs` restore).
   `canonical.rs` is not touched.
4. **Docs.**
   - The /docs Organizations section says that a small reviewed table pins the head of an identifier owned by a parent
     administration, for example ES S4111001F → Junta de Andalucía, and to read `mention_name` for the printed body.
   - Add a CHANGELOG line.
   - Amend ADR-0013 with rule v15, P0.
   - Every code deploy re-gates the 551 switch-off twin (551 C7), as deploys #3 and #4 did. After A2, U2 touches
     `head_election.rs`, the new `reviewed_heads.rs`, `lib.rs`, `supervisor.rs` (`ORG_NAME_REFRESH_EXCLUDABLE_RULES`), `docs.rs`,
     CHANGELOG and the 524 tests (`store/tests/head_election_agreement.rs`, `refresh_org_names.rs`), but not `canonical.rs`.
5. **Tests** (focused, with the gate's flags and package set; then `ops/check.sh`):
   - **T1** checks the table: unique triples, heads trimmed and NFC with no control characters, a decision on every row.
   - **T2** runs `elect_head` on 2660's vote shape (V 9,103, TARC top 1,073, H 66). The pinned triple gives P0 → "Junta de
     Andalucía". A head already equal to the pin gives Keep P0. The same votes on an unpinned triple give Keep `bar` (the control).
   - **T3** checks that P0 holds against `review_body` (a review-body head changing to a non-review name) and with
     `bind_defect: true`.
   - ~~**T4** (resolver mint parity)~~ dropped with step 3 (A2). In its place, **T4'** runs a scoped plan on a fixture of
     22525033's shape (all TARC, `ES4111001F`, unpinned). It must read R1 or keep, never P0, which proves the table holds the 5 rows
     only.
   - **T5** applies a scoped plan with one P0 change under the standing filters, then reverts it (ledger rule `P0`).
   - **T6** pins the digest and the epoch.

   No `Spec` arm is added, so there is no `run_spec` stack risk.

## The drain (U3), right after the carrying deploy (deploy #6, with 587 U2 if it is ready; neither waits for the other)

1. **Dry:** `{"kind":"refresh-org-names","org_ids":[2660,11044446,22235590,22341455,22419088]}` (A3 removes 22525033).
   - **Expect** exactly 5 entries, each `change` P0, with `old_name` as in the D594-2 table and `new_name` the pin.
   - Also 0 holds, 0 `below_min` (the minimum is 2, and the smallest row now has 53 mentions), and epoch `524u2-v15`.
   - Anything else means stop and fix, not freeze.
2. **Wet:** `{"kind":"refresh-org-names","dry_run":false,"org_ids":[…same 5…],"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]}`.
   These are the standing 524 v12 wet filters.
   - **Expect** `applied` 5, drifted 0 and raced 0.
   - Also 5 `org_name_refresh_log` rows with rule P0, and 5 `('organization', id, NULL, 'changed')` feed rows.
   - Run the wet straight after the dry (Challenge N1). The wet reads the latest stored `org-name-refresh-plan`. If another agent's
     dry plan lands in between, the 5 ids read `unplanned` and nothing is applied, so re-run the dry. Afterwards, re-plan any pending
     altid, nhs-body or r3 dry plans: their parity fails closed on the changed heads (524 runbook, step 7).
   - **Undo:** `{"kind":"revert-org-name-refresh","job":<wet id>}`.
3. **No `project` or refold** is needed (see "Expected served change").
4. **Then:** `{"kind":"build-org-match-keys","dry_run":false}`, outside a fold window, so the name keys and wall counts follow the
   head.
   - Expect new candidate pairs between 2660 and the identifier-less Junta name rows: 3231, 3526566 and the "Junta de Andalucía,
     Consejería de …" rows.
   - None of them merges by itself: a provisional ↔ canonical capture is E3 (300). That pairing is F2's input.
5. **Banner (now, until the read-back passes):** "594: Junta de Andalucía bodies printing NIF S4111001F are served as org 2660
   'Tribunal Administrativo de Recursos Contractuales de Andalucía'; read `mention_name` for the printed body."

**U4 (the tails cohort) runs in parallel, starting now.** It needs only the deployed rehoming path. Cohort `594-tails-<date>` takes
the 46 mentions under D594-4's rules, with a preparer and a challenger as in 584 U2. Its runbook is 584's (`584-u2/runbook.md`):
1. POST `/admin/rehoming`;
2. dry `apply-rehoming`;
3. `plan_check.py`: PLAN == COHORT by tuple;
4. wet;
5. `project`;
6. read back every moved mention.

## Read-back standard (U3, right after the wet run)

- **Deterministic checks:**
  - **C1:** one SQL primary-key seek, `SELECT id, name, name_norm, identifier FROM organizations WHERE id IN (…5…)`. Each name must
    equal its pin, each `name_norm` must equal the plan's `new_name_norm`, and every identifier must be unchanged.
  - **C2:** the wet report must show 5 / 0 / 0 (`applied` / drifted / raced), with a 5-row P0 ledger.
  - **C3:** GET `/v1/tenders/8857916`.
    - Procedure-Buyer: 2660 "Junta de Andalucía", `mention_name` "CONSEJERÍA DE SOSTENIBILIDAD Y MEDIO AMBIENTE".
    - Lot-ReviewOrg: 2660 "Junta de Andalucía", `mention_name` "TRIBUNAL ADMINISTRATIVO … DE LA JUNTA DE ANDALUCÍA".
  - **C4:** GET `/v1/tenders/8852785`.
    - Procedure-Buyer: 2660 "Junta de Andalucía", `mention_name` "Delegación Territorial de Cultura, Patrimonio Histórico y Deporte
      en Sevilla".
    - Lot-ReviewOrg/ReviewInfo: 22235590, under row 6's name.
- **Random 30 at 0 W:**
  - **Draw.** One bounded SQL draws 30 current party rows, uniform over the 13,506 current rows of the 4 pinned NIF orgs (A3). It seeks
    `tender_version_parties_org` per org and joins `tenders` and `organization_mentions` by primary key. The order is salted
    `594-u3-`, with the salt's integer and the statement saved beside the answer. Each row carries role, `organization_id`,
    `organizations.name` and the mention name, which is exactly what `PARTIES_SQL` serves. Three of the 30 are re-read by GET for
    parity.
  - **Replacements.** A drawn row on the frozen tail list (D594-4's 46 mentions or D594-5's 28 slip rows) is replaced by the next
    and counted.
  - **Grades:**
    - **R:** the printed body is the Junta itself.
    - **N:** an organ of the Junta, served as "Junta de Andalucía" with its own printed name as `mention_name`. Organs are a
      Consejería, Delegación, DG, Secretaría, Servicio, establishment, or the TARC. This is D576-1's accepted N.
    - **W:** any of these:
      - (i) a pinned org serving any name but its pin;
      - (ii) the printed body is a legal person other than the Junta and is not on the frozen list;
      - (iii) a missing or empty `mention_name` where the mention has a name;
      - (iv) an `organization_id` that differs from the pre-wet row (the pin moves nothing).
  - **Bar:** 0 W in 30. On a pass, retire the banner and mark Verify done. On a W, a banner line and a fix unit, not a freeze (owner
    ruling).
- **Recurrence (next daily fold, then the next full or weekly refresh plan):** 2660 still serves the pin, and any refresh plan lists
  it as `keep` P0.

**Budget:** C1 plus the draw is 2 SQL, and C3, C4 and the parity reads are 5 GETs. That is a fresh unit budget, not 594 U1's.

## Verify (replaces the issue's)

    curl -s https://tenders.zebreus.click/v1/organizations/2660 | jq -c '[.id, .name, .identifier]'
    curl -s https://tenders.zebreus.click/v1/tenders/8857916 | jq -c '[.parties[] | select(.organization_id==2660) | [.role, .organization_name, .mention_name]] | unique'

- **open (2026-10-24):** `[2660,"Tribunal Administrativo de Recursos Contractuales de Andalucía","S4111001F"]`. The Consejería
  buyer of 8857916 is served as the tribunal.
- **done, line 1:** `[2660,"Junta de Andalucía","S4111001F"]`.
- **done, line 2:** every 2660 party of 8857916 serves "Junta de Andalucía". The Procedure-Buyer's `mention_name` is the Consejería
  and the Lot-ReviewOrg's `mention_name` is TARC.
- **done, also:** 11044446, 22341455 and 22419088 serve "Junta de Andalucía" (22525033 keeps its TARC head, A3); C1–C4 and the random 30 pass; and U4's cohort
  is applied and read back, or its keeps are listed.
- **The old "done" line is withdrawn by D594-3.** It read "Consejería mentions … bind their own bodies, and 2660 serves only TARC's
  own review roles". Under D594-3, 2660 serves the Junta, and TARC's review mentions stay on it under their printed name.

## Interactions

- **584's guard spec (`584-u2/guard-spec.md`).** It must never refuse Consejería or TARC names on S4111001F. Today it cannot:
  - it is PL only;
  - its mandatory role condition needs a holder with at least 99 % review rows, and 2660 has 43 %.

  584 U3's guard decision must still write I594 into §1 Scope: a `REVIEWED_HEADS` triple is exempt from every arm (E0, re-key,
  `canon_hit`), whatever the country or role condition. Two reasons:
  - After the pin, the holder's names would be {"Junta de Andalucía"}. Consejería names share no distinctive token with that, so an
    extended guard without the role condition would refuse 7,064 mentions (N → W).
  - Option B's compiled template table must not list S4111001F. It is an identity, not a template.
- **587 D587-2.** D587-2 aliases a bare label to its notice's buyer section, so its roles inherit whatever that section binds; on
  Junta notices that is 2660.
  - Pre-pin, label 1193769's roles (118 buyer rows beside it; 117 of 135 Tenders) serve TARC. They are a known W owned by 594, and
    587's read-back excludes them.
  - Post-pin they serve "Junta de Andalucía", graded N. That is the D576-1 department-under-parent case, so 587's rubric reads it as
    N.
  - 587 needs no code change. Either order converges, but the 594 wet run (5 rows, minutes) goes first where possible, so 587's
    read-back sees the final name.
- **524.**
  - P0 is rule v15 of the ADR-0013 election, and the epoch bump makes the wet path refuse a v14 plan.
  - The 524 `org-head-drift` census should count pinned orgs as `pinned`, not as minority heads. 2660 keeps H = 118 of 9,103
    forever. This is a nicety, not a gate.
  - Unit 2c's weekly upkeep, when it is built (it is not yet), inherits P0 for free.
- **456 design 3** is what makes the Junta head N rather than W: `mention_name` keeps the printed organ. It is unchanged.
- **572-L / I1** are upheld. The binding is unchanged, and no rule keys on name disagreement.
- **545/584 rehoming** applies to U4 only, under 584's runbook and tuple plan check.
- **513/519.** The TARC identity, its consolidation onto 22235590 (TARC's DIR3), is a reviewed body consolidation. It is F1, not 594.
- **551 C7.** The carrying deploy re-bases the switch-off twin. U2 touches `docs.rs`, CHANGELOG and `head_election_agreement.rs`,
  but not `canonical.rs` (A2).

## Follow-ups (filed by the recorder)

- **F1: TARC Andalucía is split across 10 or more rows.** They are 3601 (17,461), 3947 (4,669), 2720543 (1,076), 8738172 (630),
  22235590 (300, DIR3), 14748 `TARCJA001` (208), 22635797 (122), 656 `S411101F` (125), the typo rows and 22525033 `ES4111001F` (4,
  unpinned by A3). The question is a reviewed
  513/519-style consolidation onto 22235590. Its scope is the rows keyed by TARC's own codes and its identifier-less rows, never
  the NIF-keyed mentions on 2660.
- **F2: one Junta row.** The candidates are a reviewed merge of the 4 sibling NIF rows into 2660, since the weak E2 VAT key never
  canon-binds (`crosswalk.rs:224–228`), and the identifier-less 3231 "JUNTA DE ANDALUCIA" (2,807 mentions, buyer on 2,644 Tenders) as
  an E3 capture question. It is N today (a split of the right body), so it is not urgent.
- **F3: a buyer printed in a Tenderer or winner slot.** There are 28 rows on 2660. This is 584 §5's shape and needs a source-side
  rule.
- **F4: U4's kept agency, third and TSJA mentions (agency added by A1).** These are any of the 46 that `missing_target` leaves behind. They are known W until a
  target exists.

## Units

- **U2: build** (store table, P0, docs, tests; the mint helper was dropped by A2). Status: ready-for-agent. Blocked by: nothing; it rides deploy #6.
- **U3: drain and read-back** (dry, wet, `build-org-match-keys`, C1–C4, random 30, Verify). Status: ready-for-agent. Blocked by:
  U2 deployed.
- **U4: tails cohort** (46 mentions, 584 runbook). Status: ready-for-agent. Blocked by: nothing. It runs in parallel with U2.
- **Banner:** up now, retired when U3 passes.
- **594 DONE** when U3 passes and U4 is applied and read back, or its keeps are listed as F4.

## Challenge (skeptic, 2026-10-24 ~23:20Z)

Scope of the challenge:
- **Code:** read in the working tree. Line numbers have drifted from `f482124`: `elect_head` is now at `head_election.rs:822` and the
  resolver mint at `canonical.rs:23834`.
- **Box reads:** one bounded SQL, **s8** (`raw/sql/s8_sibling_names.*`). It was plan-probed first, with plan `SEARCH m USING INDEX
  organization_mentions_org`. It returned 31 rows in 0.38 s, with no 408. **SQL now stands at 10 of 10, and GETs at 6 of 15** (I made
  no GET).
- **External source:** one web search, on the CAA's legal personality, which pointed to the BOE text of Ley 1/2004.
- Nothing was written on the box or POSTed, and no cargo or git was run.

**Headline:** the decision **stands**. S4111001F is the Junta's NIF for every body on 2660 except the frozen tails. A reviewed pin in
`elect_head` is the only mechanism that can serve the Junta, it is needed for the siblings, and it survives refolds. Four amendments
were applied (A1–A4), plus one operations note (N1). Six challenges were refuted (R1–R6).

### Applied

- **A1 (defect, D594-4): the kept agency mentions are W, not N.**
  - The Consejo Audiovisual de Andalucía is a public entity with legal personality of its own (Ley 1/2004; the Junta's 2014 budget
    report says the same). It is also tied to the Parliament, not to the Junta's executive.
  - The Consejo de Transparencia y Protección de Datos de Andalucía is likewise a public entity with its own personality (Ley
    1/2014).
  - D594-1's own legal-person test, and 530's AOK reasoning, therefore make "Junta de Andalucía" W for them.
  - What changed: their 26 mentions join third and TSJA as known W in F4 whenever U4 keeps them (`missing_target`). That covers 11
    current agency buyer rows on 2660.
  - What did not change: the read-back bar, because the frozen list already excludes them from the random 30.
- **A2 (cheaper; Mechanism step 3, (g), T4): mint parity is dropped.**
  - `docs/operations.md`'s 524 Undo note already says a from-archive rebuild re-mints first-seen heads, and that "the heads need a
    fresh dry plan and full run". The ledger comment at `canonical.rs:548–555` says the same. So every one of the ~15k 524 renames
    already depends on that run.
  - P0 in `elect_head` re-pins in that same run, and it is keyed on the identity triple, so it survives renumbering.
  - Routing "every `INSERT INTO organizations` that carries a triple" through one helper would touch about 15 mint sites across
    `canonical.rs`, `lib.rs` and `guid_twins.rs`. Some are restore re-mints that must write their pre-image byte for byte (the
    guid-twin restore at `guid_twins.rs:4143`).
  - Result: U2 no longer touches `canonical.rs`, the size stays an honest S, and T4 becomes T4'.
- **A3 (row cut, D594-2): 22525033 is not pinned.**
  - s8 shows all 4 of its mentions are TARC spellings with raw `ES4111001F`, and s7 shows its 9 current rows are all review.
  - The pin would move 9 R rows to N and fix no W, and the row fails A4's (d).
  - So 5 rows remain, and the drain, C1/C2, the random-30 frame (13,506 rows over 4 NIF orgs) and Verify were updated in place.
- **A4 (rule, D594-6): admission (d) is added.** No single organ may make up essentially all of a row (95 % of mentions, or 99 % review
  roles).
  - Without (d), bar (b) would also admit UZP's NIP under "Urząd Zamówień Publicznych", because KIO is UZP's organ. That contradicts
    584, which keeps the organ KIO as head on its parent's number.
  - With (d), the principle reads: serve the registrant when many of its organs share the number, and serve the organ when one
    organ is the number in practice.
  - 2660 passes (Junta organs 77.6 %, TARC 21.7 %, review rows 43 %). 22419088 passes (TARC 92 %). 22341455 and 11044446 pass (a
    mix of Junta organs). 22525033 fails (TARC 100 %).
- **N1 (operations, U3).** Run the wet right after the dry, because the wet reads the latest stored `org-name-refresh-plan`. If
  another dry lands in between, the 5 ids read `unplanned` and nothing is applied. That is harmless: re-run the dry.
  - After the wet, re-plan any pending altid, nhs-body or r3 dry plans (524 runbook, step 7).
  - The epoch bump also moves `ORG_NAME_REFRESH_SAMPLE_SALT` (`head_election.rs:1562–1568`), which tests pin, so U2 updates them.
  - The ledger's `rule` column has no CHECK constraint (`canonical.rs:560`), so P0 needs no schema change, only the column comment.
  - `ORG_NAME_REFRESH_EXCLUDABLE_RULES` (`supervisor.rs:2049`) gains `P0`. The plan target's `rule` must equal `HeadRule::P0.label()`,
    because `read_readiness` compares labels.

### Refuted

- **R1: "S4111001F is not the Junta's number for some of these bodies (SAS, IDEA, agencies with their own NIF)."**
  - s3 read all 987 names on 2660. The classifier sends SAS, SAE, `agencia`, `instituto andaluz`, `empresa/ente público`, APES,
    `consorcio`, the Parliament, the Cámara de Cuentas and the Defensor to `agency` *before* `junta`. Only the CAA and the CTPDA
    land there.
  - I re-scanned every junta-, unit- and other-class name for other-entity markers: ayuntamiento, diputación, ministerio, hospital,
    distrito, universidad de, fundación, sociedad, instituto, agencia, tribunal, colegio and similar. Every hit is a Junta organ.
  - Only one item is misclassed: 3 mentions of "Tribunal Administrativo de **Recusos** Contractuales" count as junta and should
    count as tarc. Both classes are organs, so nothing changes.
  - s8 closes the gap on the sibling rows, whose names were unread: 11044446 is 53 of 53 Junta organs, 22341455 is 51 Junta and 6
    TARC, and 22419088 is 143 TARC and 12 Junta.
  - The own-NIF agencies print their own numbers and never bind here.
- **R2: "Serving 'Junta de Andalucía' is wrong for some served role."**
  - The buyer, signatory, paying, financing, AddInfo, DocProvider, TenderReceipt and *Legis rows are all printed by Junta organs,
    so the parent's name is correct for them.
  - TARC in review roles is an organ of the Junta (an órgano colegiado attached to Hacienda, with no legal personality), so it reads
    N, with `mention_name` naming it. The organization id is already 2660 for buyer and review alike today, so no id-level
    semantics change.
  - The only W rows are the frozen tails (TSJA, third, and now agency per A1) and the 28 role slips (F3).
- **R3: "The mechanism does not work as claimed."**
  - The only writers of `organizations.name` are the refresh CAS and the revert CAS (`head_election.rs:2080/2105`). No fold, merge
    or rehoming path writes it.
  - Parties read the name live (`PARTIES_SQL`, `read.rs:3343–3350`), and `tender_version_parties` has no name column
    (`canonical.rs:410–421`). `/v1/organizations/:id` serves no satellites.
  - `PageOrg` already carries `country` (`head_election.rs:1608–1610`), so P0 can see the triple in the planner and in the wet
    pre-pass. The E2 probe runs only on a Change, and P0 before E2 is consistent with it.
  - Every name-keyed resolver lookup filters `identifier IS NULL` (`canonical.rs:9441, 22272, 23951, 32929, 42587, 42702`), so the
    new name `junta de andalucía` on identifier-bearing rows moves no bind key and cannot capture 3526566 or 3231 mentions.
  - No existing path can serve the Junta without new code. 513's `keep_name` is the survivor's own head, and 528's table rewrites
    mention names. A hand-made plan would drift, because the wet pre-pass re-elects every target.
- **R4: "It does not survive the next refresh-org-names or refold."**
  - Refolds and daily folds never write heads.
  - On the next refresh, P0 returns Keep P0.
  - Even without P0, 2660 would KEEP `bar` once renamed: H = 118 fails R2, and T = 1,073 is under 55 % of V.
  - 22419088, however, would flip back by R1 (T 143 of V 155), and 22525033 would too if it were pinned. So the election half of
    the mechanism is required, and A2 removes only the mint half.
- **R5: "A cheaper fix exists."**
  - The only cut available is A2, which was applied.
  - Rehoming TARC's mentions decays through E0, as the decision's (c) says.
  - A one-shot pin with no election rule would not hold the sibling rows (R4).
- **R6: "N for TARC under the parent contradicts D576-1."**
  - D576-1's own accepted cost runs the other way: a department becomes its own name-keyed party.
  - 587's rubric reads "one buyer or one administration" as N, and D587-2 already serves a label's roles under the parent buyer. The
    organ-under-parent N is therefore precedent through 587, not through D576-1's text.
  - The decision's grade stands; its citation should read "D576-1 / 587's N".

### Residual risks, not blockers

- **3,490 TARC review rows go R → N.** A consumer who reads only `organization_name` loses the review body's identity and must read
  `mention_name`. This cost was accepted, and F1 owns TARC's own identity.
- **Satellites can be stale.** `organization_names` for 2660 hold Consejería spellings (ENG and SPA), so name search still reaches
  2660 by department names. That is pre-existing and not served on `/v1/organizations`.
