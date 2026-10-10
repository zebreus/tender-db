## Unit 2: decision (2026-10-10)

An adversarial design panel decided this unit. Three proposals were each scored by two judges. The minimal-blast-radius proposal won with 50 of 60 points, and no judge found a fatal flaw in it. The judges' amendments are folded in below.

Four ideas come from the runners-up:
- the `kind == "Lot"` drop rule;
- the corroboration guard placed inside `add_head_amount`;
- the alias target `UBL-FrameworkMaximumAmount`;
- the cohort read before the wet run.

Their fatal flaws are recorded under "Rejected".

### The decision

- **BT-709-LotResult and BT-660-LotResult:**
  - Stored at the LotResult's own Lot, found through BT-13713.
  - Two new field names: `result_framework_maximum` and `result_framework_reestimate`.
  - They are **scale partners only**:
    - every scale rule sees them;
    - neither the head election nor the lot-value election ever elects them;
    - they never corroborate a figure;
    - they never count in 492's lot sums or 505's residual tables.
- **BT-118-NoticeResult and BT-1118-NoticeResult** stay unmapped (in `notice_amounts` only).
- **DE1 aliases:**
  - The two DE1 LotResult spellings are aliased onto BT-709 and BT-660.
  - The two DE1 `Overall*` totals are not aliased.
  - The bare `DE1-FrameworkMaximumAmount` is aliased to `UBL-FrameworkMaximumAmount`, which folds to `framework_maximum` at Tender scope. It goes in its own commit with its own drain pass.
- **The two slips** fall to 492's existing ×100 rule, with no rule change:
  - 395737 falls to BGN 7,348,200 (about €3,757,075).
  - 627219 falls to €6,666,666.67.
  - Refused is not corrected (471/492 precedent). The adjudicated ceilings (BGN 29.99 m, €33.23 m) are not served.
- **The triage question** (the ~17 % of carrying notices with no mapped figure):
  - A BT-709/BT-660 figure there elects nothing.
  - The figure is served in `amounts` at its lot.
  - The Tender keeps whatever its chain had: often the CN's carried BT-27/271, otherwise no value.

### Per field

Census denominators: 26,182 EU notices (SDK 1.3, 1.6–1.14) and 11,875 DE notices. The sample is lumpy:
- sdk-1.6 holds 11,975 of the 15,323 BT-709 rows and 13,013 of the 14,981 BT-660 rows, so the rate outside sdk-1.6 is given beside each total.
- Every rate is per row and notice-local.
- "pow10" counts k = 2..6 in either direction at any magnitude.
- eforms-sdk-1.15 and eForms-DE 2.x were not sampled.

| field | destination | scope | role |
|---|---|---|---|
| BT-709-LotResult | new `result_framework_maximum` | the result's Lot via BT-13713; dropped if unresolved | partner only |
| BT-660-LotResult | new `result_framework_reestimate` | same | partner only |
| BT-118-NoticeResult | none (stays in `notice_amounts`) | (would be Tender) | unmapped |
| BT-1118-NoticeResult | none | (would be Tender) | unmapped |
| DE1-NoticeResult-LotResult-FrameworkAgreementValues-MaximumValueAmount | alias → `BT-709-LotResult` | as BT-709 | partner only |
| DE1-NoticeResult-LotResult-FrameworkAgreementValues-ReestimatedValueAmount | alias → `BT-660-LotResult` | as BT-660 | partner only |
| DE1-NoticeResult-OverallMaximumFrameworkContractsAmount / -OverallApproximateFrameworkContractsAmount | no alias | – | unmapped |
| DE1-FrameworkMaximumAmount (bare, root) | alias → `UBL-FrameworkMaximumAmount` → `framework_maximum` | Tender (root section) | head candidate + Procedure partner (never a lot value) |
| BT-710 / BT-711-LotResult | none (unchanged) | – | unmapped |
| BT-156 / BT-1561-NoticeResult, BT-157-LotsGroup | none (deferred) | – | unmapped |

**BT-709-LotResult**
- **Scope.**
  - `RawResults::lot_of` (project.rs:6106-6109) resolves it, as the award roles are re-scoped at project.rs:4663-4675.
  - It is filed only when that key is a section of kind `Lot` in the same notice. Otherwise it is dropped.
  - There is deliberately no `Scope::Tender` fallback (contrast project.rs:4669). A lot-null result figure is a Procedure partner by 492 decision (c) (canonical.rs:3597-3606). It would flip `lot_only` (3556), null `lot_keys` (3563) and void 492's two exemptions.
  - Only kind `Lot` is accepted because `add_version_figures`' `sibling()` (3594-3596) turns a LotsGroup or Part key into a no-key lot figure.
