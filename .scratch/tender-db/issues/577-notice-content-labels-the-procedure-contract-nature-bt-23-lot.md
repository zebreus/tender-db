# 577 — /v1/notices/{id}/content labels the procedure-level contract nature `BT-23-Lot` on SDK 1.13/1.14 notices

Status: ready-for-agent — **slice 1 DIAGNOSED 2026-10-24 ~20:45Z (wf_feca2058-025): the 126,812 "corrections" are the reparse job's own party deletes being restored, not a 577 parse change — issue 591** (`clear_parsed` deletes every party/bid-party row citing a re-parsed notice; the paired project rewrites them). The archive XML matches the restored rows (26213590 8/8, 26198825 11/11), TED copies untouched, old = new counts where rowid gaps allow: expected net served party change 0. So slice 1 PASSES on evidence and the 0-correction bar is unreadable for any reparse slice until 591 ships. Slice 2 (2776 → 2777) runs as is; **slice 3 (2778/2779) CANCELLED 20:50Z** and is re-enqueued with the sdk rounds once 591's fix is deployed (each round would otherwise serve its Tenders without parties for about an hour). Previously: **unit 3 slice 1 RAN 2026-10-24 (reparse 2773 + project 2774, rev 6cb4408) and FAILS the 0-correction bar on party rows only**: re-parsed 218,876 de-1.x notices (0 unmatched / re-keyed / failing); project: 126,812 Tenders written, **0 verified, 126,812 corrected**, 326,839 correction rows (tender 126,812, lot 200,027). Only `tender_version_parties` (220,649 of 499,644 version tables) and `tender_version_bid_parties` (186,010 of 221,469) differ; lots, amounts, dates, texts, results, winners, contracts, bids, classifications and versions are identical, so W7's three code moves changed no canonical fact. 539 run b refolded the same notices at 02:00Z (rev 512b297, no reparse) and corrected 2,211, so the cause is the reparse under 577 U2 (prime suspect) or another parse change since. Banner updated. Diagnosis (wf_feca2058-025, `577-u3/s1-diagnosis/`): order-only NOTE class vs content. Slice 2's reparse 2776 is running, its project 2777 queued. Was: ready-for-agent — **unit 2 RE-APPLIED 2026-10-24 (bdd055d) under the owner's velocity ruling** (CLAUDE.md "Velocity first": calendar holds are void; W7 informs, it does not gate). It deploys with deploy #2 (banner + 577 U2 + CLAUDE.md), gate on 05313fa. NEXT: unit 3 as SLICES, never the unsliced job (`577-u3/README.md`; 551 `b6/fast-plan-critique.md` finding 1: 3.3M changed notices push the paired project past the 500k closure caps into a ~5 h whole-corpus re-projection). Slice 1 = de-1.x whole is W7's live test (bar: 0 corrections outside the NOTE classes). Undo is FORWARD: a `git revert bdd055d` child of the deployed rev, gated and deployed (the box refuses ancestors; critique finding 5). Was: ready-for-agent — **unit 2 REVERTED from main 2026-10-24 (b3effc2, gated green 1,384 s), because 551's W7 is NOT empty** (wf_52627394-803: an analyst and an adversarial skeptic, CONFIRMED; `551-lot-sum/b6/w7/W7.md`). Three moves the code makes on eforms-de-1.x (D577-1 (a)) land on ids the fold reads for path W: the lot estimate (its old id normalises to BT-27-Procedure), the lot framework maximum and the lot name. Reading the code says none of them changes a canonical fact (both sides normalise to the same stem; stated totals skip DE-1.x), but W7 is static by decision. So unit 2's deploy and unit 3 wait until after 551 U9 ends (11-04). NEXT (after 11-04): revert b3effc2, which re-applies 406529a unchanged; re-gate; deploy; then unit 3. Was: ready-for-agent — **unit 2 BUILT and GATED 2026-10-23** (wf build-578u2-579u4-577u2: implement, 3 review lenses, a skeptic per finding, 8 findings fixed; focused 693 passed; gate green at 406529a), on main, NOT deployed: it rides the batched deploy after 539's drain and the 10-28 day-7 fetch (decision-w-on-drains: one fold-changing commit per probe P1; 577 is parse-only, W7 overlap check first), then the twin re-gate. Unit 3 (re-parse, expected 0 corrected) after 546 U4. Was: ready-for-agent — triaged 2026-10-21 (wf triage-575-578: measurer + 2 adversarial skeptics; 4 + 2 + 2 bounded /v1/sql
reads, 3 GETs; skeptic-corrected, `## Triage (2026-10-21 …)` below). Reproduced and widened. An alias graft whose path carries more
predicates than the inventory's own declaration lands on a sibling branch that `Branch::matching` sorts first (index.rs:819,
1288-1300, 1340; parse.rs:256), so 47414082's PROCEDURE serves BT-23-Lot: 2,193 of 2,201 recent eForms notices, 40/40 root rows on
sdk-1.3/1.7/1.10/de-2.1, every EU minor and DE-2.x. On eforms-de-1.x the same mechanism serves 113 declared (110 lot-level) ids under
procedure-level/UBL- ids (de-1.2: 117 non-root title rows on DE1-ProcurementProject-Name, 0 on the lot id). The top-level EXTRA
insert (index.rs:921, not gap-only) overwrites 4 SDK ids outright (BT-92/93-Lot on all 17 SDK-shaped inventories, BT-749-Lot
1.0–1.11, OPT-050-Lot 1.0–1.8). **D577-1 DECIDED**: a declared field binds over a patch-table field only for the same business term,
on eforms-de-1.x, or where the graft matched by negated predicates alone (BT-772-Lot over BT-67(b)-Procedure on 1.3–1.7: 136 of 136
sampled exclusion-ground descriptions); the top-level EXTRA insert becomes gap-only; TEXTS gains the two DE-1.x lot ids and BT-749 so the canonical
layer changes by 0 (else DE-1.x lots lose tenderer_requirement/selection_criterion, project_apply.snapshot:143). NEXT: unit 2 build
(index.rs, parse.rs, project.rs TEXTS, tests, CHANGELOG + docs sentence). Code is free to land (551 U7/U8 PASS, 969afba); its deploy
rebuilds 551's switch-off twin d569107 (551 C7). Unit 3 re-parse (19 profiles, not sdk-0.1) after 539's drain. Was: needs-triage — filed 2026-10-19 from the hourly audit (workflow wf_90e506fc-209, auditor + skeptic, confirmed;
`.scratch/tender-db/audits/2026-10-19-recent-tenders/verify_ted.md`). Next: unit 1, find the alias graft that adds the
predicated sibling and fix the branch ordering so an SDK-declared field id wins at its own path; re-parse scope.
Kind: data quality (notice content: field ids in the source's own vocabulary, served and in the notice_* tables); medium severity —
wrong ids on nearly every eForms root nature and every DE-1.x lot; canonical facts unaffected (unit 2 keeps them at zero change)
Relates to:
- 368 (BT-23 for FTS; fts/parse.rs is its own parser and unaffected), 465.
- 551 B6 (CHANGELOG/docs files, switch-off twin d569107), 539 (DÖE drain goes first), 575/576 (same single-writer queue).
- 85/98 (DE-1.x vocabulary normalised by DE1_FIELD_ALIASES; the ≈218k DE-1.x cohort), 88 (UBL- grafts), 142/143/195 (direct grafts).

## Verify

    curl -s https://tenders.zebreus.click/v1/notices/47414082/content | jq -c '[.sections[] | .section_id as $s | .values[] | select(.field_id|test("^BT-23")) | [$s, .field_id]]'

Verifies unit 4 (after unit 3's re-parse). The DE-1.x and overwrite probes are in unit 4's read-back.
- **done**: `[["PAR-0000","BT-23-Part"],["PROCEDURE","BT-23-Procedure"]]`
- **open (2026-10-21, computed from the saved 10:26Z body `.scratch/tender-db/577-triage/get2-47414082.json`)**:
  `[["PAR-0000","BT-23-Part"],["PROCEDURE","BT-23-Lot"]]`. The 2026-10-19 command read `.fields[]?` and printed `[]` in both
  states: the content shape is `.sections[].values[]`. `tender-db.zebreus.click` answered this path with an empty body on
  2026-10-21 (get1), so use `tenders.zebreus.click`.

## What was found

- Notices 47414082 (SDK 1.14, subtype 4, tender 8852009) and 47413608 (SDK 1.13, subtype 4, tender 8852005): the root PROCEDURE
  section serves BT-23-Lot; neither notice has a Lot section or a BT-23-Procedure row. The other procedure-level terms (BT-21,
  BT-24, BT-262, BT-22) carry the -Procedure suffix. Notice 47413157 (subtype 7, SDK 1.13, tender 8851996) shows BT-23-Lot in
  PROCEDURE as well as on its lots, and no BT-23-Procedure — systemic.
- Per the repo's own `fields-1.13.0.json` / `fields-1.14.0.json`: BT-23-Procedure is the predicate-free root
  `/*/cac:ProcurementProject/cbc:ProcurementTypeCode` (mandatory on subtype 4); BT-23-Lot is the predicated Lot path (forbidden
  on subtypes 1-6, E2, X01, X02). OpenAPI promises content field ids "in the source's own vocabulary … the notice's profile names
  which", so the served id contradicts the notice's own SDK.
- Likely cause (skeptic's read of the code): `crates/ingest/src/eforms/index.rs` ALIASES — the comment says an SDK-declared field
  at the target always wins, but `insert_field`'s gap_only check is per exact branch, so the graft adds a predicated sibling
  carrying BT-23-Lot, and `Branch::matching` sorts most-predicated first, so BT-23-Lot beats BT-23-Procedure.
- Not affected: the Tender's nature rows (`project.rs` contract_nature accepts both ids and takes scope from the section).

## Proposed units

(As filed. Superseded by `### Units` in the triage below.)

1. **Fix (S).** A failing test on an SDK 1.13 subtype-4 fixture (root ProcurementTypeCode → BT-23-Procedure), the gap_only /
   matching fix, gate. Check which other aliased fields share the shape.
2. **Re-parse (S/M).** Size the affected notices (SDK versions where the alias fires) and re-parse them so stored content ids
   are right; no fold change expected (verify 0 corrected).

## Triage (2026-10-21, wf triage-575-578)

Read-only: no cargo, no writes, no git state change. Code read at HEAD 969afba. Since the triage's files were written, main gained
e718684 (551's flip: 9 app/store/docs files) and 969afba (.scratch only); crates/ingest is unchanged, so every ingest line below
holds. Files: `.scratch/tender-db/577-triage/` (shadow.py, shadow_extra.py and their .out; read1-4; get1-3; plan-copy.db). The
skeptics' scripts (`577sk/`, `577skeptic/`) and this record's reads and checks (`577rec/`: r1/r2 .sql/.out, over2.py, kinds.py,
rule_c.py) stayed in the session scratchpad; their results are below.

### Evidence

Code path. There is no label table: the served "label" IS the field_id the parser binds.
- index.rs: `FieldInfo.id` is written by `insert_field` (1318-1344) and `insert_extra` (1347-1374). parse.rs:256 binds
  `branches.iter().find_map(|b| b.field.as_ref())`, the first matched branch that has a field. `branches` is flat-mapped per parent
  in `Branch::matching` order, most predicates first (index.rs:809-821, sort at 819). `counterpart()` (parse.rs:397-422) picks a
  Date/Time partner the same way. store and app echo the id (crates/app/src/v1/json.rs:221-286).
- `index::build` order:
  1. Declared fields at their own xpath, gap_only=false (910-911).
  2. The patch tables run only off sdk-0.1 (914-919: "grafting predicate branches over its predicate-free paths would shadow its
     field ids").
  3. EXTRA at its own xpath, gap_only=**false** (920-921).
  4. The direct `insert_extra` calls, gap-only, including the PLDR/renewals guards that skip wholesale on de-1.x (1232-1286).
  5. The ALIASES loop, gap-only (1288-1300).
  Both gap checks look only at the exact branch (1340, 1365).
- BT-23. Every SDK-shaped inventory declares BT-23-Procedure at `/*/cac:ProcurementProject/cbc:ProcurementTypeCode` with no
  predicate, and BT-23-Lot/-Part with a leaf `[@listName='contract-nature']`.
  - The Lot ProcurementProject → `/*/cac:ProcurementProject` alias (index.rs:89-92) grafts BT-23-Lot onto a NEW sibling,
    `cbc:ProcurementTypeCode[@listName='contract-nature']`. Line 1340 sees that branch empty and inserts.
  - `matching` then puts the 1-predicate graft ahead of the 0-predicate declaration. The ALIASES doc comment (84-86, "a field the
    SDK already defines at the target always wins") is false here.
  - BT-23-Part sits on its own exact branch and is kept (47414082 PAR-0000 serves BT-23-Part).
  - By construction BT-23-Procedure still binds where the root code lacks `@listName='contract-nature'`.
- DE-1.x. The empirical inventory declares predicate-free `/*/cac:ProcurementProjectLot/…` paths.
  - The procedure→Lot mirror aliases (112-114) and the EXTRA/direct entries at `[cbc:ID/@schemeName='Lot']` paths add predicated
    twins of those paths, and the twins sort first.
  - Only sdk-0.1 is exempt (919), although its stated reason fits de-1.x word for word.
  - The code already states the de-1.x intent, but one call at a time: 1021-1022 (UBL-ProcessJustificationDescription), 1044-1045
    (UBL-ProcurementAdditionalTypeCode), 1077-1078 (BT-707), 1136-1138 (the BT-773 subcontracting leaf), 1232-1286 (PLDR,
    renewals).
- Exact overwrite (skeptic 1). An EXTRA entry at an xpath the inventory declares replaces the declared FieldInfo, because 921 runs
  after 910-911 without gap-fill. The comment at 1103-1114 says as much for BT-745.
  - This record checked all 75 EXTRA entries independently against every inventory's non-attribute xpaths (`577rec/over2.py`).
    There are exactly 4:
    - BT-92-Lot → UBL-ElectronicOrderUsage (all 17 SDK-shaped inventories);
    - BT-93-Lot → UBL-ElectronicPaymentUsage (all 17);
    - BT-749-Lot → UBL-SelectionCriterionName (1.0–1.11);
    - OPT-050-Lot → UBL-DocumentStatusCode (1.0–1.8).
  - shadow.py skips same-branch pairs, so its outputs never show these.

Enumeration. `577-triage/shadow.py` and `shadow_extra.py` copy `index::build` for the 18 patched inventories: 36 ALIASES, 75
EXTRA, and the direct calls with their guards. Skeptic 1 re-counted all three.
- Nested shadows, where the graft's predicates contain the declared field's, so the graft binds first. EU 1.0–1.15 and DE-2.x:
  - BT-23-Lot over BT-23-Procedure (all 17).
  - BT-772-Lot over BT-67(b)-Procedure at the procedure TQR description (1.0–1.11).
  - BT-1252-Procedure over BT-745-Lot on the Lot ProcessJustification description: over the declared field on 1.12–1.15 and
    DE-2.x, over the direct one on 1.0–1.11.
  - No -Part or -LotsGroup collision.
- 21 more rows have a UBL- catch-all loser (e.g. BT-01(c)-Procedure over UBL-ProcurementLegislationID, the docs.rs:421-427
  example). These are intended.
- DE-1.x: 118 (winner, loser) pairs and 113 distinct declared losers (109 via aliases, 21 via EXTRA/direct; skeptic 1 reproduced
  this).
  - The 109 mirror-alias ids land only on `[schemeName='Lot']`.
  - Part and LotsGroup elements get only the EXTRA-derived twins. The Lot→Part/LotsGroup aliases rewrite `sdk.fields` by each
    field's own xpath, and on de-1.x no such xpath starts with the Lot prefix.
- Non-nested co-matches. Found by skeptic 2 (`577skeptic/order.py`, which copies the `matching` order) and checked here against
  the vendored xpaths:
  - BT-06-Lot and BT-754-Lot (1.5–1.7) and BT-805-Lot (1.3–1.7), mounted at procedure level, carry a predicated
    `ProcurementAdditionalType[…]` step, so they sort ahead of the declared BT-531-Procedure
    `…/ProcurementAdditionalType/cbc:ProcurementTypeCode[not(@listName='transport-service')]`.
  - BT-71/771-Lot (1.8–1.11) and BT-71/771/821-Lot (1.12+) sort ahead of BT-67(a)-Procedure.
  - BT-772-Lot sorts ahead of BT-67(b)-Procedure on 1.12+ and DE-2.x (the graft TQR has 3 predicates, the declared one 1) when a
    procedure TQR holds both an exclusion-ground and a missing-info requirement.
  - In each of these the graft carries a positive discriminator and names the right business term.
  - On SDK-shaped inventories the only same-term pairs where the patch binds first are BT-23, and BT-531-Lot over BT-531-Part on
    1.0–1.7 (the EXTRA `eforms-contract-nature` PAT predicate grafted onto Part; it needs one PAT holding two codes).
  - The rest of shadow.out's overlap section (4,811 pairs) has not been reviewed. Unit 3 counts moves per (old, new) pair instead.
- Graft matched by negations only (`577rec/rule_c.py`, run over all 17 SDK-shaped inventories): exactly 1 different-term pair binds
  first without any positive predicate from the divergence step down. It is BT-772-Lot `TQR[not(CompanyLegalFormCode)]/
  STR[not(TendererRequirementTypeCode[@listName='reserved-procurement'])]/Description` over BT-67(b)-Procedure on 1.3/1.5/1.6/1.7;
  1.0/1.2 and 1.8+ carry a positive missing-info predicate. So every procedure exclusion-ground description on 1.3–1.7 is served
  as "late missing-info description" (R6).

What the projection reads (project.rs):
- TEXTS (240-297) maps UBL-SelectionCriterionName → selection_criterion and UBL-TendererRequirementDescription →
  tenderer_requirement by full id (294-295; `canonical_name` at 9560-9564 tries the full id first, then `stem`, 9252).
  - On DE-1.x lots these two ids are today's WINNERS, over DE1-ProcurementProjectLot-TenderingTerms-SelectionCriteria-Name and
    DE1-ProcurementProjectLot-TenderingTerms-TendererQualificationRequest-SpecificTendererRequirement-Description.
  - Those two DE1 ids have no TEXTS or DE1_FIELD_ALIASES entry (doc comment at 1318, const at 1338-1478). Letting them bind with no
    mapping drops both facts.
- crates/ingest/tests/fixtures/golden/project_apply.snapshot:143 is `4|1|tenderer_requirement|DEU|siehe Formblatt 216|5`.
  Tender 4 is doe/eforms-de-1.1-cn-7d69b0f7.xml (snapshot line 18), and the value is the lot's SpecificTendererRequirement/
  Description (fixture lines 177-180, inside LOT-0000). The golden digests the canonical layer.
- The 19 shadowed DE1 pairs that have a DE1_FIELD_ALIASES entry map to the same stem (`577skeptic/de1loss.py`, re-run). For
  example DE1-ProcurementProject-Name maps to BT-21-Procedure and the lot id to BT-21-Lot; the fold keys by stem and takes the
  scope from the section.
- Other exact-id reads:
  - BT-5141 is in PLACE_COUNTRY_FIELDS at both scopes (9636).
  - Stated totals skip DE-1.x by profile (8891), so 551's exact `BT-27-Procedure` read (8866) is untouched.
  - guid_twin reads DE1 role ids through DE1_FIELD_ALIASES (guid_twin.rs:235).
- BT-23 is read by stem (NATURE_STEMS, 9357), and `contract_nature` accepts both ids. These ids are read nowhere: BT-531, BT-707,
  BT-745, OPT-060, BT-06, BT-754, BT-805, BT-772, BT-67, BT-1252, BT-749, BT-92, BT-93, OPT-050. The other UBL- winners are in
  the `UBL_PARSE_ONLY` test ledger (762-857).
- Storage. Of the 118 DE-1.x pairs exactly one changes table (`577rec/kinds.py`; sdk.rs `decide`, 194-222):
  UBL-DocumentLanguageID (code → notice_codes) becomes DE1-ProcurementProjectLot-TenderingTerms-CallForTendersDocumentReference-
  LanguageID (id → notice_ids).

Served text:
- openapi.json:318 promises "field ids in the source's own vocabulary … the notice's `profile` names which"; the fix makes the
  server keep that promise.
- docs.rs:421-427 (BT-01(c)-Procedure in a LOT) stays true.
- CHANGELOG.md:3-5 requires an entry, because field_id values change on /v1/notices/{id}/content and in notice_* on /v1/sql. None
  exists yet.

Reads. All bounded and run one at a time, with plans probed first on copies of v551/plan.db. No 408 or 429.
- **R1**: newest 3 eforms-de-1.1 notices → 26644178, 26644175, 26644125, all parsed.
- **R2**: the window join → ssh "Connection closed by UNKNOWN port 65535", no result. Not a 408 or 429; whether the endpoint timed
  out behind it was not checked.
- **R3**: R2 retried (PK range plus covering PK seek): notices 47412000-47414200 `LIKE 'eforms:%'`, LEFT JOIN notice_codes at
  PROCEDURE for BT-23-Lot/-Procedure.
  - sdk-1.12 BT-23-Lot 65.
  - 1.13 BT-23-Lot 1,567, none 2.
  - 1.14 BT-23-Lot 561, none 6.
  - BT-23-Procedure 0. Total: 2,193 of 2,201.
- **R4**: newest 40 notices per profile through notices_profile.
  - Root BT-23-Lot: 40 rows each on sdk-1.3, 1.7, 1.10 and de-2.1 (non-root rows 94, 47, 437, 93). BT-23-Procedure 0.
  - de-1.2: DE1-ProcurementProject-Name 40 root rows (correct, the procedure title) plus 117 non-root rows;
    DE1-ProcurementProjectLot-ProcurementProject-Name 0.
- **s1/s2** (skeptic 1): s1 (notice 26644178) returned no rows; s2 is the R5 query.
- **R5** (this record, re-taking s2), using covering PK seeks on notice_id plus notices_profile:
  `SELECT t.field_id, t.section_id='PROCEDURE', count(*), count(DISTINCT t.notice_id) FROM notice_texts t WHERE t.notice_id IN
  (SELECT id FROM notices WHERE profile='eforms:eforms-de-1.1' ORDER BY id DESC LIMIT 40) AND t.field_id IN (the 2 UBL ids, the 2
  DE1 lot ids) GROUP BY 1,2`
  - UBL-TendererRequirementDescription: 118 non-root rows on 22 of 40 notices.
  - UBL-SelectionCriterionName: 50 non-root rows on 11 of 40.
  - 0 root rows, and 0 rows on either DE1 lot id.
- **R6** (this record), plan: IN-list PK seeks, then covering seeks on notice_texts (notice_id) and notice_codes (notice_id,
  section_id). Query: newest 40 sdk-1.5 plus newest 40 sdk-1.7 notices; texts in `ND-ExclusionGrounds#%` sections (the
  procedure STR node), LEFT JOIN codes in the same section.
  - sdk-1.5: code BT-67(a)-Procedure with text BT-772-Lot, 27 sections on 27 notices.
  - sdk-1.7: the same pair, 109 sections on 13 notices.
  - No BT-67(b)-Procedure text and no BT-771-Lot code. Every sampled exclusion-ground description is mislabelled.

GETs (all from the triage; none in this record):
- get1: `tender-db.zebreus.click/v1/notices/47414082/content` → empty body.
- get2: `tenders.zebreus.click/…/47414082/content` → 200. PROCEDURE has `{field_id: BT-23-Lot, list: contract-nature, code:
  services}` and PAR-0000 has BT-23-Part; the other procedure terms carry -Procedure.
- get3: `…/26644178/content` (de-1.1) → LOT-0001/0002 carry DE1-ProcurementProject-Name ('HLS', 'Elektro'), -Description, -ID,
  -ProcurementTypeCode and DE1-TenderingTerms-* ids, and no DE1-ProcurementProjectLot-ProcurementProject-* row.

### Root cause

`index::build` gap-fills the patch tables per exact branch only. A graft whose path carries more predicates than the inventory's
own declaration of the same element lands on a new sibling branch. `Branch::matching` sorts the most-predicated branch first and
parse.rs:256 binds the first field, so the graft's id shadows the declared field at the declared field's own element.
- For BT-23 the grafted leaf predicate holds on every conformant root code, so the declared id is never served.
- For de-1.x, the `[schemeName='Lot']` twins shadow the whole predicate-free lot inventory, because only sdk-0.1 is exempt (919).
- On 1.3–1.7 the BT-772-Lot graft wins on predicate count alone, though all its predicates are negations.
- Separately, the top-level EXTRA insert is not gap-only (921), so 4 EXTRA entries at an xpath the SDK declares replace the SDK id
  outright.

### Size

- **BT-23 root**: nearly every parsed EU 1.0–1.15 or DE-2.x notice whose root carries a contract nature.
  - Measured: 2,193 of 2,201 (99.6%) in R3's window (the other 8 carry no root BT-23), and 40/40 root rows on each of
    sdk-1.3/1.7/1.10/de-2.1 (R4).
  - BT-23-Procedure appears on no sampled eForms notice.
  - The corpus total is not counted (no corpus scan); it is on the order of millions. High confidence on the rate, none on the
    absolute number.
- **DE-1.x lots**: every lot of every DE-1.x notice that carries any of the 113 shadowed elements, in a cohort of about 218k
  notices (DE1_FIELD_ALIASES doc; 218,635 per issue 98).
  - Measured: R4 117 non-root title rows and 0 on the lot id; R5 118 and 50 non-root rows on the two TEXTS-read UBL ids, in 22
    and 11 of 40 de-1.1 notices; get3.
  - Part and LotsGroup are affected only through EXTRA-derived twins. High confidence.
- **1.3–1.7 procedure exclusion-ground descriptions**: all of them (R6: 136 of 136 sections, on 27 of 40 sdk-1.5 and 13 of 40
  sdk-1.7 notices).
- **Exact overwrite**: BT-92/93 on every EU/DE-2 lot that publishes the PostAwardProcess indicators, BT-749 on 1.0–1.11 lot
  selection-criterion names, OPT-050 on 1.0–1.8 document status. Not measured.
- **Canonical layer**: 0 expected changes, but only with unit 2's TEXTS entries. Without them, DE-1.x lots and parts lose
  tenderer_requirement and selection_criterion, and EU 1.0–1.11 loses selection_criterion once BT-749 binds.
- Unit 3's run is the size measurement: it counts moves per (old id, new id) pair.

### Decision D577-1 (owner, 2026-10-21; amended by the skeptics and this record)

**The rule.** At bind time (parse.rs:256 and `counterpart`), take today's first field `f`. If `f` is a patch-table field (alias
graft, EXTRA or direct), and a matched branch carries an inventory-declared field `d` that meets any of the three conditions
below, bind the first such `d`. Otherwise bind `f` as today.
- **(a)** the inventory is eforms-de-1.x;
- **(b)** `d` and `f` name the same business term, i.e. the same id once the scope segment is removed (BT-23-Lot ≡
  BT-23-Procedure);
- **(c)** `f`'s branch carries no positive predicate below the step where it splits from `d`'s, so it matched by negations alone.
  This can be computed at build time from the graft's own xpath remainder.

In addition, the top-level EXTRA insert (index.rs:920-921) becomes gap-only, so no EXTRA entry replaces a declared FieldInfo.
Nothing changes about what is claimed, what is quarantined (the relaxed ambiguity check still sees every candidate) or how
sections are formed. Only the bound id changes.

**What it changes:**
- BT-23-Procedure at every SDK-shaped root.
- The 113 DE-1.x declared ids: 110 lot ids, plus DE1-TenderingTerms-ContractExecutionRequirement-ExecutionRequirementCode,
  DE1-ProcurementProject-ProcurementAdditionalType-ProcurementTypeCode and DE1-ContractingParty-ContractingPartyType-PartyTypeCode.
- BT-67(b)-Procedure for procedure exclusion-ground descriptions on 1.3–1.7.
- BT-92-Lot, BT-93-Lot, BT-749-Lot and OPT-050-Lot.
- In theory, BT-531-Part over BT-531-Lot.

**What it keeps:** every graft id that names the right business term through a positive discriminator:
- BT-1252-Procedure on a Lot direct-award justification, on 1.0–1.11 and 1.12+ alike;
- BT-772-Lot and BT-71/771/821-Lot on missing-info and reserved requirements on 1.0/1.2 and 1.8+;
- BT-06/754/805-Lot at procedure level on 1.3–1.7;
- BT-01(c)-Procedure in a LOT.

**Why:** (a) is the de-1.x intent the code already writes per call (1021-1022, 1044-1045, 1077-1078, 1136-1138, 1232-1286), and it is the
sdk-0.1 rationale (914-918). (b) is a pure scope-suffix fix. (c) applies only where the graft does not identify its own term.

**Alternatives:**
- **D1 (the triage's first draft): a declared field always beats a patch field.** Rejected on skeptic 2's evidence: it flips
  non-nested co-matches to the wrong term.
  - A procedure strategic-procurement code would become BT-531-Procedure ("additional nature") on 1.5–1.7.
  - A missing-info description would become BT-67(b)-Procedure ("exclusion grounds") on 1.12+.
  - A direct-award justification would become BT-745-Lot ("non-electronic submission").
  - Its declared flag also dies when EXTRA overwrites the FieldInfo, so it cannot fix the 4 overwrites.
- **Skeptic 2's D1' ((a)+(b) only).** Leaves the 1.3–1.7 BT-772/BT-67(b) mislabel, which R6 shows is universal there.
- **(B) Skip a graft at build time when a declared predicate-subset sibling exists.** Depends on insertion order, needs a search
  for the divergence step, and misses the non-nested cases.
- **(C) Special-case BT-23.** Leaves the 113 DE-1.x ids, the 1.3–1.7 pair and the 4 overwrites wrong.
- **(D) Exempt de-1.x from the patch tables like sdk-0.1.** Matches the rationale at 914-918, but un-claims DÖE-quirk content
  (the ALIASES entries for issues 78/195), risks unmeasured quarantines, and does not fix BT-23.
- **(E) Relabel at serve time in json.rs.** The notice_* rows on /v1/sql would stay wrong.

### Units

- Unit 1: decide (S). Status: **DONE 2026-10-21** (this section). Blocked by: nothing.
- Unit 2: build (M). Status: ready-for-agent (next). Blocked by: nothing for the code. 551 B6's flip e718684 is on main and
  deployed, U7 has drained and U8 passed (969afba). The deploy follows 551 C7: rebuild and re-gate the switch-off twin d569107 on
  the deployed base (22-35 min), keeping 577's CHANGELOG and docs hunks.
  - index.rs:
    - Add `FieldInfo.declared`, set only by `insert_field` with gap_only=false.
    - Add a `negated_only` flag on patch entries placed on a new sibling.
    - The built index records whether the inventory is eforms-de-1.x (build already reads `sdk.sdk_version`, 919).
    - Make 921 gap-only.
    - Reword the ALIASES doc comment (84-86) and the comment that EXTRA "would insert without gap-fill" (1103-1114).
  - parse.rs: apply the D577-1 rule at 256 and in `counterpart` (397-422).
  - Same-term test: strip the scope segment. Do NOT reuse project.rs `stem()` (9252): it cuts `BT-195(BT-106)-Procedure` at the
    inner dash to `BT-195(BT`, which would make every BT-195 declaration one term.
  - project.rs TEXTS:
    - full id `DE1-ProcurementProjectLot-TenderingTerms-TendererQualificationRequest-SpecificTendererRequirement-Description` →
      tenderer_requirement;
    - full id `DE1-ProcurementProjectLot-TenderingTerms-SelectionCriteria-Name` → selection_criterion;
    - stem `BT-749` → selection_criterion.
    - Lot ids only. The procedure-level `DE1-TenderingTerms-…-SpecificTendererRequirement-Description` binds its own id today and
      is unmapped; mapping it would ADD facts.
  - Tests:
    - (a) An SDK 1.13 subtype-4 notice: the root ProcurementTypeCode gives BT-23-Procedure, the lot keeps BT-23-Lot, the part keeps
      BT-23-Part.
    - (b) doe/eforms-de-1.1-cn-7d69b0f7.xml: the lot Name gives DE1-ProcurementProjectLot-ProcurementProject-Name, the lot TQR
      description gives its DE1 lot id, and the lot LanguageID lands in ids. project_apply.snapshot:143 is unchanged and the golden
      epoch is not bumped.
    - (c) BT-01(c)-Procedure in a LOT is kept.
    - (d) Positive-discriminator grafts are kept:
      - sdk-1.12 Lot ProcessJustification with direct-award-justification → BT-1252-Procedure (same on 1.10);
      - sdk-1.12 mixed procedure TQR → BT-772-Lot on the missing-info requirement;
      - sdk-1.5 procedure strategic-procurement code → BT-06-Lot.
    - (e) sdk-1.5/1.7 procedure exclusion-ground description → BT-67(b)-Procedure; an sdk-1.8 missing-info one keeps BT-772-Lot.
    - (f) An sdk-1.x lot ElectronicOrderUsageIndicator → BT-92-Lot; an sdk-1.10 lot SelectionCriteria/Name → BT-749-Lot, and the
      fold still yields selection_criterion.
    - (g) An index-wide invariant over ACCEPTED: copy the `matching` order plus the D577-1 rule, pin the changed set to the list
      above, and assert that no EXTRA top-level insert replaces a declared FieldInfo. A predicate-subset port of shadow.py would
      miss the non-nested cases.
  - CHANGELOG entry, placed above 551 B6's:
    - EU/DE-2 root nature: BT-23-Lot → BT-23-Procedure.
    - 1.3–1.7 procedure exclusion-ground description: BT-772-Lot → BT-67(b)-Procedure.
    - Lot BT-92-Lot and BT-93-Lot (were UBL-ElectronicOrderUsage and UBL-ElectronicPaymentUsage).
    - BT-749-Lot on 1.0–1.11 (was UBL-SelectionCriterionName).
    - OPT-050-Lot on 1.0–1.8 (was UBL-DocumentStatusCode).
    - DE-1.x lot values under DE1-ProcurementProjectLot-* (were DE1-ProcurementProject-*, DE1-TenderingTerms-*,
      DE1-TenderingProcess-*, UBL-* and a few BT-/OPT- ids). The lot LanguageID moves from notice_codes to notice_ids.
    - Each notice changes once it is re-parsed (unit 3). Canonical tenders are unchanged.
  - Docs (the filed unit 3, folded in): one sentence in /docs notice-content and in the openapi content description. A value's
    scope is its section, and a gap-filled graft can carry another scope's suffix where it names the right term (BT-01(c)-Procedure
    in a LOT, BT-1252-Procedure on a lot direct-award justification). docs.rs and openapi.json are flip files, hence the C7 twin
    rebuild.
  - Gate with ops/check.sh.
- Unit 3: re-parse (M/L). Status: ready-for-agent. Blocked by: unit 2 deployed; 539's drain finished; the single-writer slot behind
  576 U3 and 575 as the board orders them; not on a 551 U9 check morning.
  - `reparse` matches exact profiles (`n.profile IN (…)`, store/src/lib.rs:3432-3447). Profiles: eforms:eforms-sdk-1.0/1.2/1.3/
    1.5…1.15, eforms:eforms-de-2.0/2.1, eforms:eforms-de-1.0/1.1/1.2 (sdk.rs:46-97). Not sdk-0.1, which has no patch tables.
  - `reparse` has no dry mode (Spec::Reparse {profiles, packages, after}, supervisor.rs:280, 4192-4221).
    - Size it first with bounded per-profile counts (plan-probed, split by id window), or with a `packages`-capped first slice.
    - Re-parsed notices land with projected=0, and the paired `project rebuild=false` folds them.
  - Bar: the paired projection reports 0 corrected. Any correction stops the run (it means a mapping is missing), and its tender ids
    go on 551 C7's switch-off list.
- Unit 4: verify and read back (S). Status: ready-for-agent. Blocked by: unit 3.
  - Run the Verify above, then re-take:
    - R3: expect BT-23-Procedure 2,193 and BT-23-Lot 0 at PROCEDURE.
    - R4 de-1.2: lot rows on DE1-ProcurementProjectLot-ProcurementProject-Name.
    - R5: rows on the two DE1 lot ids, 0 on the UBL ids.
    - R6: BT-67(b)-Procedure.
    - The overwrite probe: an EU lot from R3's window that publishes ElectronicOrderUsageIndicator serves BT-92-Lot (pick it from a
      bounded notice_integers read before unit 3, so the open state is recorded).
  - Check that 26644178 LOT-0001 serves DE1-ProcurementProjectLot-ProcurementProject-Name 'HLS' and that tender 4's
    tenderer_requirement is still served. Then set `Status: **DONE <date>**`.

### Interactions

- **551 B6.** No longer a held patch.
  - The flip e718684 is on main (committed 2026-10-21 10:21Z) and deployed. U7 drained and U8 passed (969afba, 10:55Z).
  - The switch-off twin d569107 reverts the 9 flip files to d209375: CHANGELOG.md, openapi.json, docs.rs, v1/sql.rs,
    supervisor.rs, canonical.rs, docs/operations.md, app/tests/admin.rs, store/tests/head_election_agreement.rs.
  - 577's code (crates/ingest: index.rs, parse.rs, project.rs, tests) is in neither commit. Its CHANGELOG entry and docs sentence
    overlap the twin's files, hence C7.
  - C9 (path-W inputs): if unit 3 reports 0 corrected, it changes no winners, org merges, rounds, stated totals (DE-1.x is
    excluded, 8891) or lot numbers. It still stays off U9 check mornings.
- **539.** Its drain is next: repair-notice-instants plus the DÖE R3 refold over sdk-0.1 and de-1.0…de-2.1, about 1.16M notices.
  Unit 3 re-parses de-1.x and de-2.x and pairs a projection, so it runs after that drain; that keeps 539's read-back clean and the
  writer single. 539 U3 is deployed, so the re-parse applies the same instant logic.
- **575 / 576.** Same single writer, no data dependency: 577 touches no Organization or Touchpoint field, and no ORG path is in the
  shadow lists.
- **530 (DONE).** Unaffected.
  - 530-T runs in `mentions_and_dropped`.
  - The shadowed DE1 role refs (AppealInformation/Receiver/Mediation, Employ/Environ/FiscalLegis, TenderRecipient) map to the same
    OPT-301 ids in DE1_FIELD_ALIASES, and `guid_twin_role_ref` reads through that map (guid_twin.rs:235).
  - Its tests hand-build today's shadowed shape and stay green. Expect 0 corrected.
- **Minor.** After unit 3, the data-quality report's unmapped_fields (crates/ingest/src/data_quality.rs, keyed by exact id) moves
  counts from DE1 procedure/UBL ids to DE1 lot ids. Cosmetic.

### What the skeptics corrected (each checked against code or files before recording)

- **"Projection unaffected / 0 canonical changes" was false for the triage's D1.** UBL-TendererRequirementDescription and
  UBL-SelectionCriterionName are today's DE-1.x lot winners and are read by full id (TEXTS 294-295). Their DE1 losers are mapped
  nowhere. R5 re-measured the exposure: 118 and 50 rows, in 22 and 11 of 40 de-1.1 notices. Unit 2 now maps both lot ids.
- **project_golden goes red without that mapping.** project_apply.snapshot:143 is tender 4's lot tenderer_requirement, from fixture
  lines 177-180. Skeptic 2's "canonical tables store no source field_id, so the golden stays green" is wrong and has been dropped.
- **A second mechanism: the exact EXTRA overwrite** (921 runs after 910-911). Its 4 pairs are invisible to shadow.py. It is folded
  into 577 (same contract, same re-parse cohort) as gap-only EXTRA plus a BT-749 TEXTS entry.
- **D1 is replaced by D577-1.** D1 would relabel non-nested co-matches to the wrong term (skeptic 2's finding, checked here against
  the 1.3/1.5/1.7/1.8/1.12/1.13 xpaths).
  - Skeptic 2's D1' became (a)+(b). This record added (c) after R6 showed the 1.3–1.7 BT-772/BT-67(b) mislabel on every sampled
    section.
  - Skeptic 1's 1.0–1.11 vs 1.12+ BT-1252/BT-745 inconsistency disappears: both keep BT-1252-Procedure.
  - "Exactly 3 pairs" undercounted what a bind-order rule touches. The non-nested pairs are listed above.
- **Counts and wording:**
  - "22 UBL- catch-all rows" is 21, plus BT-1252-Procedure over the direct BT-745-Lot (1.0–1.11).
  - "40/40 de-1.2" counted the correctly labelled root rows. The shadow evidence is 117 non-root rows against 0.
  - Part and LotsGroup get only EXTRA-derived twins, not "up to 113".
  - "BT-23-Procedure served on no eForms notice" is a sample result, not a guarantee.
  - DE1_FIELD_ALIASES is at doc 1318 / const 1338-1478, not 1319-1422.
- **CHANGELOG** also names the LanguageID move from notice_codes to notice_ids.
- **The invariant test** (the triage's test (e), now (g)) copies the `matching` order and the bind rule, not predicate subsets.
- **551's state was stale.** e718684 and 969afba are committed. Unit 2's code no longer waits on 551; only C7's twin rebuild
  remains, for its deploy. Only 539's drain gates unit 3.
- **Also corrected in this record:**
  - `reparse` has no dry mode (the triage assumed a "dry plan"), so unit 3 sizes with bounded counts instead.
  - Same-term matching must not reuse `stem()`, because of the BT-195 forms.
  - The 1.3–1.7 BT-772 rule (c) is new here, with R6 as its evidence.

**551 W-on drain protocol (2026-10-23, `551-lot-sum/b6/decision-w-on-drains.md`).** Unit 2's deploy joins the batch only if
551's static check W7 is empty. W7 intersects this issue's full shadow/overlap pair list (the 4,811 unreviewed pairs
included) with every field id or stem the fold reads: Amount, LotResult, StatedTotal incl. BT-27-Procedure, published lot
numbers, winners, tenderer refs. It costs 0 statements and is recorded in 551. If the intersection is not empty, unit 2's
deploy and unit 3 wait until after 11-04. Unit 3 is class Z (bar: 0 corrected) and runs in slices. The first slice is
packages-capped to time the rate, and each slice ends ≥ 2 h before the next tick. Unit 3 runs after 539's drain, after 576 U3
AND after 546 U4. That last condition is recorder's check R1: an undrained 546 U3 re-places mixed legacy/eForms chains in the
paired project and would trip the 0 bar. If 546 U4 slides past 11-04, unit 3 either waits behind it, or runs as class N per
slice with this bar read as "0 corrections outside the place-only and order-only NOTE classes", recorded here before the
first slice. A slice passes when corrected = 0, the cursor is unchanged (or one E1 statement shows no tender row), and only
its own jobs ran. Any correction triggers this issue's own stop, then gd5 class N on that slice, with every NEW/MOVED read and
its ids registered in 551's R. Cost: 0–1 statement per clean slice. **Next step:** unit 3 is now also blocked by 576 U3, 546
U4 and W7.

**551 W7 result (2026-10-24, wf_52627394-803).** The intersection is NOT empty.
- Set A has 631 ids. They are the 125 (old, new) pairs 577 U2 can move, from a port of `index::build` + `Index::bind`
  that reproduces every pin of the gated tests, plus shadow.out's 155 shadow rows, its 4,811 overlap pairs and
  shadow_extra's 24 rows.
- Set B holds the fold's 302 field-id keys, classed W / W-ind / nonW.
- W hits from code moves (all eforms-de-1.x):
  - `DE1-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount` (→ BT-27-Procedure via normalise_de1)
    → its Lot id (BT-27-Lot);
  - the framework maximum (BT-271);
  - `DE1-ProcurementProject-Name` → its Lot id (BT-21, the lot title R6 reads).
- 14 further W hits come from the overlap list only (BT-137/27/271/21-*, BT-195(BT-720)): not moves the code makes.
- The skeptic rebuilt set A and the literal scan independently and found no hit the analyst missed. It also found that
  the commit's test and the port share a blind spot (parse.rs's select_by_name relaxation); that adds 53 pairs, none
  of them W-tier.
- Consequence: 406529a was reverted on main (b3effc2) so that 551's batched deploy (576 U2, 578 U2, 579 U4, 539's
  caveat retirement) does not ship it.
