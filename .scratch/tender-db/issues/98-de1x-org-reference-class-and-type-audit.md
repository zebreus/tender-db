# 98 — eForms-DE 1.x organization-reference class is dead: `id` vs `id-ref` typing + missing role aliases

Status: RESOLVED — VERIFIED IN PROD 2026-08-15. All three fix parts landed in `a50ad0d`
("issue 98: DE-1.x organization references — flag them, alias them, gate them", 2026-08-02) and are
deployed (ancestor of the serving rev `9bdfe80`): `de1_mark_reference` flags the `OPT-300-`/`OPT-301-`
families `is_ref=true` in `normalise_de1`, `DE1_FIELD_ALIASES` carries all 20 reference fields, and the
vendored `fields-de-1.x.json` now types 28 fields `id-ref`. The fix materialised in the full rebuild and
is now measured live on the box.

**Provenance measured (not presence), the gate this issue demanded.** Bounded `/v1/sql` over a recent
de-1.x window (`notices.id BETWEEN 26717000 AND 26722031`, queue idle):

- 651 party rows across 71 DE-1.x notices; **525 (80.6%) carry `mention_notice_id = n.id`** — they
  originate from the DE notice itself. **Pre-fix this number was 0** (the visible ~35% was all TED-twin
  carry-forward). The remaining ~19% are legitimate carry-forwards from merged twins, exactly as designed.
- The role class recovered is precisely this issue's table, every role DE-origin: **Procedure-Buyer 71
  rows across 71 notices (100% of projecting notices — the C6 gate that read a false 35%)**, Lot-ReviewOrg
  157, Lot-AddInfo 156, Lot-TenderReceipt 62, Lot-ReviewInfo 41, Tenderer 35, Lot-Mediator 2,
  Procedure-SProvider 1. Role strings are the eForms suffix verbatim (`Procedure-Buyer`, `Lot-ReviewOrg`,
  …), matching what TED twins already publish onto these same Tenders.

Award **winners** are deliberately NOT part of this resolution — they are issue 100 (a parse-layer
section-id defect, design decided, deferred), and `read_results` never gates on `is_ref`, so 98 cannot
and does not touch them. Winners staying near 0% from DE after this fix is expected and tracked there.
Was: open — DISCOVERED 2026-08-02 (sdk-vendor, from the post-refold snapshot `tender-db-1785661162.db`).
Kind: correctness / completeness (projection mapping — the org layer)
Blocked by: —
Relates to: 75 (the empirical DE-1.x inventory — this is its defect), 85 (the fact-layer fix this sits on top of),
88, 86/48 (the Group-2 org-projection batch this joins), 100 (winners — the sibling defect this does NOT fix)

## Symptom

The issue-85 re-fold landed: DE-1.x tenders now carry title, description, CPV, NUTS, **lots**, dates,
amounts and subtype at 98.9% of sampled versions. The verification's `C6` gate then hard-failed:

```
FAIL C6   buyer party (source 100.0%) = 383/1094 (35.0%, expected ≥ 90%)
```

**The 35% is an illusion. The true DE-1.x contribution is 0%.** Every party row sitting on a DE-1.x
version — **698 of 698** in the sampled window — has `mention_notice_id` pointing at an
`eforms:eforms-sdk-1.7` notice, i.e. the TED twin the DE notice merged with. The fold is correctly
carrying parties forward along the merged chain; not one party row originates from a DE-1.x notice.
The 35% is simply the share of DE versions that sort after a TED version in their chain.

Award winners show the same thing from the other side: of 247 award-bearing DE versions in a de-1.2
window, **16 (2%)** have a winner — and those are the same TED carry-forward.

So: **the fold fixed the fact layer; the organization layer is still empty.** A DE-1.x tender renders
with a German title, CPV, NUTS and lots, and no buyer and no winner.

## Root cause — TWO defects, both in the vendored inventory (issue 75)

### 1. Every reference field is typed `id`, never `id-ref`

`crates/ingest/sdk/fields-de-1.x.json` types 84 fields `id` and **zero** `id-ref`. The chain:

- `sdk.rs:112` — `Field.kind` is `#[serde(rename = "type")]`, i.e. the JSON's `type` **is** `kind`.
- `value.rs:71` — `is_ref: field.kind == "id-ref"` → every DE-1.x id is stored `is_ref = 0`
  (confirmed in the snapshot: zero `notice_ids` rows with `is_ref=1` across the cohort).
