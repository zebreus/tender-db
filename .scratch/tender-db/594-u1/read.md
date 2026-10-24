# 594 unit 1 — the read (reader, 2026-10-24 ~22:55Z)

Inputs: issue 594 (Status: unit 1 sized), `reads.json` (3 earlier reads), precedents 584 (`584-u2/guard-spec.md`), 572
(`572-triage/decision.md`), 528 U3, 524 (`crates/store/src/head_election.rs`), 513/519, 545/584 rehoming, 456 design 3,
D576-1.

Budget: **8 `/v1/sql` statements of 10** (s1, s2, s3, s4a, s4b, s4c, s5, s6), each plan-probed locally first with turso
`plan-probe` on a copy of 584's `plan_prod.db` (prod schema + prod indexes); every plan is an index seek, and the only `SCAN`s are a
subquery or a ≤6-row CTE. Each answer has `row_count` and `columns` and is not truncated. There were 0 errors, 0 408s and 0 retries;
the slowest statement took 1.78 s. **6 public GETs of 15**, all 200. Everything is logged in `requests.log`, with statements, plans
and answers in `raw/sql/` and GET bodies in `raw/get/`. Nothing was written on the box, and nothing was POSTed.
Code lines are cited at **HEAD `f482124`**: the working tree is being edited by deploy #5's build, so its line numbers drift. One
exception is 575 U3's `cf35.rs`, which is not in HEAD yet.

Nothing was sampled. s3 reads **all 9,103** mentions on org 2660, and s4a/b/c read **all 40,265** of its party rows (every role, every
version). The totals match `reads.json` exactly.

## (1) Rows keyed by S4111001F and its spellings, and the Junta by name

Identifier probes (s1, `organizations_identifier_id` equality seeks): `S4111001F`, `ESS4111001F`, `S4111001`, `ES4111001F`,
`4111001F`, `ESS4111001`, and the Junta's DIR3 `A01002820` / `ESA01002820`.

| org | identifier (kind) | head | mentions | party rows | what it serves |
|---|---|---|---|---|---|
| **2660** | S4111001F (national) | Tribunal Administrativo de Recursos Contractuales de Andalucía | 9,103 | 40,265 | the Junta's whole NIF stream: 7,064 Junta-department mentions and 1,976 TARC mentions (see 2) |
| 22341455 | ESS4111001F (vat) | Consejería de la Presidencia y Administración Local de la Junta de Andalucía | 57 | 109 | Junta, VAT spelling |
| 22419088 | S4111001 (national, check letter lost) | TRIBUNAL ADMINISTRATIVO DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCIA | 155 | 342 | **the same defect in small**: 143 TARC mentions, 12 Consejería mentions, Procedure-Buyer 11 rows / 8 Tenders (s5) |
| 22525033 | ES4111001F (vat) | Tribunal Administrativo de Recursos Contractuales de la Junta de Andalucía | 4 | 13 | TARC |
| 11044446 | 4111001F (national) | Junta de Andalucía. Consejería de Cultura. Secretaría General Técnica | 53 | 207 | Junta |

- No row carries the Junta's DIR3 (`A01002820`).
- Every one of 2660's 9,103 raw identifiers normalises to the exact literal: `S4111001F` ×7,916, `S-4111001-F` 366,
  `S-4111001F` 307, `S 4111001F` 249, `S4111001F.` 76, and so on. All are country ES; scheme is `national` on 4,781 and NULL on 4,322.
- The VAT spellings do not canon-bind to 2660. `ES:nif` is an **E2** key (`crosswalk.rs:224–228`), and the Stage-2 `canon_hit`
  binds E1 keys only (`canonical.rs:23231`). So each spelling mints its own row.
- The name range seek also found TARC NIF typos, each its own tiny row: 656 `S411101F` (125 mentions), 23201016 `S411111001F` (7),
  22671835 `S4111001FA`, 22541509 `S4111001FA01034437`, 22803632 `0S4111001F`, 22635873 `S4111100F`, 22522700 `F4111001F`, and
  22720082 `TARCJA4111001F`.

**Junta de Andalucía rows by name (ES).** From s2, `organizations_name_norm_id` range seeks:
- 3231 "JUNTA DE ANDALUCIA" has **no identifier**, provisional, **2,807** mentions. It is buyer on 2,790 rows / 2,644 Tenders (older
  eras, printed with no NIF; s5).
- 3526566 "Junta de Andalucía" has no identifier, 46 mentions.
- The `junta de andalucía…` prefix holds 113 rows and `junta de andalucia…` holds 664. Almost all are provisional
  "Junta de Andalucía, Consejería de …" name rows. Only 7910826 (`S1141001F`, a typo) and 11044446 carry an identifier.
- **No row is both named "Junta de Andalucía" and keyed by S4111001F.**

