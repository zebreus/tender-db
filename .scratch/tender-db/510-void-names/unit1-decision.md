## Unit 1: decision (2026-10-10)

Suggested status line: `Status: ready-for-agent — unit 1 DECIDED 2026-10-10 (below). NEXT: unit 2, build (predicate module, fold application, store retire, cohort job); then unit 3, drain.`

**In one paragraph.** The rule is a new pure predicate, `ingest::partyname::not_a_name`. It sorts a non-name into one of two classes. `VoidLot` is a statement that the lot was not awarded. `Placeholder` is a pointer, a withheld name, or a summary of the award. The fold applies only `VoidLot`, and it does so in every era and every role, whether or not the mention has an identifier. Such a mention mints no organization and carries no role, no result winner and no bid party. A legacy or sdk-0.1 result that is left with no real winner reads `clos-nw`, the same way `TED-NO_AWARDED_CONTRACT` already does. A refold retires the recorded mention rows inside the resolver's transaction, so the 443 sweep can remove the orgs. The text parser keeps its early drop but takes its lists from the same module, rejecting both classes. The fold does not apply `Placeholder` in this issue (§9).

### 1. Where the rule lives

| piece | site | what it does |
|---|---|---|
| **The predicate (one definition)** | new `crates/ingest/src/partyname.rs`, registered in `crates/ingest/src/lib.rs` next to `idgate` / `orgid` (lines 14–16) | `pub fn not_a_name(name: &str) -> Option<NotAName>` with `enum NotAName { VoidLot, Placeholder }`. Pure, no I/O, no store. Same contract as `orgid.rs:6-8`: parsers store the published value and the projection decides. `NAME_REJECTS` / `NAME_WHOLE_REJECTS` (`text/parse.rs:150-170`) move here. |
| **The fold's one question** | new `fn void_party_sections(sdk01, sections, alias, parsed) -> BTreeSet<String>` beside `party_names` (`ingest/src/project.rs:4305`) | Returns each outermost party section where **every** non-empty name value is `VoidLot`, plus the inner halves of those sections. It uses `party_name_field` (:4293), `enclosing` (:5712) and `nested_org_aliases` (:5684), the same walk `NoticeState::mentions` uses. Its callers must never decide "void" any other way. |
| consumer 1 | `NoticeState::mentions` (:4759) | Skips the void outer sections. A sibling `mentions_and_void(..) -> (Vec<Mention>, Vec<String>)` also returns the void section ids. `mentions()` stays as `.0`, so the guards (:7348) and the role census (`role_census.rs:609`, `:855`) are filtered with no change on their side. |
| consumer 2 | `NoticeState::read` (:4339) | After `org_alias` (:4674): `raw_roles.retain` drops any target whose outer section is void. `read_results` (:4685) receives the void set (see §3). This works even when the recorded mention row still exists, because Phase 2 binds through `mentions_by_ids` (`store/src/canonical.rs:9830`). |
| consumer 3 | the plan sweeps, full (`project.rs:2102`) and incremental (`project.rs:3104`) | Collect the void `(notice_id, section_id)` keys of each chunk and call the store retire (below) next to `resolve_mentions` (:2156 / :3345). The void mentions never reach the resolver, so `resolved_orgs` (:7951) stays aligned with the buyer tokens. |
| **Store retire (no language in the store)** | new `Db::retire_mentions(&mut MentionResolver, &[(i64, String)])` in `store/src/canonical.rs` beside `resolve_mentions` (:15297) | Per key, inside `Db::immediate` (the issue 498/501 rule): seek the mention by its full primary key. If it exists: `DELETE` from `tender_version_parties`, then from `tender_version_bid_parties`, by `(mention_notice_id, mention_section_id)` (the `_mention_key` indexes, :11607-11608). Then delete the mention row by its full primary key. Then stamp stale **the `tender_id`s of the rows just deleted, plus** the notice's own `caused_by` Tenders (`stamp_tenders_of_notices_stale`, :9438), in the same transaction. Adds a `resolver.mentions_retired` counter, reported next to the 434 tallies. A key with no recorded row costs one seek and writes nothing. |
| auto sweep | `sweep_after_fold` (`app/src/supervisor.rs:2836`, call at :4478) | Fires on `mentions_rebound + mentions_retired > 0`. A retire empties orgs the same way a re-bind does. |

