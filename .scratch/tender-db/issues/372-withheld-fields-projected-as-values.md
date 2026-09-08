# 372 — a WITHHELD eForms field is projected as if it were data: `-1.00` becomes an amount and `unpublished` becomes a currency

Status: ready-for-agent — **unit 1 sampled 2026-09-08 (see "Unit 1 — the census"): 8/8 `-1.00`
rows are `result_value`, and 4/4 of their notices declare the withholding explicitly. The BT-195 code
names the SOURCE field, which the projection's own `AMOUNTS` mapping already translates — so unit 2
needs no new vocabulary.** Was: needs-triage (filed 2026-09-08, found by issue 366's sentinel sweep on
its second corpus run — job 817)
Kind: defect (projection — the parse→canonical boundary), and the ROOT CAUSE behind the
corpus's largest sentinel class
Relates to: 366 (filters `-1` out of the head election — the symptom fix, and it stays
useful; this is the cause), ADR-0013 D5 + 173 + 274 (the withheld-field mechanism and
its reveal recheck, which already model this correctly at the NOTICE layer), 87
(quarantine relabelling), ADR-0004 (store as published)

## Observed

Issue 366's sweep, second run (job 817, deployed rev `b100e4a`):

| currency | value | rows | tenders |
| --- | --- | --- | --- |
| EUR | **−1.00** | 18,766 | **17,841** |
| PLN | −1.00 | 181 | 32 |
| **`unpublished`** | −1.00 | 141 | 92 |
| DKK | −1.00 | 80 | 78 |
| NOK | −1.00 | … | … |

Two things there are not publisher noise:

1. **`currency` holds the literal string `unpublished`** — confirmed stored, not a render
   artifact: `SELECT currency, count(*) FROM tender_version_amounts WHERE currency =
   'unpublished'` → 141 rows. That is a CODE sitting where a currency belongs.
2. **The `-1.00` convention is multi-currency** (EUR, PLN, DKK, NOK, and `unpublished`),
   which is what a *specified mechanism* looks like rather than one publisher's habit.

**And the fixture says exactly what the mechanism is.** From
`crates/ingest/tests/fixtures/eforms/can-withheld-29-00495618-2026.xml`:

```xml
<efac:FieldsPrivacy>
  <efbc:FieldIdentifierCode listName="non-publication-identifier">rec-sub-cou</efbc:FieldIdentifierCode>
  <cbc:ReasonCode listName="non-publication-justification">oth-int</cbc:ReasonCode>
  <efbc:PublicationDate>2030-07-26Z</efbc:PublicationDate>
</efac:FieldsPrivacy>
<efbc:StatisticsCode listName="received-submission-type">unpublished</efbc:StatisticsCode>
<efbc:StatisticsNumeric>-1</efbc:StatisticsNumeric>
```

and again on an award criterion:

```xml
<cbc:AwardingCriterionTypeCode listName="award-criterion-type">unpublished</cbc:AwardingCriterionTypeCode>
<cbc:Name languageID="NLD">unpublished</cbc:Name>
<efbc:ParameterNumeric>-1</efbc:ParameterNumeric>
```

So when a buyer withholds a field under BT-195/`FieldsPrivacy`, the eForms SDK writes the
literal code **`unpublished`** in the code slot and **`-1`** in the numeric slot. `-1` is
not a sentinel a publisher invented. **It is the SDK's documented "this value is
withheld" marker**, and the notice also states WHICH field is withheld, WHY, and the date
it becomes publishable.

## Why, exactly

**One sentence:** the notice layer models withholding correctly and the canonical layer
does not consult it, so a marker meaning "there is no value here" is projected as a value.

- The withheld mechanism IS modelled — at the notice layer. `notice_withheld_fields`
  (`crates/store/src/lib.rs:380-384`) is a VIEW over the BT-195/196/197/198 codes, and
  ADR-0013 D5's reveal recheck walks it (`Db::reveal_recheck`, `crates/store/src/lib.rs:1296`).
  Job 815 reports **284,243 withheld field(s) in corpus**, so this is a well-populated,
  already-trusted structure.
- **Nothing in the projection reads it.** The amount path takes the published numeric at
  face value, so `-1` becomes `Fact::Amount { cents: -100 }` and the sibling code
  `unpublished` becomes the `currency`. The canonical layer therefore asserts a €−0.01
  contract where the notice asserted "withheld until 2030-07-26".
- **Issue 366 fixes the symptom, correctly, and cannot fix this.** `sentinel_amount`
  refuses negatives so the head election skips them — right, and it stays right. But the
  row is still in `tender_version_amounts` claiming to be an amount; `?max_value=0` still
  reaches it (366 unit 4); `currency = 'unpublished'` still pollutes any currency
  aggregate; and a consumer reading the satellite directly still sees data where there is
  none. Filtering an extremum is not the same as not asserting the value.
- **This is the root cause of the largest sentinel class in the corpus** — 17,841 tenders
  on EUR −1.00 alone. 366's Leg A rule ("any negative amount … a documented publisher
  convention for 'not stated'") is right about the disposition and wrong about the cause:
  it is not a convention, it is a specification.

## Units