**TARC rows (needed for 3).** TARC exists apart from 2660, split many ways:
- **identifier-less rows:**
  - 3601 "Tribunal Administrativo de Recursos Contractuales de la Junta de Andalucía." has **17,461** mentions;
  - 3947 (the ALL-CAPS no-accent spelling) has 4,669;
  - 2720543 "…de Andalucía" has 1,076 and is review-only (ReviewOrg 586, review-body 524, ReviewInfo 360 rows);
  - 8738172 "(Tarcja)" has 630;
  - 17592209 has 168, 8652559 has 164, 1710091 has 99, and 15246321 has 80.
- **identified rows:**
  - **22235590 `A01034437` (national)**, headed "Tribunal Administrativo de Recursos Contractuales", has 300 mentions. All of them are
    TARC spellings with raw `A01034437`, and its roles are review-only: ReviewOrg 385, ReviewInfo 216, Mediator 21, FiscalLegis 4.
    A01034437 is TARC's own DIR3 code: notice 47417709 prints it on the TARC section.
  - 14748 `TARCJA001` has 208 mentions, 22635797 `TARCJA1` has 122, and 656 `S411101F` has 125.
- The generic 3708 "Tribunal Administrativo de Recursos Contractuales" (4,592 mentions) pools several regions' tribunals.

## (2) Org 2660's mentions by class, and their roles

The classes come from a regex classifier (`scripts/classify.py`), checked by reading every non-Junta name by hand:
- **tarc**: a recurs…+contract… phrase or TARC;
- **junta**: Consejería / Delegación / D.T. / Secretaría / Dirección General / Servicio / Junta de Andalucía;
- **agency**: own legal person;
- **third**: a company, foundation or association;
- **review**: a court;
- **unit**: an establishment;
- **other**: a procurement title printed as the name.

The table comes from `summary.json` (`scripts/summarize.py`). Party rows count all versions.

| class | names | mentions | buyer rows (Procedure-Buyer + buyer) | review rows (ReviewOrg/Info/Mediator, Lot + Part) | other buyer-shaped rows (AddInfo, DocProvider, TenderReceipt, TenderEval, Signatory, Paying/Financing, SProvider, Legis …) | Tenderer/winner |
|---|---|---|---|---|---|---|
| **junta** | 899 | **7,064** (77.6 %) | **6,794** | 6,088 | 15,791 | 26 |
| **tarc** | 65 | **1,976** (21.7 %) | 5 | **11,362** | 48 | 2 |
| agency | 4 | 26 | 22 | 17 | 59 | 0 |
| review (TSJA) | 3 | 12 | 0 | 12 | 0 | 0 |
| unit | 5 | 9 | 5 | 0 | 6 | 0 |
| third | 8 | 8 | 1 | 9 | 3 | 7 |
| other | 3 | 8 | 8 | 0 | 0 | 0 |
| **total** | 987 | 9,103 | 6,835 | 17,488 | 21,907 | 35 |

**The buyer side is Junta-named.**
- Procedure-Buyer has 2,244 rows: 2,234 junta, 5 tarc (on 4 Tenders), 4 agency, 1 third.
- buyer has 4,591 rows: 4,560 junta, 18 agency, 8 other, 5 unit, and 0 tarc.
- On **current versions**: Procedure-Buyer 1,054 rows (1,046 junta) and buyer 2,280 rows (2,267 junta). This is W today: the
  consumer reads "TARC" as the buyer.

**TARC-named mentions sit in review roles.** Of their 11,417 rows, 11,362 are review roles (ReviewOrg 7,620, ReviewInfo 2,778,
Mediator 860, Part 104). These are R in substance, served under TARC's own head.

**Junta names in review roles (6,088 rows) are mostly the buyer's own section.**
- ReviewInfo has 4,833 rows: the buyer named as the source of review information.
- ReviewOrg has 878 rows: the recurso de reposición goes to the contracting organ itself.
- Under a Junta head these read right.

**Tails** (these need a cohort whatever the head):
- **agency, 26 mentions:** Consejo Audiovisual de Andalucía ×22 (buyer on 13 rows) and Consejo de Transparencia y Protección de
  Datos ×4. Both are legal persons of their own that print the Junta's NIF, so this is 584-shaped.
- **third, 8 mentions:** companies and associations (ASCENDER SL, Equipamientos de Oficina Trujillo S.L, AV13 … UTE, Fundación
  Mornese, Verdiblanca …), 7 of them as **Tenderer**. This is W under any head: 545/584 rehoming.
- **review, 12 mentions:** the TSJA contentious-administrative chamber, a court and not the Junta's executive.
- **Junta/TARC named as Tenderer/winner, 28 rows:** a publisher role slip, outside this issue.
- **unit, 9 mentions:** residences. **other, 8 mentions:** procurement titles printed as the party name.