**Rejected sites.**
- `mentions()` alone: the recorded rows survive, Phase 2 keeps binding them, and the sweep keeps every org.
- `bind_organizations` (:4879) alone: it runs after `read_legacy_results` has already decided `selec-w` from the void winner (:5987, :6036-6038).
- The r209 walker or a per-parser rule: needs a reparse of 2011–2021 and leaves eForms, FTS and sdk-0.1 uncovered. Blanking only the name mints one nameless org per mention (canonical.rs:16259).
- The store resolver: it must return one id per mention, and `organization_id` is NOT NULL (:447).

### 2. The predicate

```text
fold(s)     = store::buyer_name_fold(&project::match_norm(s))    // == role_census::fold (role_census.rs:362)
              case folded, Latin accents folded, every non-alphanumeric run -> one space
stripped(s) = s.trim() with ONE trailing "( lot|lots|lote|lotes|lotto|lotti … )" group removed,
              plus any punctuation after it; hand-rolled with rfind('('), since ingest has no regex
not_a_name(s):
  f = fold(s); empty -> None
  if " "+f contains " "+stem for any VOID_STEM:      // left word boundary, may end mid-word
       if award_clause(f) -> Placeholder              // "…infructueux, puis attribué à la Société X"
       else               -> VoidLot
  if " "+f contains " "+stem for any PLACEHOLDER_STEM -> Placeholder
  if fold(stripped(s)) ∈ VOID_WHOLE        -> VoidLot  // whole compare only after the lot strip
  if fold(stripped(s)) ∈ PLACEHOLDER_WHOLE -> Placeholder
  None
award_clause(f) = a word in {attribue, attribuee, attribues, attribuees, attribuer} not right after
                  {non, pas, sans}; or " avec la societe " / " avec l entreprise " / " avec les societes "
```

**Why these choices.**
- **Left word boundary instead of raw substring.** The raw rule refuses real-looking names such as `Gestiver Información SL` and `Server Información…` (`VER INFORMACI`), `Les Artisans Suite…` (`SANS SUITE`), `Tennessee Section` and `Cannon Attributes`. The bounded rule does not. Over both corpora below, there are 0 names the raw text rule refuses that this rule lets through.
- **Stems search the full fold, not the stripped one.** The prototype stripped first and turned `Sans objet (lot déclaré sans suite)` into a name. That was the only regression found, and it is now pinned.
- **The award clause.** It moves 32 census orgs (32 mentions, 16 notices) and 6 window names to `Placeholder`. All of them say the failed lot was relaunched and then awarded to a named company, for example `Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Sandoz…`, `…marché attribué à: Berthelet`, `…attribuée à LD Bio Diagnostics`. Reading those as `clos-nw` would contradict the publisher. As `Placeholder`, the fold leaves them as they are today and the text parser still refuses them.

**VoidLot stems.** All are from the 508 list except the two marked new. Columns: census orgs / mentions / winner-role rows, then hits in the 285,757-name window scan.

| stem | orgs | mentions | winner | window | source |
|---|---:|---:|---:|---:|---|
| `infructu` | 4,780 | 11,342 | 11,342 | 321 | 508; hub 1169469 |
| `infructeu` (new) | 10 | 44 | 44 | 4 | the `infructeux` misspelling evades `INFRUCTU` (1238693: 26) |
| `sans suite` | 2,992 | 7,278 | 7,277 | 274 | 508; hub 1170715 |
| `non attribu` | 486 | 918 | 882 | 41 | 508; the fold also catches `Non-attribué` / `non-attribution` (37 orgs), which the raw rule missed |
| `declarado desiert` | 202 | 215 | 215 | 0 | 508 |
| `declarada desiert` (new) | 2 | 4 | 4 | 0 | feminine form of the 508 stem |
| `queda desiert` | 1 | 1 | 1 | 1 | 508 |
| `nessuna aggiudicazione` | 2 | 2 | 2 | 0 | 508 |

