# 576 — a party role whose eForms reference names a touchpoint (TPO-…) is dropped from the tender's parties

Status: ready-for-agent — triaged 2026-10-21 (wf triage-575-578: measurer + 2 adversarial skeptics; 4 + 2 bounded /v1/sql reads,
0 GETs; skeptic-corrected, `## Triage (2026-10-21 …)` below). Reproduced: `bind_organizations` silently drops every OPT-301 Lot/Part
role whose target is a Touchpoint (TPO-…) section (project.rs:6115-6118). Window 47408000-47415999: 1,179 TPO refs dropped on
345-374 of 7,122 eForms notices (4.8-5.3%; corpus ≈160-175k notices, low confidence); ≥73 (6%, lower bound) print the parent
ORG's own name, 1,086 another (review roles: mostly tribunals, a minority buyer departments or junk). Omission only, no wrong data
served. **D576-1 DECIDED**: same-name touchpoint → the parent ORG's organization, anchored at the parent section; different-name →
its own identifier-less mention behind the 510/511/520/525 rules plus a touchpoint name gate (section-id shapes, URL/www/@, portal
labels); unreferenced touchpoints mint nothing; touchpoint mentions stay out of `buyer_fix` and `buyer_side_mentions`. NEXT: unit 2
build (project.rs + tests). 551 U7/U8 have passed, so code may land, but its deploy first re-bases 551's switch-off twin (551 C7).
Then unit 3 drain (`refold-sections` kinds=[Touchpoint]) after 539's drain. Was: needs-triage — filed 2026-10-19 from the hourly audit (workflow wf_90e506fc-209, auditor + skeptic, confirmed;
`.scratch/tender-db/audits/2026-10-19-recent-tenders/verify_award.md`). Next: unit 1, size how many role references target a
touchpoint (by role and platform) and decide what a touchpoint-targeted role should serve.
Kind: data quality (parties: omission; no wrong organization is served)
Relates to:
- 530 (A7 / Build 1b): its role guard reads OPT-300/OPT-301 references; a touchpoint target is never an ORG section there either.
- 368 (unmapped source vocabulary dropped silently), 456 (each party serves its own published name).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/8851944 | jq -c '[.parties[] | [.role, .organization_id]]'

- **done**: unit 1's decision is applied: either line 1 serves a review-body party for LOT-0000 (as decided), or the docs say
  plainly that a role whose reference names a touchpoint is not served, and why.
- **open (2026-10-19)**: `[["Procedure-Buyer",6614],["Tenderer",12032625]]` — no Lot-ReviewOrg.

## What was found

- Tender 8851944 (TED 00722535-2026, notice 47411306, eForms subtype 29, ES): LOT-0000 carries `OPT-301-Lot-ReviewOrg = TPO-0001`
  (an id-ref, scheme `touchpoint`). TPO-0001 is a Touchpoint section under ORG-0001 with BT-500 "Tribunal Administrativo Central
  de Recursos Contractuales (TACRC) o Tribunal Superior de Justicia (TSJ).", 28000 Madrid, ES300. The served parties hold only the
  buyer and the winner; the review body is missing. Re-read byte-identical (version 1, one notice).
- Systemic: notice 47413157 / tender 8851996 (ES) loses Lot-ReviewOrg and Lot-AddInfo on 4 lots the same way; every
  ORG-targeted role on the audited tenders (8851923, 8852005, 8852009) is served.
- Cause: `crates/ingest/src/project.rs` (bind_organizations, ~6111): "A reference to something that is not an Organization
  section (a touchpoint, a lot, a result) is notice-layer detail, not a party" — the role is skipped when its target is not a
  minted ORG section. No public doc says so (/docs and OpenAPI list Lot-ReviewOrg with no touchpoint caveat).