- **Why a partner.** Both slips need one, and 395737's only exact ×100 partner is this field.
- **Why not a candidate.** As a candidate it would mostly elect the award:
  - 14,109 / 15,323 rows (92.1 %; 64.8 % outside sdk-1.6) equal a BT-720.
  - The award is deliberately not a head candidate (canonical.rs:3176-3193; docs.rs:515-520 "never from award results"). 492's drain left 7 Tenders without a head for this reason.
  - Heads and lot values would appear on 2,671 rows with no mapped figure (17.4 %; 24.0 % outside).
  - Heads would rise on 234 rows (1.5 %; 4.2 % outside).
  - Most framework lot values would change: it equals its own lot's BT-271/27 on only 3.2 % (13.5 % outside).
  - None of this was measured at head-version level.
- **Why a new name.** Supersession is per lot per `("amount", field)` (canonical.rs:3053; project.rs:5530-5536, 5604-5608).
  - Under `framework_maximum`, a CAN that does not restate BT-271-Lot would delete the CN's ceiling.
  - That is structural on VEAT (25-27) and modification (38-39) notices: the SDK forbids BT-271-Lot there and allows BT-709.
- **New pow10 pairs.** 11 rows (0.07 %; 10/3,348 = 0.3 % outside sdk-1.6). Every sampled one is far below the €1 bn gate.

**BT-660-LotResult**
- Same scope and role as BT-709.
- **Why a partner.** 627219 needs it: 3,322,770,833 cents = slip / 100.
- **Why not a candidate.**
  - 11,254 / 14,981 rows (75.1 %; 49.5 % outside) equal nothing mapped, so it would replace lot values wherever it is the largest figure.
  - It exceeds the head proxy on 138 rows (0.9 %; 2.0 % outside). Lot-level exceedance was never measured.
  - Whether the re-estimate or the estimate is "the" value is a modelling question this issue does not decide.
- **Why not `estimated_value`.**
  - It would supersede the CN's BT-27-Lot on every CAN that does not restate it.
  - It equals its own lot's BT-271/27 on 845 rows (35 % outside sdk-1.6). In another field that is a cross-field corroboration, which would shield a k ≥ 3 BT-271-Lot slip from 471.
- **New pow10 pairs.** 16 rows (0.11 %; 15/1,968 = 0.8 % outside sdk-1.6), all far below the gate in the sample.

**BT-118-NoticeResult: left.**
- Neither slip needs it, and it has 0 pow10 pairs in 879 rows.
- **Partner hazard.** A lot-null notice total registers as a Procedure partner unless it equals an award (canonical.rs:3601-3607).
  - That clears `lot_only` and nulls `lot_keys` for every value it shares with a lot figure.
  - That disables 492's framework-total exemption (3783-3787) and sibling-lot exemption (3799-3808).
  - It equals a lot figure on 139 / 879 rows (15.8 %). The typical case is a CAN awarding one lot of a multi-lot framework, which is the shape of all nine genuine ×100 heads in 492.
- **Candidate hazard.** As a head candidate it raises the head on 97 / 879 rows (11.0 %). Example: 23670345, €12.68 m over €0.60 m. That is an unmeasured head shift.
- **Reopen** only when a slip needs it. It then needs a registration that files a notice total equal to a lot figure as that lot's figure, which extends decision (c).

**BT-1118-NoticeResult: left.**
- For 627219 it is redundant: BT-660 carries the same cents at the lot.
- It has 1 pow10 pair in 406 rows: SEK 1,000,000 against a 1.00 placeholder, which is below the 10.00 partner floor.
- It equals a lot figure on 108 / 406 rows (26.6 %), the same hazard as BT-118.
- It exceeds the head proxy on 37 rows (9.1 %).

**DE1 LotResult spellings**
- They are the same facts: MaximumValueAmount has 70 sample rows (23 de-1.1, 47 de-1.2); ReestimatedValueAmount has 68 (16 + 52).
- `normalise_de1` (project.rs:8040-8054) renames them before `NoticeState::read`.
- The lot resolves through the existing alias `DE1-NoticeResult-LotResult-TenderLot-ID` → `BT-13713-LotResult` (project.rs:1237). DE1 lots are kind `Lot` (`de1_lot_kind`, 8169-8175).

**DE1 `Overall*` totals**
- They follow BT-118 and BT-1118: 37 sample rows (10 + 27) and 34 (4 + 30).
- The alias gate `every_de1_alias_target_is_a_field_the_projection_reads` (project.rs:10928) refuses an alias whose target nothing reads, so leaving them out is enforced.