**VoidLot whole values.** These are compared after the lot strip. Every new entry is whole-value only. An entry is admitted when it says the lot was not awarded, has at least 20 winner-role mentions, and has no other-role use and no real company with that exact fold in the census.

| whole | orgs | mentions | winner | window | | whole | orgs | mentions | winner | window |
|---|---:|---:|---:|---:|---|---|---:|---:|---:|---:|
| `desierto` (508) | 8 | 1,229 | 1,229 | 20 | | `no contract awarded` | 28 | 28 | 28 | 0 |
| `desierta` (508) | 30 | 30 | 30 | 0 | | `niet gegund` | 50 | 310 | 310 | 9 |
| `deserto` (508) | 436 | 449 | 449 | 18 | | `uniewazniony` | 255 | 256 | 256 | 0 |
| `deserta` (508) | 24 | 24 | 24 | 0 | | `uniewazniono` | 105 | 106 | 105 | 0 |
| `desiertos` | 2 | 8 | 8 | 0 | | `postepowanie uniewaznione` | 2 | 25 | 25 | 1 |
| `lote desierto` | 101 | 104 | 104 | 0 | | `brak ofert` | 151 | 162 | 162 | 1 |
| `lotto deserto` | 433 | 453 | 453 | 1 | | `aucune offre` | 2 | 79 | 79 | 0 |
| `gara deserta` | 106 | 107 | 107 | 0 | | `aucune offre recue` | 3 | 36 | 36 | 0 |
| `non aggiudicato` | 356 | 369 | 369 | 3 | | `pas d attributaire` | 2 | 45 | 45 | 0 |
| `non aggiudicata` | 2 | 21 | 21 | 0 | | `pas d offre` | 3 | 31 | 31 | 1 |
| `lotto non aggiudicato` | 191 | 212 | 212 | 3 | | `sans offre` | 2 | 30 | 30 | 0 |
| `not awarded` | 180 | 270 | 269 | 6 | | `abandon` | 2 | 114 | 114 | 0 |
| `lot not awarded` | 57 | 60 | 60 | 0 | | `aufgehoben` | 36 | 52 | 52 | 2 |
| `no award` | 68 | 70 | 70 | 0 | | `nicht vergeben` | 4 | 20 | 20 | 0 |
| `no award made` | 31 | 37 | 37 | 0 | | `contract not awarded` | 21 | 24 | 23 | 0 |

`desierto` covers `Desierto.`, `desierto (lote 3)`, `Desierto (lotes VIII y IX)` and `Desierto (lot 9/10)`: the issue's gap 2, 4 orgs, now caught. `Desierto OÜ`, `DESIERTO, S.L.` and `Desierto Florido SL` stay names.

**Placeholder** (the text parser refuses it; the fold ignores it until the follow-up):
- the 508 stems `would prejudice`, `not applicable`, `see section`, `perfil del contratante`, `perfil de contratante`, `voir autres informations`, `voir renseignements`, `ver informaci`;
- the whole value `various`;
- the 32 award-clause names.

Census: 2,952 orgs and 5,672 mentions. Their party rows are mostly **not** winners: review-body 1,782, mediation-body 1,252, winner 1,214, appeal-information 403, Lot-ReviewOrg 293, RESPONSIBLE_FOR_MEDIATION_PROCEDURES 194, purchasing-body 172, specifications-provider 160, further-information 46, Tenderer 46, tender-receipt 35, **buyer 20**. Dropping a buyer-slot mention would change buyer tokens and grouping, so it needs its own decision.

**Deferred and not in either list.** These are placeholders that mostly sit in review or mediation slots, or are ambiguous: `neant` (mediation 582), `ingen` (mediation 297; `InGen` is a real name), `n a`, `none`, `nie dotyczy`, `entfällt`, `keine Angabe`, `brak`, `sans objet` (mediation 106, 2 with an identifier), `aucun`, `nessuno`, `varios`, `diverse`, `various suppliers`, `multiple …`, `withheld …`. Also void wordings under 20 mentions: `ΑΓΟΝΟΣ` 9, `uniewaznienie` 17, `cancelled` 5, `zrušeno` 14, `annulleret` 14. And `Desierto por …` prose (about 100 mentions), which only a leading-word rule could catch, and that rule would also kill `Desierto OÜ`.