- Design question for unit 1: a touchpoint is a contact point OF its parent organization, but here the publisher used the
  buyer's touchpoint to print a different body (the review tribunal). Binding the role to the parent ORG would serve the buyer
  as its own review body (530's AOK shape); minting a party from the touchpoint's own name would serve the tribunal.

## Proposed units

1. **Size and decide (S).** A bounded census of `notice_ids` role references whose target section kind is a touchpoint (by role,
   source, platform/country), plus a read of 20 salted cases: is the touchpoint's printed name the parent org (same body) or a
   different body? Decide: (a) bind to the parent ORG when the names agree, (b) mint a party from the touchpoint's own name and
   address, (c) leave out and document. Record the decision.
2. **Build + drain (M)** per the decision (a refold of the affected notices), or the docs sentence.

## Triage (2026-10-21, wf triage-575-578)

Read-only: no cargo, no writes, no git state change. Code read at HEAD 969afba (main = 551 flip e718684 + its U7/U8 records;
vps/main = e718684). `git diff 9d1b4ca..HEAD` is empty for project.rs, project/, partyname.rs, eforms/index.rs, guid_twins.rs
and read.rs; canonical.rs and supervisor.rs changed in the flip, not at the lines cited. Reads: `.scratch/tender-db/576-triage/`
(r1-r4 `.sql`/`.out`, plan.db). The skeptic's k1/k2 outputs stayed in the session scratchpad (`576skeptic/`); their results are below.

### Evidence

Code path:
- project.rs:5683-5692: every id-ref outside the `ojs` scheme with a `role_name` goes to `raw_roles` as (scope, section, role,
  target), so `OPT-301-Lot-ReviewOrg = TPO-0001` is collected. `role_name` (10235-10244) strips `OPT-300-`/`OPT-301-` and never
  checks the target's kind.
- project.rs:5923, 5948-5951 (`mentions_and_dropped_gated`): eForms `mention_kinds = [Organization]`, so a Touchpoint section
  mints no mention. `nested_org_aliases` (7048) runs over Organization only, so no TPO is aliased to its ORG.
- project.rs:6115-6118 (`bind_organizations`): `let Some(&organization_id) = by_section.get(target.as_str()) else { continue };`
  is the drop; its comment calls a touchpoint "notice-layer detail, not a party". No counter, doc or CHANGELOG line says so.
- project.rs:6119-6123 → canonical.rs:42934-42943: a party row's `mention_section_id` is the role's target verbatim.
  canonical.rs:404-415 FKs (mention_notice_id, mention_section_id) → organization_mentions; store/lib.rs:158 sets
  `PRAGMA foreign_keys = ON`. read.rs:3313-3320 (PARTIES_SQL) + `mention_name_above` (3368) serve the nearest mention above a
  mention-less anchor.
- `party_name_field` (4693-4699: BT-500-Organization-Company + legacy fields) and `drop_decision` (4914-4938, Organization only):
  no touchpoint name or country reaches the parent's mention today, so nothing leaks. BT-16-Organization-* is read nowhere in ingest.
- eforms/index.rs:898-906: the kind comes from the node id, so ND-Touchpoint → `Touchpoint`; its id is OPT-201 (TPO-xxxx); its
  parent is the enclosing ORG section.

SDK (`crates/ingest/sdk/fields-1.15.0.json`; skeptic 1 found the same split in every vendored version):
- `idSchemes ['ORG','TPO']`, 20 fields: OPT-301-{Lot,Part}-{AddInfo, DocProvider, EmployLegis, EnvironLegis, FiscalLegis, Mediator,
  ReviewInfo, ReviewOrg, TenderEval, TenderReceipt}.
- ORG only: OPT-300-{Procedure-Buyer, Procedure-SProvider, Contract-Signatory, Tenderer}, OPT-301-LotResult-{Financing,Paying},
  OPT-301-Tenderer-{MainCont,SubCont} (and ReviewBody/ReviewReq in older SDKs). OPT-302 is the UBO reference.
- A touchpoint has BT-16/500/502/503/505/506/507/509/512/513/514/739-Organization-TouchPoint and no BT-501 identifier.

Exhibits (saved bodies under `.scratch/tender-db/audits/2026-10-19-recent-tenders/raw/`, no new GETs):
- 47411306 / tender 8851944: LOT-0000 Lot-ReviewOrg → TPO-0001, a Touchpoint under the buyer ORG-0001 (DG Racionalización), BT-500
  "Tribunal Administrativo Central de Recursos Contractuales (TACRC) o Tribunal Superior de Justicia (TSJ).". Served:
  [Procedure-Buyer 6614, Tenderer 12032625]. The 2026-10-19 recheck was byte-identical (verify_award.md).
- 47413157 / 8851996: ReviewOrg ×4 → TPO-0001, the Madrid tribunal (another body); AddInfo ×4 → TPO-0002, printed with ORG-0001's
  own name (the same body). Both shapes occur in one notice.
- Controls 47410357, 47413608, 47414082: every ORG-targeted role is served.

Reads: 6 bounded /v1/sql, all exit 0, no 408 or 429. Each plan was probed first on a copy of v551/plan.db: a PK range on
notice_ids, CROSS JOIN PK seeks on notice_sections and notice_texts ×2, then notice_codes; R4 seeks organizations_name_norm_id.
- **R1**: notice_ids 47408000-47415999, is_ref, `OPT-30%`, role × target prefix; 41 rows. 1,179 TPO-targeted refs:
  - Lot-ReviewOrg 672 refs / 345 notices, against 16,432 / 6,201 to ORG (5.3%);
  - ReviewInfo 329 / 172 (4.5%); Mediator 54 / 36 (4.6%); AddInfo 65 / 44 (1.1%);
  - TenderReceipt 26 / 15; DocProvider 17 / 13; TenderEval 10 / 9; Employ/Environ/FiscalLegis 1 / 1 / 2; Part-ReviewOrg 2 / 2.
  - Procedure-Buyer is on 7,122 notices.
- **R2**: TPO refs × parent-ORG country × name agreement (`lower(trim())`, ordinal 0 only); 50 rows, 1,159 of the 1,179 refs.
  - Same name 73 (6.3%), different 1,086.
  - Review family (ReviewOrg, ReviewInfo, Mediator): 1,010 different / 40 same. That is 1,008 / 40 on Lot roles plus Part-ReviewOrg's 2.
  - By parent country: ES 701, NL 214, PL 125 (97 from one 97-lot KIO notice, 47415594), DE 59, IT 35, FR 18.
- **R3**: the top 60 different-name groups by refs (589 refs, 542 of them review-family). These read by ref count, not a
  salted draw, so ES multi-lot notices weigh heavily.
  - Tribunals and courts, 467 refs: TACRC, Aragón, Canarias, Andalucía, Madrid, Catalan, CyL, Cádiz, KIO, Rechtbank
    Noord-Holland/Limburg/Midden-Nederland, TAR Calabria, Vergabekammer BW.
  - Not tribunals but in review roles, 75 refs: MAZ contracting departments (ReviewInfo 28+22+4), "Mutua Montañesa" against
    "Dirección General de Mutua Montañesa" (6+6), "Stadt Fürth - Zentrale Vergabestelle" (ReviewInfo 5), "ORG-0001" (4).
  - Other roles: "Agentur Wieseke" (AddInfo 6), Stadt Fürth (×5 each in TenderReceipt, TenderEval, DocProvider, AddInfo), and 21
    refs of NL TenderNed prose as AddInfo/TenderReceipt: "De inschrijvingen dienen ingediend te worden via www.tenderned.nl", "Voor
    inlichtingen over de aanbesteding kunt u ons bereiken via de aanbesteding op www.tenderned.nl", "Vragen kunt u stellen via
    TenderNed …", "Indiening Inschrijving … digitale kluis van TenderNed".
- **R4**: organizations with name_norm 'tribunal administrativo central de recursos contractuales'; 25 rows.
  - Provisional ES 3955 holds 77,706 mentions; it is the lowest-id ES identifier-less row.
  - Identified rows: 2989 (3,717), 2926 (590), and more. Country-less provisional 1167339 holds 1,214.
- **k1** (skeptic 1): Touchpoint sections in the window, a covering seek on notice_sections_kind_notice: 567 sections on 374 notices.
- **k2** (skeptic 1): DE1-* id refs with TPO- values in the window: 0 rows.

### Root cause

`bind_organizations` (project.rs:6115-6118) resolves a role's target only through `by_section`. That map holds only the notice's
minted Organization sections (`mention_kinds = [Organization]`, 5923). Only `nested_org_aliases` widens it, and that also runs over
Organization only (7048). The SDK lets each of the 20 OPT-301-{Lot,Part}-* roles above point at a Touchpoint (`['ORG','TPO']`), and
the parser files that section as kind `Touchpoint`, a child of its ORG. A TPO target is in neither map, so the `let … else
{ continue }` drops the role with no count.

The design gap behind it is what a touchpoint is. By spec it is a contact point of its parent organization. In practice publishers
use the buyer's touchpoint to print a different body, usually the review tribunal: 94% of TPO refs carry a name that differs from
the parent's. So neither "it is the parent" nor "it is a separate party" holds unconditionally.

### Size

The window is notice ids 47408000-47415999, 7,122 eForms notices with a buyer, roughly 1-2 days of ingest around the 2026-10-19 audit.
- 1,179 TPO-targeted role refs are dropped. They sit on 345-374 notices (4.8-5.3%): at least Lot-ReviewOrg's 345, and at most the
  374 that carry any Touchpoint section (k1). So almost every touchpoint present is a referenced one, and unit 3's cohort is about
  the size of the affected set.
- By parent country: ES 60%, NL 18%, PL 11% (mostly one notice), DE 5%, IT 3%.
- Both named tenders are affected: 8851944 (1 role) and 8851996 (8 roles on 4 lots).
- Corpus, low confidence: about 160-175k notices and about 0.5M role refs. That is one window scaled to issue 228's ~3.3M eForms
  notices, and the rate moves with era and SDK adoption. The exact count comes from unit 2's Report counter during unit 3's drain,
  not from a box scan.
- The same-name share (73, 6%) is a lower bound. It was measured with `lower(trim())` on ordinal 0; the resolver's normaliser over
  every language variant can only match more.
- The tribunal share is not measured. In R3's 542 review-family refs, 467 (86%) are tribunals. Over all 1,050 review-family refs it
  is at most about 89% (≤ 935), and the 468 different-name refs beyond row 60 are unread.
- Org layer: a plain "Tribunal Administrativo Central de Recursos Contractuales" + ES binds the existing 3955. The lead exhibit's
  compound "TACRC … o TSJ." does not; it mints or reuses a compound name-keyed row. Suffixed names ("- Secretaria") do the same.
- Wrong data served today: none. This is an omission only. Severity: low.

### Decision D576-1 (owner, 2026-10-21; amended by the skeptics)

A role reference whose target is a Touchpoint section is resolved through the touchpoint's own published identity and is never
skipped silently.
- **(i) Same normalised name as the parent ORG.** Compare BT-500-Organization-TouchPoint with BT-500-Organization-Company using
  the resolver's name normaliser over any language variant. On a match, the role binds the parent Organization's organization (the
  issue-259 alias shape). The party row is anchored at the parent ORG section: unit 2 overrides `section_id: target`
  (project.rs:6122) for a TPO target, because the parent section holds the mention row the FK needs. This is 0 wrong by
  construction; ≥73 of 1,159 refs in the window.
- **(ii) Different name.** The role binds a mention minted from the touchpoint: name and variants from
  BT-500-Organization-TouchPoint, no identifier. It is resolved by name + country like any identifier-less ORG party and is subject
  to `drop_decision`'s 510/511/520/525 classes. Country is the touchpoint's BT-514; when that is absent, unit 2 decides and pins the
  fallback. The proposal is the parent ORG's BT-514, because R2-R4 assumed the parent's country, and the counter should report how
  often it is used. BT-16-Organization-TouchPoint is not read, the same as BT-16-Organization-Company today. Window: 1,086 refs.
- **(iii) Only role-referenced touchpoints mint.**
- **(iv) Touchpoint name gate (skeptic 2).** A different-name touchpoint is dropped (it binds nothing and is counted) when, besides
  the existing classes:
  - its name is shaped like a section id (`^(ORG|TPO|UBO)-\d+$`);
  - it carries a URL, `www.` or `@`;
  - or `role_census::portal_label` hits it (NameList::Portal/Platform: `tenderned`, `digitaal via`, …).

  The existing nets do not catch these names. `partyname::classify` (1025-1120) has no stem for them, and `blank_raw` (1184-1187)
  drops only punctuation-only or all-zero names. So the 21 NL prose refs and the 4 "ORG-0001" refs in R3 alone would otherwise be
  served as party names (W). Bar for the gate on R3's 60 groups: it drops exactly those 25 refs and no tribunal.
- **(v) Census isolation (both skeptics).** Touchpoint mentions never reach `role_census::buyer_fix` / `notice_parties_from` or
  `buyer_side_mentions`. They are returned on a separate list that only Phase 1 recording and `bind_organizations` read, or they
  are filtered by section kind.
  - Why: `notice_parties_from` (role_census.rs:604-641) turns any mention whose section is a role target into a Party. ROLE_KINDS
    (103-134) makes AddInfo, TenderReceipt and TenderEval BuyerShaped and DocProvider DocsProvider.
  - judge's `real-buyer-elsewhere` (682-696) would then make `review-body-name` decisive (705) on a review-body-named buyer.
    `promotable` (945-965) passes names like "Agentur Wieseke" and "Stadt Fürth - Zentrale Vergabestelle", so `fix_from` (968-985)
    could drop the published buyer and promote a touchpoint to Procedure-Buyer.
  - That moves the served buyer (`NoticeState::read` applies `buyer_fix`, project.rs:5772-5774), and 369's `buyer_key` and 481's
    tokens with it (`buyer_side_mentions`, project.rs:10377-10395).
  - Promoting a touchpoint is not decided here. If it is ever wanted, it is its own decision with its own size.

Alternatives weighed:
- (a) Bind every TPO ref to the parent ORG: rejected. It would serve the buyer as its own review body on the other-body refs: ≥467
  in the window (R3's tribunals alone), up to ≤935. That is wrong data and 530's AOK shape, in place of today's harmless omission.
- (b alone) Mint from the touchpoint with no same-name alias: rejected. It would mint self-duplicates of the buyer for the ≥6% that
  print the buyer's own name (the 47413157 AddInfo shape).
- (c) Leave out and document: rejected as the end state. It keeps a silent, systematic loss of about 5% of review bodies (60% of it
  Spanish) although the SDK explicitly allows TPO targets. It stays the fallback only if the build is abandoned.
- Role-kind split (review roles mint; buyer-shaped roles bind the parent whatever the name): rejected. External bodies sit in
  buyer-shaped roles too ("Agentur Wieseke" as AddInfo).
- Accepted cost: a buyer department printed under another name ("Stadt Fürth - Zentrale Vergabestelle", the MAZ contracting
  departments) becomes its own name-keyed party, as it does when a publisher prints it as an ORG section. Per 456 each party is
  served under its own published name, so it reads as N, not W.

### Units

- Unit 1: size and decide (S). Status: **DONE 2026-10-21** (this section). Blocked by: nothing.
  The tribunal share rests on R3's top 60 by refs, not on the 20 salted cases the unit asked for. The decision does not depend on
  it, because (i)/(ii) split on name agreement, not on share. Unit 4's stratified salted read measures the share.
- Unit 2: build (M). Status: ready-for-agent. Blocked by: nothing for code (551 U7 drained and U8 PASS, 969afba). Its DEPLOY first
  rebuilds and re-gates 551's switch-off twin on the new base (551 decision C7).
  - Where: crates/ingest/src/project.rs, which is in neither e718684 nor d569107.
  - (a) Same-name alias (i), anchored at the parent ORG section.
  - (b) Different-name mint (ii) through `drop_decision` plus gate (iv); a dropped touchpoint binds nothing.
  - (c) An unreferenced touchpoint mints nothing.
  - (d) Census isolation (v).
  - Do NOT just add `Touchpoint` to `mention_kinds` or `party_name_field`. `nested_org_aliases` would alias every TPO to its ORG,
    and `enclosing` would fold the tribunal's name into the buyer's mention variants: a wrong name on the buyer.
  - (e) A Report counter `touchpoint_roles {aliased, minted, dropped, country_fallback}`.
  - (f) Tests:
    - 47411306: ReviewOrg mints the tribunal's mention.
    - 47413157: TPO-0001 is minted; TPO-0002 is aliased to ORG-0001.
    - Dropped by (iv): the four NL prose strings, "ORG-0001", and a void name.
    - No mention for an unreferenced TPO.
    - A review- or platform-named buyer beside a touchpoint AddInfo leaves `BuyerFix`, `buyer_key` and the guard tokens unchanged.
    - `guid_twin_section_roles` output unchanged.
    - guid_twins planner step 4d: its role mix gains only the (i) rows on the parent section.
    - The FK holds for (i) and (ii) rows.
  - (g) Docs (openapi `parties`, /docs Tender detail, CHANGELOG) say a touchpoint-targeted role is served and how. These files are
    in the flip and in its switch-off twin, so they land only once the twin is rebuilt on a base that contains them.
- Unit 3: drain (M). Status: ready-for-agent. Blocked by: unit 2 deployed; 539's drain finished.
  - Use the existing job, not a new walk: `refold-sections` with kinds=["Touchpoint"] (Spec::RefoldSections, supervisor.rs:744,
    arm 10316-10338). It re-queues every carrier (`notice_ids_with_section_kind`) and stamps its tenders epoch-stale. The
    incremental fold then re-mints mentions for the changed set (`mentions_and_dropped`, project.rs:3397-3401), so old notices
    gain their touchpoint mentions.
  - It has no dry run and no `expect` guard. Size the cohort first with one bounded single-table COUNT over the
    notice_sections_kind_notice prefix kind='Touchpoint' (plan-probed; split it by notice-id windows if it nears the 15 s cap).
  - Not on a 551 U9 check morning. Record its tender ids for 551 C7's switch-off list.
  - Afterwards: read the counter, then re-read 530's R3/recurrence census and expect the (i) delta (see Interactions).
- Unit 4: verify + read-back (S). Status: ready-for-agent. Blocked by: unit 3.
  - Verify: 8851944 serves a Lot-ReviewOrg party for LOT-0000 (the compound TACRC/TSJ name). 8851996 serves ReviewOrg ×4 (the
    Madrid tribunal) and AddInfo ×4 (the buyer's own organization).
  - Then read 30 minted touchpoint parties, salted `576-u4-` and stratified to include the NL AddInfo/TenderReceipt cohort, for
    R/N/W with a 0-W bar. A department of the buyer minted under its own name counts as N.
  - Then close with `Status: **DONE <date>**`.

### Interactions

- **551 B6.** The flip e718684 is on main and deployed. U7 drained (3,467 fired as predicted) and U8 verify PASSed (969afba). The
  switch-off twin d569107 (hold/551-switchoff) reverts all 9 flip files to d209375: CHANGELOG.md, openapi.json, docs.rs, v1/sql.rs,
  supervisor.rs, canonical.rs, docs/operations.md, app/tests/admin.rs, store/tests/head_election_agreement.rs.
  - 576's code (project.rs) conflicts with neither commit.
  - 576's doc sentence overlaps three of those files. Per C7, any deploy during the U9 14-day watch rebuilds and re-gates both
    twins on the deployed base (22-35 min).
  - C9's standing rule covers drains that change path-W inputs (winners, org merges, rounds, stated totals, lot numbers). 576
    changes none of these: it touches only non-winner OPT-301 roles, and the LotResult/Tenderer refs are ORG-only. But its drain
    re-projects the cohort's tenders and writes change rows, which enlarge C7's switch-off id list. So: no drain on a U9 check
    morning, and its tender ids are recorded.
- **539.** Its drain (repair-notice-instants + the DÖE R3 refold) was waiting on 551 U8 and is next. 576's drain queues after it:
  one writer, and both re-project tenders.
- **530 (DONE).**
  - The 530-T gate (`guid_twin_gate`) runs in `mentions_and_dropped_gated`, where touchpoint minting goes. Touchpoint mentions
    carry no identifier, so 530-T is inert on them.
  - A7: `guid_twin_role_ref` (guid_twin.rs:233-243) and `guid_twin_section_roles` (252-260) read ALL raw OPT-300/301 refs, TPO
    targets included (keyed TPO-xxxx). They stay on the raw refs and are not fed the alias.
  - `guid_twin_parties_sql` (guid_twins.rs:446-452), which planner step 4d reads (1629-1645), WILL see the (i) rows on the
    buyer's ORG key. That is the same shape as today's ORG-targeted AddInfo → buyer, about 73 refs per window. (ii) rows are keyed
    TPO-xxxx, outside any twin group. Expect that delta when 530's R3/recurrence census is re-read.
- **483/484 role census + `buyer_fix`, 369/481 `buyer_side_mentions`.** Affected unless (v) holds; see D576-1 (v).
  `buyer_role_refold_window` counts move only if (v) is broken.
- **510/511/520/525 drop rules.** (ii) runs through them. (iv) adds the touchpoint-only classes, because the lexicons miss
  touchpoint prose.
- **456** (each party serves its own published name): (ii) follows it. **368** (silent drops): this was one; the counter closes it.

### What the skeptics corrected (each checked against code or files before recording)

- **483/484/369/481 are affected, not "unaffected because OPT-300 is ORG-only".** `notice_parties_from` takes every role target,
  and `buyer_fix` / `buyer_side_mentions` read the same mention list touchpoint minting would extend (role_census.rs:604-641,
  682-705, 945-985; project.rs:5772-5774, 10377-10395). This became D576-1 (v).
- **"96% are tribunals" was "96% carry a different name".** R3's top 60 already holds 75 non-tribunal review refs, so tribunals
  are ≤ 89%. Alternative (a)'s cost is restated as ≥467 / ≤935. The lead exhibit's compound name mints its own row; it does not
  bind 3955.
- **The (i) rows are anchored at the parent ORG, not at the TPO.** `guid_twin_parties_sql` therefore sees them on the buyer's
  key; this is a delta to expect, not rows to ignore.
- **The drop rules do not catch touchpoint prose or "ORG-0001".** The lexicons have no stem for them, which is why (iv) was added.
- **551's state was stale.** It is no longer a held patch: the flip is on main and deployed, U7/U8 have passed, and the switch-off
  twin d569107 is armed. The flip touches 9 files, not 7 (admin.rs and head_election_agreement.rs as well).
- **Union of affected notices.** It is 345-374 (k1), not "about 400", and the corpus estimate is ≈160-175k.
- **A7 line numbers and scope.** `guid_twin_role_ref` is at 233-243 and `guid_twin_section_roles` at 252-260; the latter reads
  TPO targets too.
- **Review-family arithmetic.** 1,010 includes Part-ReviewOrg's 2 (Lot only: 1,008 / 40).
- **Unit 3 uses the existing `refold-sections` job,** which re-mints mentions through the incremental fold. Its missing dry run
  needs a COUNT first.
- **Units, not decision changes.** The same-name share is labelled a lower bound. The tribunal share is labelled unsalted. A
  BT-514 fallback and BT-16 handling were added.
- **Dropped as unconfirmed:** skeptic 1's note that DE-1.x touchpoints are a differently spelled section kind outside the cursor.
  k2's 0 rows in the window stand.

**551 W-on drain protocol (2026-10-23, `551-lot-sum/b6/decision-w-on-drains.md`).** Unit 2 deploys in one batch with 578 U2
and 579 U4, and 577 U2 joins only if 551's W7 check is empty. Unit 2 is the batch's only fold-changing commit. The batch runs
after 539's drain and the 10-28 day-7 fetch, and needs ≥ 15 GB free before each gate. It ships only if the gate, the deploy,
the twin re-gated last and P1 all fit before the gap's latest switch-off start. Unit 3 is class N, mid-scale. It runs after
P1 passes and the day-7 read is recorded PASS (about 10-29), and before 546 U4 and 577 U3. It never runs between a chain and
its U9 check, and the job and its checks end ≥ 2 h before the next 09:35 Berlin tick (08:35Z from 10-25). Before it: the
bounded Touchpoint COUNT, expected NEW = MOVED = LOST = 0, salt `551-b6-drain-576-`, a writer-lock line. The check is gd5
class N. Any NEW is a STOP and every one is read. More than 30 means the class premise is broken: read a salted 30 under U5's
bar and stop the programme. MOVED or SILENT is a STOP, and switch off if it is unexplained at the end of the gap. 546 place
moves on mixed legacy/eForms chains are NOTEs. Cost: about 25–40 statements, 1.5–2.5 h. **Next step:** unit 2's deploy waits
for the batch slot, and unit 3 also waits for P1 and the day-7 PASS.