- `project.rs:1567` — org roles are collected **only** from `NoticeValue::Id { is_ref: true, .. }`
  → `raw_roles` is empty → `bind_organizations` emits no `Fact::Party`.

The empirical generator inferred types from observed values, and a reference is
indistinguishable from an identifier by its lexical form — so it labelled all of them `id`.

**This does not affect which table a value lands in**: `decide()` maps `"id" | "id-ref"` to
`Decision::Ids` alike, which is why the ids are all present and correct in the notice layer. The
typing is only wrong for the *reference* semantics.

### 2. Fifteen of the twenty reference fields have no alias at all

`DE1_FIELD_ALIASES` covers buyer, tenderer and subcontractor. It does **not** cover the lot- and
result-level role parties — so even with `is_ref` fixed, `role_name()` would return `None` for them
and they would still produce nothing. Fixing only `is_ref` would have recovered the buyer and left
nine role classes empty — a second failed fold.

## The complete reference class (measured, not guessed)

Derived from the snapshot: values that point at a section **other than their own** in the same notice.
`self_id = 0` for every row below, so none of these are identifiers. Counts are one 400-notice window.

| DE-1.x field | → eForms id | n | target | aliased today |
|---|---|---:|---|---|
| `DE1-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID` | `OPT-301-Lot-ReviewOrg` | 693 | Organization | **no** |
| `DE1-ContractingParty-Party-PartyIdentification-ID` | `OPT-300-Procedure-Buyer` | 462 | Organization | yes |
| `DE1-ProcurementProjectLot-TenderingTerms-AdditionalInformationParty-PartyIdentification-ID` | `OPT-301-Lot-AddInfo` | 453 | Organization | **no** |
| `DE1-TenderingTerms-AppealTerms-AppealInformationParty-PartyIdentification-ID` | `OPT-301-Lot-ReviewInfo` | 245 | Organization | **no** |
| `DE1-NoticeResult-LotResult-TenderLot-ID` | `BT-13713-LotResult` | 215 | Lot | yes¹ |
| `DE1-TenderingTerms-TenderRecipientParty-PartyIdentification-ID` | `OPT-301-Lot-TenderReceipt` | 195 | Organization | **no** |
| `DE1-NoticeResult-LotTender-TenderLot-ID` | `BT-13714-Tender` | 186 | Lot | yes¹ |
| `DE1-NoticeResult-TenderingParty-Tenderer-ID` | `OPT-300-Tenderer` | 170 | Organization | yes |
| `DE1-ProcurementProjectLot-TenderingTerms-DocumentProviderParty-PartyIdentification-ID` | `OPT-301-Lot-DocProvider` | 92 | Organization | **no** |
| `DE1-TenderingTerms-AppealTerms-MediationParty-PartyIdentification-ID` | `OPT-301-Lot-Mediator` | 79 | Organization | **no** |
| `DE1-ProcurementProjectLot-TenderingTerms-TenderEvaluationParty-PartyIdentification-ID` | `OPT-301-Lot-TenderEval` | 79 | Organization | **no** |
| `DE1-NoticeResult-SettledContract-SignatoryParty-PartyIdentification-ID` | `OPT-300-Contract-Signatory` | 31 | Organization | **no** |
| `DE1-ContractingParty-Party-ServiceProviderParty-Party-PartyIdentification-ID` | `OPT-300-Procedure-SProvider` | 31 | Organization | **no** |
| `DE1-Changes-Change-ChangedSection-ChangedSectionIdentifier` | `BT-13716-notice` | 25 | Lot | **no** |
| `DE1-NoticeResult-LotResult-FinancingParty-PartyIdentification-ID` | `OPT-301-LotResult-Financing` | 8 | Organization | **no** |
| `DE1-NoticeResult-LotResult-PayerParty-PartyIdentification-ID` | `OPT-301-LotResult-Paying` | 7 | Organization | **no** |
| `DE1-NoticeResult-TenderingParty-SubContractor-MainContractor-ID` | `OPT-301-Tenderer-MainCont` | 2 | Organization | **no** |
| `DE1-NoticeResult-TenderingParty-SubContractor-ID` | `OPT-301-Tenderer-SubCont` | 2 | Organization | yes |
| `DE1-TenderingTerms-FiscalLegislationDocumentReference-IssuerParty-PartyIdentification-ID` | `OPT-301-Lot-FiscalLegis` | 1 | Organization | **no** |
| `DE1-ProcurementProjectLot-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID` | `OPT-301-Lot-ReviewOrg` | 1 | Organization | **no** |