**Bare `DE1-FrameworkMaximumAmount`**
- **Target.** Aliased to `UBL-FrameworkMaximumAmount`, not to `BT-271-Procedure` as the triage note said.
  - It is the same root `efbc:FrameworkMaximumAmount` that eforms/index.rs:967-986 already claims under that id on EU-minor notices (issue 195).
  - index.rs deliberately keeps it off BT-271 as "the national notice-level variant".
  - Both targets fold identically to Tender-scope `framework_maximum` (AMOUNTS full-id entry, project.rs:208). The UBL target gives one element one id in both dialects.
- **Size.** 4 rows in 6,000 de-1.1 notices and 0 in de-1.2; about 100 notices by that rate.
- **Hazard.** It is a Procedure registration, the same `lot_only` hazard used to reject BT-118. It is accepted here because:
  - it is parity with the EU path that already exists, not a new policy;
  - it touches about 100 notices;
  - it is a repaired dialect gap.
- It is the only piece of unit 2 that can raise a head, so it gets its own commit and its own pass.

**BT-710 / BT-711**
- Lowest and highest tender received are losing bids, not values of the procurement.
- BT-711 would become the head: it exceeds the proxy on 311 rows. The pin is `eforms-chain/4-can-29-380868-2026.xml`, BT-711 €2.29 m against an €816 k estimate.

**BT-156 / BT-1561 / BT-157**
- 1 and 4 sample rows, and no slip needs them.
- Their true scope is a LotsGroup named through BT-556, which the results graph does not resolve. Filed as a no-key lot figure, they would null `lot_keys`. Deferred until a fixture and a reason exist.

### What "partner-only" means (the invariant the build must hold)

A `result_framework_*` amount:
- is written to `tender_version_amounts` with its `lot_id`, and served in REST `amounts` and `v_tender_amounts`;
- is a `ScalePartners` figure at `FigureScope::Lot(Some(key))` through `scoped_amounts`, with no code change (canonical.rs:3590-3612);
- is never a head candidate, never a lot-value candidate, and never enters the corroboration map (it neither corroborates nor is corroborated);
- never counts in `head_procedure`, `head_lot_sums` / `head_lot_max` (492 (d)), or `head_lot_fields` / `head_lot_field_figures` (505);
- supersedes only itself, per lot;
- does not count toward data-quality value completeness.

So it only adds partners. A new partner can refuse but not raise, with the one exception class in Risk 1.

### Code changes

**Commit 1: "506: result-level framework values as lot-scoped scale partners"**

`crates/store/src/canonical.rs`
1. After `QUALITY_WITHHELD` (3044), add:
   ```rust
   pub const RESULT_FRAMEWORK_MAXIMUM: &str = "result_framework_maximum";
   pub const RESULT_FRAMEWORK_REESTIMATE: &str = "result_framework_reestimate";
   pub const PARTNER_ONLY_AMOUNT_FIELDS: &[&str] = &[RESULT_FRAMEWORK_MAXIMUM, RESULT_FRAMEWORK_REESTIMATE];
   pub fn electable_amount_field(field: &str) -> bool { !PARTNER_ONLY_AMOUNT_FIELDS.contains(&field) }
   ```
   The doc comment states the invariant above and cites issue 506.
2. `head_value_eur_cents_with`: in the candidate arm at 3204-3205, add the guard: `if quality.is_none() && !sentinel_amount(*cents) && electable_amount_field(field)`. Add one sentence to the comment block at 3195-3217.
3. `elect_lot_value`: at 3302, `if !electable_amount_field(a.field) || a.quality.is_some() || sentinel_amount(a.cents) { continue; }`. This covers the fold's caller at 30895-30907 with no edit there. The read layer reads the stored value (read.rs:4080-4095).
4. `add_head_amount` (3570): return early when `!electable_amount_field(field)`. Putting the guard inside the pub function covers `set_head`'s loop (3627, fed by `amounts()` at 3725-3731) and any future caller. The corroboration test in `refuses_by_partner` (3945-3950) then needs no edit. Update the doc at 3568-3569 and the `head` field doc (3506-3510).
5. `set_head`:
   - `head_procedure` push (3633-3638): add `&& electable_amount_field(field)`. This is defensive; partner-only facts are never at Tender scope.
   - `per_lot` loop (3660-3666): bind `field` and add the guard, so 492's (d) sum is unchanged.
   - `head_lot_fields` / `head_lot_field_figures` loop (3692-3706): add the guard, so 505's tables are unchanged.
6. `add_version_figures` gets no code change. Update its doc and add a "Partner-only amounts (issue 506)" paragraph to the `ScalePartners` doc (3388-3473).

`crates/ingest/src/project.rs`