**Satellites.** 2660's `organization_names` hold ENG "DELEGACIÓN TERRITORIAL DE ECONOMÍA, HACIENDA … EN CÓRDOBA" and SPA "Consejería de
Agricultura,Ganadería,Pesca y Desarrollo Sostenible. Dirección General de Infraestructuras del Agua." The latest variant per
language wins (ADR-0013 D4), so name search reaches 2660 through department names.

**Exhibits** (GETs, served state):
- Tender **8857916** (notice 47424252): the Procedure-Buyer is 2660 "Tribunal Administrativo de Recursos Contractuales de Andalucía",
  with mention_name "CONSEJERÍA DE SOSTENIBILIDAD Y MEDIO AMBIENTE". Lot-ReviewOrg is also 2660, with mention_name "TRIBUNAL
  ADMINISTRATIVO … DE LA JUNTA DE ANDALUCÍA". Both sections print the NIF (`S4111001F` / `S-4111001-F`).
- Tender **8852785** (notice 47417709): the Procedure-Buyer is 2660, with mention_name "Delegación Territorial de Cultura, Patrimonio
  Histórico y Deporte en Sevilla". Lot-ReviewOrg/ReviewInfo is **22235590**, because this TARC section printed `A01034437`. So TARC
  binds its own row whenever a notice prints TARC's DIR3 rather than the Junta's NIF.
- `/v1/organizations/2660`: `identifier_status: null` and `merged_identifiers: []`.

## (3) Which mechanism can serve the right name