**No identifier guard.** The void class has exactly 2 orgs with an identifier: 23059681 `Sans suite` (FR 1451529201) and 23387611 `DESIERTO` (ES A11001100). Both are eForms Tenderers with 1 mention each, and both are junk. If a void mention carrying a real company's id were kept, it would hand that company a void lot. The real companies an identifier guard was meant to protect (Desierto OÜ, N/A s.r.o., Void arhitektura, DESERTOT) are already protected by whole-value matching.

### 3. What happens to the result and the role

Applied in `NoticeState::read` before anything binds, keyed on the void set (outer and inner sections):

- **Roles.** Every `raw_roles` entry that targets a void section is dropped, in every role. Measured cost: 3 non-winner rows out of 24.5k, and all 3 are junk (`Not Awarded` purchasing-body, `Contract not awarded` review-body, `Unieważniono` Lot-DocProvider). This also covers the unfolded internal-ojs `AWARD_AND_CONTRACT_VALUE`, which a filter on `winner` would miss.
- **Legacy results** (`read_legacy_results`, :5951). A void ref is not pushed to `direct_winners` (:5987); it sets `r.void_lot = true`. The derivation (:6031-6041) becomes, when `decision` is unset:
  1. any remaining real direct winner → `selec-w`;
  2. else `void_lot` → **`clos-nw`**;
  3. else the existing rule: a value → `selec-w`, no date → `clos-nw`, date only → NULL.

  The publisher's `TED-NO_AWARDED_CONTRACT` (:6015) still wins, because it sets `decision` first. This is the marker semantics: a void phrase is an explicit no-award statement, so it outranks a value or a date. Today a void ref manufactures `selec-w`. Re-deriving without the marker would leave a lie:

  | org | result-winner rows (all `selec-w` today) | neither value nor date | date only | value (of which exactly 0) |
  |---|---:|---:|---:|---:|
  | 1169469 infructueux | 3,570 | 3,023 | 456 | 91 (61) |
  | 1170715 Sans suite | 2,265 | 1,943 | 291 | 31 |
  | 1177920 Lot infructueux | 1,135 | 951 | 84 | 100 (91) |
  | 1185001 desierto | 1,158 | 349 | 670 | 139 (114) |

  The plain evidence rule would put 1,501 of these rows at NULL ("an award was announced and its outcome withheld"). It would also keep 361 at `selec-w` with no winner, and 266 of the 330 value rows on the three hubs where the zero count was read are exactly 0. The marker makes all of them `clos-nw`. The value stays on the row, as it does today under `NO_AWARDED_CONTRACT`. `reason` (BT-144) stays NULL; it is not guessed.
- **sdk-0.1** (`read_sdk01_results`, :6062). A void `WinningParty` is skipped (:6095). No real winner, `void_lot` set and no code → `clos-nw`. A published `TenderResultCode` still wins.
- **eForms and FTS.** A void section is skipped when the `TenderingParty` members are pushed (:5913). BT-142 is the publisher's code and stays: 1247907's eForms rows keep `selec-w` with no winner. The Bid and its value stay; the bid party goes.
- **Bound output.** No `winner` / `Tenderer` party row, no `tender_version_result_winners` row, no bid-party row. `buyer_winner_sections` (:4246) never sees the void section.
- **Text era.** The parser drops the slot, so a reparsed text notice has **no** lot-result row. A standing text notice refolded by this drain keeps its parse-time `RES-n` and so reads `clos-nw`. This asymmetry is inherent to 508/509's choice. It is accepted and stated here.
- **Other consumers.**
  - Role census 483/484 (`notice_parties`, `buyer_fix`): void winners vanish. Only winner-side denominators move. A void name never equals a buyer, so no buyer class moves.
  - Guards (:7320) are filtered automatically.
  - Issue 456 `mention_name`: there is no party row left to read.
  - `NON_NAME_FOLDS` (`role_census.rs:539`) is unchanged; unifying it with `Placeholder` is follow-up work.

### 4. The text parser's own lists: keep the early drop, give up the lists