1. **Census, bounded**: how many `tender_version_amounts` rows are `cents = -100`, how
   many carry `currency = 'unpublished'`, and how many of their notices have a matching
   `notice_withheld_fields` row for the corresponding BT. The third number is the one that
   matters — it says whether the notice layer can already explain each marker, or whether
   some `-1`s are genuinely unexplained.
2. **Decide the disposition** (owner). ADR-0004 says store as published, so the parse
   layer must keep `-1`. The question is what the CANONICAL layer does: (a) do not emit
   the fact at all, (b) emit it with a `quality`/`withheld` marker — which is issue 366's
   proposed `quality TEXT` column, arriving here for a second, independent reason — or
   (c) emit NULL. (b) is the only one that keeps "withheld" distinguishable from "not
   published at all", and those are genuinely different claims.
3. **`currency = 'unpublished'` must never be stored as a currency** regardless of unit
   2's answer — it is not a currency code, and no consumer can do anything with it.
4. **Cross-check the other satellites.** The same marker pair appears on award criteria
   (`ParameterNumeric -1`) and submission statistics (`StatisticsNumeric -1`) in the same
   fixture, so the amounts column is unlikely to be the only place a `-1` was projected
   as data. Sweep the numeric satellites for the same shape before designing the fix.

## Unit 1 — the census, sampled and bounded (2026-09-08, rev `1ca7427`)

`notice_withheld_fields` **cannot be read unbounded**: `SELECT notice_id, withheld_field FROM
notice_withheld_fields LIMIT 3` returned 408 at the 10 s cap (the view GROUPs over every
`FieldsPrivacy` section corpus-wide, and `LIMIT` does not bound the aggregation). Not retried, per
`docs/agents/prod-box-reads.md`. The census is therefore a SAMPLE through indexed seeks, and the
underlying tables are queried directly rather than the view so the `notice_id` predicate actually
seeks instead of being blocked behind the GROUP BY.

**Every `-1.00` amount in the sample is on `result_value`.** An indexed `tender_id` range
(`tender_id <= 100000 AND cents = -100`, 8 rows) returned `field = 'result_value'` for all eight —
tenders 34, 189, 254, 288, 347, 511, 656, 728. The marker is concentrated in the AWARD value, which
is the commercially sensitive one, exactly as a confidentiality mechanism would predict.

**4 of 4 sampled notices carry an explicit withheld declaration for that field.** Probing
`notice_sections` (`kind = 'FieldsPrivacy'`) joined to `notice_codes`, per notice id:

| notice | BT-195 codes declared |
| --- | --- |
| 25390373 | `BT-195(BT-161)-NoticeResult` = `not-val`; `BT-195(BT-720)-Tender` = `win-ten-val` ×3 |
| 24622961 | `BT-195(BT-161)-NoticeResult` = `not-val` |
| 24666167 | `BT-195(BT-161)-NoticeResult` = `not-val` |
| 25405915 | `BT-195(BT-161)-NoticeResult` = `not-val`; `BT-195(BT-720)-Tender` = `win-ten-val`; `BT-195(BT-193)-Tender` = `win-ten-var` |

25390373 also carries `BT-197(BT-720)-Tender` = **`law-enf`** — the withholding REASON. The
`#0/#1/#2` section suffixes are per winning tender, which is why one notice yields several `-1.00`
rows.

### The finding that de-risks unit 2: the declaration matches the fact EXACTLY, by field id

`BT-161` → `result_value` is already in the projection's own amount mapping
(`crates/ingest/src/project.rs:135`). So the BT-195 code names the withheld SOURCE field, and the
existing `AMOUNTS` table already translates that source field to the canonical name the fact carries.

**No new vocabulary is needed for the fix.** The projection can ask, of a fact it is about to emit,
"does this notice declare a `FieldsPrivacy` withholding for the source field this fact came from?" —
and the answer is a lookup through a mapping that already exists. That removes the main design risk
unit 2 looked like it carried (inventing a correspondence between withheld-field codes and canonical
facts); the correspondence is the identity.

### What the sample does NOT establish

- **Corpus-wide coverage.** 4/4 is a sample, not a proof that every `-1` is declared. Unit 1's third
  number still wants a corpus count, and it needs an in-process job (the app reading its own database
  — outside the prod-box rule, as section 10 itself is) rather than `/v1/sql`. Cheapest route: fold
  the count into the weekly report beside section 10, where the scan is already affordable.
- **Whether any `-1` is UNdeclared.** That residue is the interesting number for unit 2's disposition:
  a declared withholding can be marked precisely, whereas an undeclared `-1` is a guess and might
  deserve to stay quarantined instead.
- **The other satellites** (unit 4). The same fixture shows `ParameterNumeric -1` on an award
  criterion and `StatisticsNumeric -1` on submission statistics, and neither was probed here.

## Done when

- no `tender_version_amounts` row carries `currency = 'unpublished'`;
- a withheld amount is distinguishable from an absent one in the canonical layer, and
  `/v1` says which;
- issue 366's negative-sentinel leg is re-read against this: it stays as defence in depth,
  but its doc must stop calling `-1` a publisher convention;
- the withheld-marker count in section 10 falls to whatever units 2–4 leave behind, and
  that residue is explained here.

*One issue because:* the `-1.00` amount, the `unpublished` currency and the award-criterion
`-1` are one mechanism (BT-195 `FieldsPrivacy`) projected through a layer that never asks
whether the value it is copying is a value.