7. After AMOUNTS (which ends at 235, before CLASSIFICATIONS at 236), add:
   ```rust
   /// Issue 506: framework values of a LotResult. A LotResult has no Lot ancestor, so these
   /// reach their lot through BT-13713 (`RawResults::lot_of`) and never through `scope_of`;
   /// never lot-null (492 decision (c)). Stored as partner-only amounts (canonical.rs).
   const RESULT_LOT_AMOUNTS: &[(&str, &str)] =
       &[("BT-709", canonical::RESULT_FRAMEWORK_MAXIMUM), ("BT-660", canonical::RESULT_FRAMEWORK_REESTIMATE)];
   ```
   It is stem-keyed through `canonical_name` (6600-6605). `stem` gives "BT-709" from "BT-709-LotResult" (6307). No other id has these stems; `BT-195(BT-709)` has the stem `BT-195(BT-709)` and is a Code.
8. `NoticeState::read`:
   - Before the values loop (4423), declare `let mut result_amounts: Vec<(String, Fact)> = Vec::new();`.
   - In the `NoticeValue::Amount` arm (4509-4541), compute `let deferred = canonical_name(RESULT_LOT_AMOUNTS, field_id);`.
   - Take the target as `deferred.clone().or_else(|| amount_target(..))`.
   - Build the `Fact::Amount` exactly as today. `tax_basis` and the issue-372 `quality` are keyed by the value's own section and stem, so a `BT-195(BT-709)` under the RES section still marks the fact withheld (`withheld_source_fields`, 6358-6377).
   - If `deferred.is_some()`, push `(value.section_id.clone(), fact)` and yield `None`, so the scope insert at 4579-4589 never sees it. Otherwise yield the fact as today.
9. After the roles re-scoping (4663-4675) and before `NoticeState {` (4686), while `lots` is still the map:
   ```rust
   for (section, fact) in result_amounts {
       if let Some(lot) = raw_results.lot_of(&sections, &section)
           .and_then(|key| lots.get_mut(&key))
           .filter(|l| l.kind == "Lot")
       {
           lot.facts.insert(fact);
       }
   }
   ```
   Add a comment saying anything unresolved is dropped, never placed at Tender scope. This step has to sit after `read_results` (4652), for the same reason the roles use their two-step. Several LotResults on one lot each add a fact, which is harmless because none is elected. `read` is synchronous, so no `run_spec` future grows (CLAUDE.md stack note).
10. In `DE1_FIELD_ALIASES`, after 1156, add the two LotResult aliases. Add a comment that the two `Overall*` ids are deliberately not aliased, with a pointer to this decision and to the gate at 10928.
11. `has_destination`, `Channel::Amount` (6459-6463): add `|| canonical_name(RESULT_LOT_AMOUNTS, field_id).is_some()`. This updates the sieve (`table_reads`, section 13, `/admin/unmapped-fields`) and lets the alias gate accept the new aliases. Add one sentence to the doc at 6427-6436: an unresolved LotResult figure is dropped by design.
12. `amount_target` (7144-7156) and `AMOUNTS` are unchanged. BT-118 and BT-1118 stay unread.

`crates/ingest/src/data_quality.rs`

13. The FIELDS `value` spec (90): set `predicate: Some(VALUE_PRESENT)` with
    `const VALUE_PRESENT: &str = "field NOT IN ('result_framework_maximum', 'result_framework_reestimate')";`
    Amend the FIELDS doc comment. Section 16's band listing (1660-1705) is unchanged: it already reads every amount as a partner, which now matches the election.

Docs and surface

14. `crates/app/src/v1/sql.rs`:
    - In the `v_tender_amounts` note (991-1000), name the two fields: always lot-scoped, never elected, a scale partner only.
    - In the `value_cents` note (1143-1148), say the pick excludes them.
15. `crates/app/src/v1/docs.rs`:
    - In the amounts bullet (774-796), add one sentence: the award notice's framework maximum and re-estimate (BT-709/BT-660) are kept in `amounts` at their lot and act only as partners of the scale rules.
    - In the lot-value paragraph (515-520), say: "nor from the award notice's framework values".
    - In the `currency` filter's row, say a Tender now matches when its only amount in that currency is such a row (read.rs:1201-1209 checks any amount row).
16. `CHANGELOG.md`: add "Unreleased (issue 506)". It covers:
    - the two new `field` values (additive under ADR-0015 D1);
    - a figure of €1 bn or more that is exactly 100× (or 10^k×) such a figure is now refused;
    - the `currency` filter note.
17. `docs/operations.md`: add a subsection "Result-level framework values as scale partners (issue 506)" after the residual rule (~675-681). It holds the drain below.
18. No `PROJECTION_EPOCH` bump: this is a profile-scoped mapping change plus a scale-rule input (canonical.rs:1764-1798; 492 precedent). No new job kind and no `Spec` arm.

**Commit 2: "506: alias the bare DE1-FrameworkMaximumAmount"**