`plausible_name` (`text/parse.rs:183-195`) keeps its letter check and its contact-line check (:197+). It replaces both list checks with `crate::partyname::not_a_name(name).is_none()`, which refuses both classes. The drop stays at the parse layer because it is the cheapest point: no `RES-n`, no `ORG-n`, nothing to retire. The lists live in one place.

- **Lost refusals:** 0 on the 27,821 census names and 0 on the 285,757 window names.
- **New refusals** (they apply only when a text notice is reparsed): `Desierto.`, `Desierto (lotes …)`, `Non-attribué`, `Lot déclaré infructeux`, and the new whole values.
- **Newly accepted:** the raw-substring traps (`Gestiver Información SL`).

No text reparse is part of this drain. The fold covers the standing text mentions (690 cohort notices, including the 99 text mentions on 1185001 and pre-2004 text). Gap 3 is therefore closed by the fold, not by a reparse.

### 5. False-positive guard and evidence

1. **Prefix census** (unit-1 read). 27,821 distinct orgs from about 280 phrase prefixes, across accents, case and 12 languages.
   - Void class: 11,166 orgs, 24,565 mentions, 24,525 winner-role rows, 10,239 country-less.
   - Real names next to the phrases, all classified `None`: Desierto OÜ (EE 14836245), DESERTOT, VÁRIOS MUNDOS UNIP. LDA., Diverse Care Services, Diversey, NIL d.o.o., N/A s.r.o., SEE LAUER, See & Go, CF Cefarm, Brake, Vedise, Void arhitektura, VOID SISTEMAS, Unknown Architects, Confidential Waste Services, Multiplex, Nonet, Vacant Vårdbemanning, InGen.
2. **XML-era window scan.** This is the issue's ask, done on stored values rather than on orgs. 81 bounded windows of 1,000 notice ids over `notice_texts` (`TED-OFFICIALNAME`, BT-500 for eForms): 285,757 names on 58,947 notices (r208 209,394; r209 47,285; eForms 26,076; text 3,002), of which 109,346 are distinct.
   - 706 `VoidLot` hits. Every hit was read.
   - **One real party is affected:** `LV Consultants Cv1 Simarouba - 97310 Kourou, FRANCE/ La consultation a été déclarée sans suite pour la part du marché non couverte`. It is a prose-named junk org today, and `clos-nw` is half true for it. This is accepted residual risk, 1 in 285,757.
   - The 6 "then awarded to X" names are the award clause above.
3. **Commercial-form scan.** Void stem hits that also contain a `COMMERCIAL_FORMS` word (`role_census.rs:267`): 3 names, all French prose `sa` ("her"): 31488262, 2698776, 5605787. A legal-form guard would leak exactly those, so no guard is added. The cohort job's dry plan lists them instead.
4. **Org-scan soundness.** The same 81 windows were read on `organization_mentions`: 949 void-named mentions on 467 orgs. **0** of them sit on an org whose head name is not void. 113 of the 467 orgs (143 mentions) are absent from the prefix census, so **the census undercounts by about 15% of mentions and about 24% of orgs**. The drain therefore enumerates by a full org scan, not by the census list.

### 6. Tests (unit 2)

- **`partyname.rs`:**
  - `every_measured_void_lot_spelling_is_refused`: one prod specimen per entry, plus `INFRUCTUEUX`, `infructueux — relance en marche negocie…`, `Sans objet (lot déclaré sans suite)`, `Desierto.`, `desierto (lote 3)`, `Desiertos (lotes 3 y 5)`, `Non-attribué`, `Lot déclaré infructeux`.
  - `a_void_phrase_that_goes_on_to_name_an_award_is_a_placeholder` (the Sandoz / Berthelet / LD Bio shapes) and `non attribué à ce jour` stays `VoidLot`.
  - `placeholders_are_their_own_class`.
  - `real_names_beside_the_phrases_are_names`: every name in §5.1, plus `Gestiver Información SL`, `Les Artisans Suite`, `DESIERTO, S.L.`, `Abandon Records Ltd`, and the 508 keep list.
  - `the_lot_qualifier_is_stripped_only_for_the_whole_compare` (`ACME (lot 3)` → None).
  - `the_fold_is_the_role_census_fold`.