| mechanism | exists? | what it needs (code, HEAD f482124) | fit for 594 |
|---|---|---|---|
| **refresh-org-names** (524) | yes | Job `refresh-org-names` (`supervisor.rs:5121`) with `org_ids` scoping (`head_election.rs:1358`), and `revert-org-name-refresh` (`supervisor.rs:5206`). Ledger `org_name_refresh_log` (`canonical.rs:556`, not on `/v1/sql`'s allow-list). Rule `elect_head` (`head_election.rs:795`): E1 keeps an identifier-less org (`:843`), E2 a bind-defect verdict (`:846`), S1 (`:914`), R1/R2 bars (`:916`), otherwise KEEP `bar` (`:921`) | **No.** It elects only among mention spellings. For 2660: V = 9,103, T = 1,073 (11.8 %), H = 66, so R1 (T ≥ 55 % V) and R2 (H ≤ 1) both fail and the verdict is KEEP `bar`. The "Junta de Andalucía" cluster has only **118** votes (1.3 %), so no vote rule can serve it. |
| **Reviewed head pin** | **no** | No endpoint or job writes `organizations.name` except refresh-org-names (524 triage: no `UPDATE organizations SET name`). A mint writes the first mention's name (`canonical.rs:23700`), and a from-archive rebuild re-mints first-seen heads (the `org_name_refresh_log` comment, `canonical.rs:556`). | **New code.** A compiled, reviewed table keyed by the identity triple: `(ES, national, S4111001F)` → "Junta de Andalucía", in 528's compiled-table style. It is read (i) by `elect_head` as a rule ahead of S1/`bar` (bump `HEAD_ELECTION_EPOCH`), then written by `refresh-org-names {"org_ids":[2660]}` with its ledger and D5 rows, and (ii) at mint, so a rebuild keeps it. The serve holds do not block it: `review_body` holds a change *to* a review name, never away from one (`crosswalk.rs:2096`). Effect: all 9,103 mentions / 40,265 rows serve "Junta de Andalucía" with the printed body as `mention_name` (456 design 3, `read.rs:635`). The 22419088 / 22525033 / typo rows need their own entries or a merge. |
| **528 U3 reviewed rename table** | yes | `NL_AWARD_REVIEWED` (`partyname.rs:4614`), `nl_award_reviewed` (`:4875`), read at fold on each BT-500 (`project.rs:6264`) | **Not as is.** It rewrites a *published mention name*. Here the names are right and the *head* is wrong. It is the pattern for the pin: compiled, reviewed, changed only by a gated commit. |
| **Reviewed rehoming** (545/584) | yes | `POST /admin/rehoming` (`admin.rs:48`). A `rehome` needs `target_org_id` (`admin.rs:130`). `apply_rehoming` (`canonical.rs:33821`) moves only HIGH rehomes to a target that **exists**; it never mints one (`:33779`, otherwise `missing_target`). Then dry `apply-rehoming` (`supervisor.rs:4648`), a tuple plan check, wet, `project`. | **Partly.** Targets exist for TARC: 22235590 (TARC's DIR3 A01034437, review-only) or 3601 (the largest identifier-less TARC row). Cohort: 1,976 TARC mentions (65 names), plus 12 TSJA and the 34 agency/third tails. **Recurrence:** the E0 exact-triple bind (`canonical.rs:23199`) sends every new TARC section that prints S4111001F back to 2660. Moved rows stay put (`RecordedMention::publishes`, `canonical.rs:15759`). Rehoming the 7,064 Junta mentions *out* is wrong: it breaks 572-L (a mention binds its identifier's holder), the only Junta-named target 3231 is an identifier-less name row, and every new notice recurs. |
| **Alias** | GB only | nhs-body alias: `GB:nhs-body` merge verdicts, `nhs_body_triple_bind` `canonical.rs:22185`. Register alias: GB (469 U5). 453 re-key alias: needs a HIGH `wrong` identifier verdict with `correct_identifier` (`POST /admin/identifier-verdicts`, `admin.rs:51`; table `canonical.rs:781`) | **No.** S4111001F *is* the Junta's own number, and TARC has no NIF, so a re-key has nowhere right to go. A `related` verdict only flags the org and freezes the head (E2). The exact literal still binds ("The exact triple below still binds", `canonical.rs:21273`), and all 9,103 raw spellings normalise to that exact literal, so the 452 guard catches 0. |
| **584 name-agreement guard** | spec only | `584-u2/guard-spec.md` §1 and §2 (A: the resolver arms at `canonical.rs:23199/23231`; B: a compiled table beside `guid_twin_gate`, `project.rs:6342`; the cf35 refusal in `gated_identifier` is the same shape, 575 U3, not yet in HEAD) | **No.** The spec is PL only. Its mandatory role condition needs a holder with ≥ 99 % review rows, and 2660 has 17,488 / 40,265 = **43 %**. Its premise is "a template identifier printed under an *unrelated* body's name". That fits only the agency and third tails (34 mentions). The Consejerías and TARC are organs of the same legal person (the Junta), so they are not unrelated. |

**Reading of the precedents.**
- **572-L:** the binding stays with the identifier's holder, and only the holder's identity is in question.
- **456 design 3:** each party already serves its own printed name.
- **D576-1:** its accepted cost (576 issue lines 195–197 and the rubric at 238) reads a buyer department as N when it is not served as
  the parent's own wrong name. A Consejería served as "Junta de Andalucía" with its own `mention_name` is the department-under-parent
  N. TARC served as "Junta de Andalucía" is the same N, since TARC is an organ of the Junta.
- **584:** there, KIO sat inside UZP, whose NIP it was, and only unrelated legal persons were rehomed. The same split applies here: the
  head becomes the Junta, and only the 34 agency/third mentions go to a cohort.

## (4) How 2660 got TARC's name

- **First seen at mint, not elected.** Of 2660's 9,103 mentions, the lowest notice id is **notice 995**, printing "Tribunal
  Administrativo de Recursos Contractuales de Andalucía" with `S4111001F` (that spelling and raw: 56 mentions). This is the head
  byte for byte.
- All five identifier rows read in s1 carry `created_at` 1786785861 (2026-08-15 09:24Z, the from-archive rebuild). The mint INSERT writes the
  triggering mention's name (`canonical.rs:23700`), and the fold keeps "first-seen semantics" for the head (`project.rs:6269`).
- **The election keeps it.** The vote is an approximate port of `head_vote_key` (`scripts/election.py`): V 9,103, 626 clusters,
  T 1,073 (11.8 %), H 66. R1 fails at T·20 < V·11 and R2 fails at H > 1, so the verdict is KEEP `bar` (`head_election.rs:916–921`).
  The issue's line "the head election picked a TARC spelling because review-role mentions dominate" is corrected: no vote ever chose
  the head. Notice order did.
- **Which spelling would vote.** Even if a vote decided, TARC wins. Its spellings concentrate:
  - the top cluster is "tribunal administrativo de recursos contractuales de la junta de andalucia" with 1,073 votes (391 + 348 + 205
    …);
  - next is "tribunal administrativo de recursos contractuales" with 428;
  - the misspelled "…contracturales…" has 172.
- The Junta's 7,064 mentions fragment over 899 spellings and **570 clusters**. The largest is "Consejería de Agricultura, Ganadería,
  Pesca y Desarrollo Sostenible. DG Infraestructuras del Agua" with 381; "Junta de Andalucía" itself has 118. TARC has 36 clusters in
  all. So any count-based head serves TARC, and only a reviewed pin serves the Junta.

## Follow-ups noticed (not decided here)

- TARC itself is split across at least 10 rows: 3601, 3947, 2720543, 8738172, 22235590, 14748, 22635797, 656, 22419088, … This is a
  513/519-style consolidation question.
- 22235590's head is the generic "Tribunal Administrativo de Recursos Contractuales". 237 of its 300 mentions (79 %) name the
  Junta de Andalucía.
- D587-2 inherits whatever 2660 serves. With a Junta head, the label-aliased roles on 1193769 become N instead of W.