19. In `DE1_FIELD_ALIASES`, add `("DE1-FrameworkMaximumAmount", "UBL-FrameworkMaximumAmount")` with a comment citing eforms/index.rs:967-986.
20. Add its test and one CHANGELOG sentence.

### Tests

Run them through `ops/check.sh`. Focused runs use the gate's flags and package set with `--features tender-db/server`, as CLAUDE.md requires.

`canonical.rs` lib tests (beside 33963-34700):
- `a_partner_only_amount_is_never_elected`:
  - A head whose only figure is a lot `result_framework_maximum` elects None, and `elect_lot_value` returns None.
  - Beside a smaller `estimated_value`, the estimate wins both elections.
- `a_result_lot_figure_refuses_its_x100_slip_395737`:
  - One Lot, in BGN. `estimated_value` 734,820,000 at Procedure and at Lot; `framework_maximum` 299,867,940,000 at the lot; `result_framework_maximum` 2,998,679,400 at the same lot.
  - Expected: the head is 734,820,000 BGN in EUR, and the lot value is 734,820,000.
  - Without the partner row the slip is elected. This pins today's behaviour.
- `a_result_reestimate_refuses_its_x100_slip_627219`:
  - The same shape in EUR, with `result_framework_reestimate` 3,322,770,833.
  - Expected: head and lot value 666,666,667.
- `a_partner_only_figure_never_corroborates`:
  - A k ≥ 3 slip F with a partner-only F in the head stays refused.
  - `add_head_amount("result_framework_maximum", ..)` is a no-op.
- `partner_only_figures_stay_out_of_lot_sums_and_residuals`:
  - 8800131's kept-framework shape plus a large partner-only lot figure (one that would push the sum past F if it counted) stays kept.
  - 8784848's residual shape still fires.
- `a_lot_partner_from_its_own_result_voids_the_sibling_exemption`:
  - 8748271's genuine shape is kept with no partner row.
  - Its ×100 lot is refused once that lot's own result states F/100.
- `a_new_k3_partner_moves_an_x100_framework_total_to_the_corroboration_test`: pins Risk 1's path switch.
  - A ×100-exempt framework total gains a partner-only F/1000 and is refused unless corroborated.
  - A corroborated ×100-refused F that gains one is kept.
- Add partner-only facts to the chain of `the_running_scale_rule_is_of_chain_of_each_prefix` (33963).

`project.rs` lib tests (the `Parsed` builder shape at ~9024-9806):
- `a_lot_result_framework_value_lands_on_its_lot_through_bt_13713`: two Lots and two LotResults; each value lands on its BT-13713 lot; nothing at Tender scope.
- `an_unresolved_lot_result_amount_is_dropped_not_lot_null`: a LotResult with no BT-13713, one naming an absent lot, and one naming a LotsGroup each yield no fact anywhere.
- `a_withheld_lot_result_value_keeps_its_marker`: `BT-195(BT-709)` under the RES section's FieldsPrivacy gives quality `withheld` on the lot fact.
- `the_de1_lot_result_framework_values_alias_onto_bt_709_and_bt_660`: an eforms-de-1.2 notice with the DE1 ids and the DE1 TenderLot-ID gives the same facts.
- Commit 2: `a_bare_de1_framework_maximum_is_a_tender_framework_maximum`.
- Sieve pins:
  - `table_reads("notice_amounts", ..)` is true for BT-709-LotResult, BT-660-LotResult and both DE1 LotResult spellings.
  - It is false for BT-118-NoticeResult, BT-1118-NoticeResult, both DE1 `Overall*` ids, BT-710-LotResult and BT-711-LotResult. That pins the decisions to leave them.
- The alias gate (10928) and `has_destination_answers_per_channel_not_per_field` (10736) stay green.

`crates/ingest/tests/project.rs` (ingest + project, then SQL):
- `result_level_framework_values_land_on_the_results_lot`, on `eforms/can-cvd-legacy-00412845-2025.xml`:
  - LOT-0000 carries `result_framework_maximum` 400,000,000 NOK cents and `result_framework_reestimate` 200,000,000.
  - No Tender-scope row of either field exists.
  - BT-118 and BT-1118 add no row.
  - The lot's `framework_maximum` is unchanged, so nothing was superseded.
  - The head and the stored lot value are unchanged.
- `each_lot_result_figure_goes_to_its_own_lot`: `eforms/can-cvd-lot-00054478-2025.xml`; each BT-660 is on its BT-13713 lot.

Store integration (`lot_value_election.rs`, `head_election_agreement.rs`):
- `tender_version_lots.value_*` and `tenders.current_value_eur_cents` ignore partner-only rows.
- The read layer's `elected` pick (read.rs:2139-2146) serves the same cents and currency when a partner row ties the head's `eur_cents`.