- **`text/parse.rs`:** extend `a_void_lot_or_a_pointer_in_the_winner_slot_names_nobody` (:4457) with the rows above. `Gestiver Información SL` joins the accepted names.
- **`ingest/tests/project.rs`:**
  - `a_void_lot_winner_mints_no_organization_and_closes_its_result` (RED today: org minted, `selec-w`).
  - `a_void_lot_with_a_zero_value_still_closes`.
  - `a_real_co_winner_keeps_the_result_selected`.
  - `the_no_award_marker_and_a_void_winner_agree`.
  - `a_void_winner_named_through_its_nested_half_is_dropped` (a ref to the inner `ADDRESS_CONTRACTOR`).
  - `a_party_with_one_real_name_is_not_void`.
  - `an_eforms_void_tenderer_keeps_the_publishers_decision` (`selec-w`, no winner, no bid party, Bid kept).
  - `a_standing_void_mention_is_retired_by_a_refold`, run as `Refold::{Incremental, Full}` on the 434 harness (`a_refreshed_mention_moves_its_party`, :5663). Fold `ACME SARL`, reparse the same section to `Infructueux`, refold. Expect `mentions_retired == 1`; mention, party, bid-party and result-winner rows all 0; `clos-nw`; epoch current. A second refold retires 0, and the ACME org ends up mention-less.
- **`project.rs` units:**
  - `a_stale_mention_row_binds_no_void_winner`: `bind_organizations` with a `by_section` that still maps the void section.
  - `buyer_tokens_stay_aligned_when_a_void_mention_is_skipped`: `resolved_orgs` over [buyer, void, winner].
- **`store` (canonical.rs):** `retiring_a_mention_takes_its_party_rows_and_stamps_their_tenders`. Covers FK order, a party row in a version caused by another notice getting its Tender stamped, idempotence, an absent key being a no-op, and running inside `Db::immediate`.
- **`app` (supervisor.rs):**
  - extend the `sweep_after_fold` test (:18180) so a retire queues the sweep;
  - extend `the_orphan_sweep_counts_plans_and_sweeps_real_orphans` (:18090) with an org a retire emptied;
  - `refold-void-names`: `dry_counts_and_writes_nothing`, `refuses_over_its_expect`, `requeues_and_stamps`.
  - The new `Spec` arm body goes in `Box::pin(async move {..}).await` (CLAUDE.md stack note).
  - Gate through `ops/check.sh` only. `cargo check -p tender-db` does not compile supervisor.rs.

### 7. Drain (unit 3)

1. **Deploy.** From then on the daily fold drops new void mentions; eForms carries about 46 notices' worth in total. A full-fallback fold, if one ever runs, retires corpus-wide. That is correct, and its report shows a large `mentions_retired`.
2. **`refold-void-names dry=true`**, a new job. It walks `organizations` by id window the way the 443 sweep does (job 1628: 348 s over 7.37M orgs). It applies `partyname::not_a_name(name) == Some(VoidLot)`, the **same function as the fold**, never SQL `LIKE`. It then seeks `organization_mentions_org` for the notices. The plan records orgs, mentions, notices by profile, the identifier-bearing orgs and the legal-form-word orgs, and the top 20 orgs by mentions.

   **Expected:** orgs 11,166 to about 15k; mentions 24,565 to about 29k; notices 9,511 to about 11.5k. The census cohort is 9,511 notices. Before the 32 award-clause orgs were excepted it was 9,522: r208 8,501, text 690, internal-ojs 146, r209 138, eForms 46, FTS 1. Identifier-bearing: 2. Legal-form `sa` prose: 3. Top of the list: 1169469, 1170715, 1185001, 1177920.

   **Stop if** any count falls below the census floor, which means the predicate or the scan regressed. Read samples first if orgs exceed about 22k. Stop and add an exemption if an identifier-bearing or legal-form org reads as a company.