¹ the two `TenderLot-ID` entries are aliased and already work: `read_results` matches
`NoticeValue::Id { value, .. }` and never tests `is_ref`, which is why the results *structure*
(lot_results 247, bids 216, contracts 217) lands while its org edges do not.

The eForms target ids are taken from the vendored EU SDK 1.13 `fields.json` by xpath, where every one
of them is typed `id-ref` — independent confirmation that this is one coherent class. The `Lot-`
variants are used for the procedure-scope DE fields too: eForms splits Lot/Part by the schemeName
predicate DE-1.x does not carry (issue 75), and `role_name()` uses the suffix verbatim, so the role
string matches what TED twins already produce (`Lot-ReviewOrg`, `Lot-AddInfo`, …).

**Cross-check:** the roles observed on DE versions via TED carry-forward — Lot-ReviewOrg 172,
Lot-AddInfo 121, Lot-TenderReceipt 87, Lot-ReviewInfo 54, Lot-Mediator 23, Lot-TenderEval 21,
Lot-DocProvider 19, Procedure-SProvider 97, Procedure-Buyer 90 — are *exactly* this table's roles.
The TED twin is publishing the same procurement, so the two lists agreeing is strong evidence the
mapping above is right.

## The broader audit — is there a THIRD class? No.

`type` (= `Field.kind`) drives exactly three things. All were checked:

1. **`decide()` → which value table.** `"id" | "id-ref"` → `Ids` alike, so the mis-typing costs
   nothing here. Every other type maps correctly, and the data agrees: texts, codes,
   classifications, amounts, dates, integers and numbers all land at 98.9% of versions.
2. **`Decision::Dates → match field.kind` (`"date"` vs everything else, `value.rs:44`).** The
   inventory types 35 fields `date` and 9 `time`, and every `EndTime`/`OccurrenceTime`/`IssueTime`
   is correctly `time` against its `EndDate`/`OccurrenceDate`/`IssueDate` partner. The parser
   reunites each pair into one instant (issue 03), and `C9` measures dates at 91.1% — so there is
   no date/time defect.
3. **`is_ref` (`value.rs:71`)** — defect 1 above.

So the gap list is complete: **one class, two defects.** No third class.

## Fix

1. **`normalise_de1`** (the in-memory pass at the 5 load sites) marks the 20 fields above
   `is_ref = true`. This is the deploy-now mechanism: `is_ref` is written at parse time, so fixing
   only the JSON would require re-parsing all 218,635 notices from the archive; doing it in the
   projection keeps this a **projection-only fix + scoped re-fold**, the same shape as issue 85.
2. **Extend `DE1_FIELD_ALIASES`** with the 15 unaliased entries, targets per the table.
3. **Correct the vendored `fields-de-1.x.json`** to type those 20 fields `id-ref`, so future ingests
   are right at the source. Idempotent with (1) — a value already `is_ref` stays `is_ref`.
4. There is a precedent for the shape at `project.rs:1538`: sdk-0.1 synthesises its buyer role
   because that dialect carries no OPT-300 reference at all. DE-1.x is the milder case — the
   references exist and are correctly shaped, they are simply not flagged.

## Verification the current suite does NOT catch (must be added before the next fold)

`C6` only just caught this, and only because the sampled rate was far below the source rate. **A
version can show a full set of parties while contributing none of them** — that is precisely what
made 0% look like 35%. The gate must assert provenance, not presence:

```sql
-- parties that ORIGINATE from the DE-1.x notice, not inherited from a merged twin
SELECT COUNT(*) FROM notices n
  JOIN tender_versions v ON v.caused_by_notice_id = n.id
  JOIN tender_version_parties p ON p.tender_id = v.tender_id AND p.seq = v.seq
 WHERE n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2')
   AND p.mention_notice_id = n.id;     -- pre-fix: 0. post-fix: must track the buyer rate.
```

Same for `tender_version_result_winners` (winner provenance) — the 2% winner rate needs its own gate.

## Note

This is my defect twice over: the issue-75 inventory recorded `type` but had no way to infer
reference semantics from values alone and I did not check it against the EU SDK's own `id-ref`
typing; and the issue-85 verification asserted that *facts* land without ever asserting that
*parties* land **from DE notices**. The fold mechanism itself is proven sound by this run —
conservation closes exactly (216,450 = 216,450 − 4,540), no double-count, no headless tenders, parse
layer intact. What is left is one metadata class, now fully enumerated.