data_quality: `VALUE_PRESENT` names exactly `PARTNER_ONLY_AMOUNT_FIELDS`, checked quoted and by count.

Golden: `project_apply.snapshot` and `project_refold.snapshot` must stay **byte-identical**.
- No fixture in either corpus (project_golden.rs:155-176, 413-424) carries these fields or a root FrameworkMaximumAmount (checked by grep).
- A golden diff means a bug.
- The rederive E2 test (985-1001) compares two folds, so it stays green.

### Drain (unit 3)

**0. Cohort read before the wet run (precondition; run before deploy).** It reads the notice layer only.
- Script `.scratch/tender-db/506-result-amounts/cohort.py`, in census.py's shape: bounded `/v1/sql` through `/root/sq.sh`, paced, never retry a 408.
- **Cohort.** The union of:
  - the Tenders `refold-value-band` would stamp: a stored head of €1 bn or more, or any stored lot value of €1 bn or more in any version. Read them as 492/505 unit 1 did (keyset pages off `tenders_current_value_eur` plus the lot-value read);
  - the ids in `.scratch/tender-db/492-x100/` (the 56 + 9 adjudicated, the 13 lot pairs), 492's 9 corrected Tenders outside the sample, and 505's 5. These are the Tenders whose ×100-refused figure the path switch could re-admit.
- **Per batch of ≤ 200 Tenders:**
  - the version notice ids (`tender_versions` PK prefix);
  - the stored figures (`tender_version_amounts` and `tender_version_lot_results` by `tender_id` prefix);
  - `notice_amounts WHERE notice_id IN (..) AND field_id IN` the BT-709/BT-660 ids, both DE1 LotResult spellings and `DE1-FrameworkMaximumAmount`.