3. **`refold-void-names expect=<dry notices>`**, wet. It re-enumerates and refuses above `expect` (the `refold-fields` gate, supervisor.rs:2142). Then `unmark_projected_by_ids` + `stamp_stale_for_notices` in chunks, then `project rebuild=false`. It replaces about 11 hand-run rounds of `refold-notices` (cap 1,000, :229). The census list alone would leave the 15–25% the census never saw.

   **Expected fold:** `mentions_retired` ≈ the dry mention count (the org/mention name agreement was measured at 949 of 949); `mentions_refreshed` about 0; 0 new orgs from these notices; Tenders stamped ≤ notices. The closure is far under `LEGACY_CLOSURE_CAP` (500k, project.rs:2788). About 15–30 min (issue 471: 324 legacy notices in 22 s; issue 379: about 1 min per 1,000). The refold also picks up every projection change since these notices were last folded. That is expected, and the scope is bounded.
4. **`sweep-orphan-orgs`**, dry → read → wet. The fold queues it on retires. `counted` should be about the dry org count, which is over `AUTO_SWEEP_CAP` (10,000, :2820), so the plan is recorded and nothing is swept automatically (509's 2150/2151 pattern). Read `protected` (509 kept 9 by review tables, which `/v1/sql` cannot read), then run wet.
5. **Verify** (beyond the issue's line):
   - `name_prefix=Infructueux` → `0`. Today it lists 1,879 orgs, not one page.
   - `name_prefix=` `Sans%20suite`, `Lotto%20deserto`, `Niet%20gegund` → `0`.
   - `SELECT COUNT(*) FROM tender_version_result_winners WHERE organization_id IN (1169469,1170715,1177920,1185001,1247907)` → `0`.
   - If `Infructueux` stays above 0, read the residual ids against the sweep's `protected` count before calling it a miss.

### 8. What could go wrong, and how the build pins it

| risk | pin |
|---|---|
| A real company is dropped | Whole-value only for new entries. Stems are the 508 stems plus two spellings, matched left-word-bounded. Pinned by the real-name test (§5.1). The dry plan lists identifier-bearing and legal-form orgs with a stop rule. Window evidence: 1 residual in 285,757. |
| A stem inside a company's name ("ACME SA (lot 2 infructueux)") | Unmeasurable beyond the window scan, which found 0 such names, and the text era's 300k scan, which also found none. Accepted. The daily fold has no dry check; the cohort job does. |
| "Infructueux, then awarded to X" read as no award | The award clause → `Placeholder`; tested. |
| Phase 2 binds through a stale recorded row | `read()` drops the void refs without consulting `by_section`; `a_stale_mention_row_binds_no_void_winner`. |
| The void decision is manufactured from the winner (today's bug) or from a 0 value | Marker semantics before the evidence rule; the zero-value and co-winner tests. |
| Buyer tokens misaligned | Void mentions are removed before `resolve_mentions`; alignment test. |
| FK violation, or a Tender left naming the junk org | The retire order and the stamp of the deleted rows' Tenders happen in one transaction; store test. The `Full` and `Incremental` refold tests. |
| Orgs never swept | `mentions_retired` feeds `sweep_after_fold`; supervisor test. The manual wet is planned anyway. |
| `mentions()` and `read()` disagree on "void" | Both call `void_party_sections`, one function, alias-aware; nested-half test. |
| Text parser regresses | 0 lost refusals on both corpora; the extended 508 test. |
| The drain misses mid-string names | Full org scan with the fold's own predicate. Org-name proxy measured sound (0 of 949). Census floor as a stop rule. |
| The new `Spec` arm overflows the `run_spec` stack | `Box::pin` the arm (CLAUDE.md); the existing stack-sensitive test stays green under `ops/check.sh`. |

### 9. Follow-up issue to file with unit 2

"Placeholder names (pointers, withheld, `Various`, award summaries) mint parties in every role." It carries the role split in §2, including the **20 buyer rows**, the deferred list above, and unifying `NON_NAME_FOLDS` with `Placeholder`. Its semantics differ from this issue: a winner was chosen, its name is elsewhere or withheld. So `selec-w` stays, nothing is bound as the winner, and buyer-slot handling has to be decided.

Working files, kept in this directory: the predicate prototype `final.py` (with `pred.py`), the cohort `cohort_ids_all.json`, the void org ids `void_org_ids.json`, the excepted award-clause notices `excepted_notices.json`, and the void hits of the mention-window scan `mwin_void.json`. The raw 17 MB name-window scan (`windows.jsonl`) was not kept; it is re-derivable from the 81 windows the scan names.