- **Report:**
  - every stored figure F ≥ €1 bn with a NEW exact partner F = P × 10^k (k ≥ 2, P > 1,000 cents, P a result figure of the chain in F's currency), classed as one of:
    - new-×100 (no partner before);
    - switch (a new k ≥ 3 partner where only a ×100 partner existed);
    - new-k3;
  - whether F is corroborated in its head version;
  - any DE1 bare figure above the head;
  - the count of carrier LotResults whose BT-13713 is missing or names no Lot of the notice (the drop count; no threshold);
  - the "before" heads and lot values.
- **Adjudication.** Two independent readers per listed Tender (492/505 practice). Expected: 395737, 627219 and few others.
- **Stop and re-panel** if any of these is predicted:
  - a rise;
  - more than one genuine figure refused (one is recorded, like 8819939);
  - a list beyond about 20 Tenders.

**1. Deploy** both commits in a queue gap.

**2. Pass A, the value band:**
- `{"kind":"refold-value-band"}` dry, then `"dry_run":false`, then `project`.
- Add `refold-notices` for any predicted Tender outside the band (expected none).
- Verify:
  - `stamped N` equals the dry count;
  - `compare: N verified, M corrected`. M includes band carriers that only gained rows;
  - the Tenders whose head or any lot value moved, from the before/after re-read, are a subset of the adjudicated list;
  - 395737 and 627219 are re-read (below);
  - anything else that moved is a finding.

**3. Pass C, the DE1 bare alias:**
- `{"kind":"refold-fields","profiles":["DE1-FrameworkMaximumAmount"],"tables":["notice_amounts"],"expect":1}`, then the same with the reported count.
- Read every head that moved against its notice and list it on this issue.

**4. Pass B, storage completeness (R3):**
- `{"kind":"refold-fields","profiles":["BT-709-LotResult","BT-660-LotResult","DE1-NoticeResult-LotResult-FrameworkAgreementValues-MaximumValueAmount","DE1-NoticeResult-LotResult-FrameworkAgreementValues-ReestimatedValueAmount"],"tables":["notice_amounts"],"expect":1}`.
- Size it first. Its sizing counts eforms-sdk-1.15 and eForms-DE 2.x, which the census missed.
- Apply operations.md's stop rule (~907-911), then run wet on a quiet box with disk headroom checked. The size precedent is 489's F14 refold: 191,493 notices, 156,263 Tenders.
- Verify:
  - N + M equals stamped;
  - no served head or lot value moves: the band read before and after is identical, and the cohort read predicted no rise outside it;
  - any move is a finding.
- Until B runs, storage is lazily incomplete: a carrier Tender gains its rows on its next refold.

### Expected prod effects

- **Heads.**
  - No new candidate.
  - Moves happen only through refusal of figures of €1 bn or more that gain an exact 10^k partner (falls, all inside Pass A's cohort), plus Risk 1's switch class. The cohort read enumerates both in advance.
  - Expected corrected heads:
    - 395737: €1,533,200,431.53 → BGN 7,348,200 (≈ €3,757,075);
    - 627219: €3,322,770,833 → €6,666,666.67;
    - plus the adjudicated list, if any.
  - Not moving:
    - the 234 BT-709 and 138 BT-660 rows above the head proxy;
    - the 17.4 % (BT-709) and 15.1 % (BT-660) of rows on notices with no mapped figure.
- **Lot values.**
  - Same as heads: refusals only.
  - The slips' lots fall to their BT-27-Lot: BGN 7,348,200 and €6,666,666.67.
- **492 / 505 state.**
  - The exemption inputs (b)(c)(d) and 505's tables are unchanged.
  - Partner metadata can only tighten. A lot partner adds its own key to `lot_keys` (`add_figure`, 3542-3557), so a lot whose own result states F/100 loses the sibling exemption.
  - `lot_only` cannot flip to false in commit 1, because nothing new registers at Procedure scope. Commit 2 does register at Procedure scope, on about 100 notices.
- **DE1 bare:** about 100 de-1.1 notices gain a Tender-scope `framework_maximum`, which can raise their heads. Pass C lists them.
- **API and SQL vocabulary.**
  - Two new `field` values, always with `lot_id`, in REST `amounts`, `tender_version_amounts` and `v_tender_amounts` (ADR-0015 D1, additive).
  - `?currency=` matches Tenders whose only amount in that currency is such a row.
  - The unmapped-field report stops listing BT-709-LotResult, BT-660-LotResult and both DE1 LotResult spellings. BT-118, BT-1118, the `Overall*` ids and BT-710/711 stay listed.
- **Storage and feed.**
  - About one new row per carrying lot per version, from the award notice on.
  - Pass B writes ADR-0017 correction rows for every carrier Tender whose stored rows differ, although no served value moves there.
  - Daily: a framework CAN now emits a lot `changed` row where it would otherwise have restated identical lot facts.
- **Data quality.** Value completeness is unchanged (change 13). Per-field densities show the new fields.
- **Golden:** no change.

### How the two slips end

**395737** (BGN, one Lot, award notice 44551095; all four versions carry the same figures)
1. **Fold.** BT-709-LotResult (2,998,679,400) sits in the RES section.
   - `read_results` (4652) gives it BT-13713 = the Lot.
   - That Lot is in `lots`, kind `Lot`, because the same notice states BT-271-Lot there.
   - The lot gains `result_framework_maximum` 2,998,679,400. BT-660 (2,265,845,400) becomes `result_framework_reestimate`. BT-710 stays unmapped.
   - Nothing is superseded.
2. **Partners.** `add_version_figures` registers both as Lot(Some(lot)) partners, both above the 1,000 floor.
3. **Head** (3176-3248).
   - Candidates: `estimated_value` 734,820,000 (Procedure and Lot) and `framework_maximum` 299,867,940,000 (HeadLot, ≈ €1.533 bn, at or above the gate). The partner-only figures are skipped.
   - `residual_slip`: the field differs from P's, and `head_lot_count` = 1, so it returns false.
   - `partner()` (3748-3758): /10^3 = 299,867,940 and /10^4 = 29,986,794 are not figures, and 10^5 through 10^8 do not divide. None.
   - `partner_x100` (3761-3765): /100 = 2,998,679,400 is a figure. HeadLot returns true (3939): **refused**.
   - Head = BGN 7,348,200 ≈ €3,757,075, which is 505's procedure figure for this Tender.
4. **Lot value** (3295-3317): Lot(Some) with a ×100 partner. `sibling_lot` needs `head_lots` ≥ 2 (3800), and this is one lot: **refused**. The lot value is BGN 7,348,200 in every version.

**627219** (EUR, one Lot, direct award, notice 24995400 = 00584430-2025)
1. **Fold.** BT-660-LotResult (3,322,770,833) resolves through BT-13713 to the Lot and becomes `result_framework_reestimate`. BT-1118 (same cents) stays unmapped and is not needed.
2. **Partners.** It registers as Lot(Some(lot)).
3. **Head.**
   - BT-271-Lot is 332,277,083,300 (€3.32 bn, HeadLot).
   - Residual: none, one Lot.
   - k ≥ 3: the figure mod 1000 = 300, so no k ≥ 3 can divide.
   - `partner_x100`: /100 = 3,322,770,833 is a figure: **refused**.
   - Head = €6,666,666.67.
4. **Lot value:** refused (one lot, no sibling exemption). The lot value is €6,666,666.67.
5. **Earlier versions.** If the chain holds an earlier version without the result figures, that version's lot row keeps the slip (492's unreachable class). The head is unaffected.

The adjudicated ceilings (BGN 29,986,794; €33,227,708.33) stay visible in `amounts` at the lot and are not served.

### Residual risks

1. **Path switch.** A new partner F/10^k with k ≥ 3 moves F from the ×100 test, which has 492's exemptions, to the k ≥ 3 test, which has corroboration only (`refuses_by_partner`, 3927-3951). Three things follow:
   - A genuine ×100-exempt framework total can be refused, for example a round €1 bn ceiling beside a new €1 m re-estimate.
   - A corroborated ×100-refused slip can come back.
   - If a new partner refuses a procedure total P, P stops exposing a 505 residual slip.

   No measured head reaches these paths: none of 492's 56 ×100 heads had a k ≥ 3 partner, and 505's five are pre-eForms or FTS. The cohort read enumerates every case in advance, and tests pin both directions.
2. **Unresolved LotResults are dropped.** This happens when BT-13713 is missing or names no Lot of the notice. The cohort read counts it. The direction is conservative: no partner, which is today's state.
3. **BT-118/BT-1118 are deferred.** A procedure-level slip whose only partner is a notice total stays unrefused. The census has 0 and 1 such pairs, and the 1 is below the floor. Reopening needs the "notice total equal to a lot figure is that lot's figure" registration.
4. **Drain cost.** Pass B is a large R3 refold: correction rows for every carrier, new rows on a tight disk. Size it with `expect:1` and run it in a quiet window.
5. **Guard discipline.** Every current election site calls `electable_amount_field`: the head arm, `elect_lot_value`, `add_head_amount`, and `set_head`'s three tables. A future site must call it too, and the tests pin each site.
6. **Analyst-visible rows.** `MAX(cents)` over `tender_version_amounts`, and `?currency=`, now see partner-only rows. This is documented and additive.
7. **DE1 bare alias** can raise heads on about 100 notices and registers at Procedure scope. It is parity with issue 195, and Pass C lists its moves.
8. **The served value is the estimate, not the ceiling.** Serving the adjudicated ceilings would mean electing BT-709/BT-660. Reopen only with a head-version census that simulates both the head and the stored lot value (risk-1 fallback reasoning from the runner-up).

### Rejected

- **Electing BT-709/BT-660 as head and lot candidates under new names** (runner-up, 46 points).
  - Lot values would move broadly and unmeasured: BT-709 differs from its own lot's BT-271/27 on 96.8 % of rows (86.5 % outside sdk-1.6), and BT-660 equals nothing mapped on 75 % (50 % outside).
  - Heads would appear on CAN-only chains from what is mostly the award (contradicting docs.rs:515-520 and 492).
  - A new lot candidate makes the first election pass succeed where today it falls back to a carried PIN Part (507's `elect(false)`, 3237-3248). Heads could fall with no gate, outside the band.
  - Its tender-scope BT-118/BT-1118 would poison `lot_only` / `lot_keys` in 492's genuine multi-lot shape.
- **Same-field mapping** (BT-709 and BT-118 → `framework_maximum`; BT-660 and BT-1118 → `estimated_value`) (40 points; judged fatal).
  - Per-lot supersession would delete the CN's ceiling and estimate on every CAN that does not restate them. That is structural on VEAT 25-27 and modifications 38-39.
  - Tender-scope totals would delete the CN's procedure figures.
  - Before SDK 1.10, BT-709 is not forbidden on non-framework lots. A BT-709 copy of BT-720 in `framework_maximum` would corroborate an equal BT-161 and switch off 471's k ≥ 3 refusal (canonical.rs:3401-3406).
- **Plain `AMOUNTS` mapping:** unit 1's finding. A LotResult figure lands lot-null.

### Corrections to unit 1

- Unit 1's proposed decisions 1–3 (`framework_maximum` / `estimated_value`, tender-scope BT-118/BT-1118) are superseded by the above.
- "Mapping BT-709/118/660/1118 changes `project_apply.snapshot` and `project_refold.snapshot`" is wrong. Neither corpus carries these fields. `can-cvd-legacy-00412845` and `can-pat-social-00250633` are only in the rederive E2 corpus (project_golden.rs:985-1001). The goldens must not change.
- "New 10^k partners ≤ 0.1 %" is the lumped total. Outside sdk-1.6 the rates are 0.3 % (BT-709) and 0.8 % (BT-660), counted at any magnitude in both directions. The census holds no pair at or above the gate; the cohort read is what measures that.

### Next (unit 3)

1. Run the cohort read and adjudicate the list.
2. Build commits 1 and 2.
3. Run `ops/check.sh`, writing to a file, and read `GATE-EXIT=`.
4. Push by explicit ref and deploy in a queue gap.
5. Run Pass A, then Pass C, then Pass B, and verify each as above.
6. Re-read 395737 and 627219 at every seq.