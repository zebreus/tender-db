# 372 — a WITHHELD eForms field is projected as if it were data: `-1.00` becomes an amount and `unpublished` becomes a currency

Status: ready-for-agent — **unit 1 CORPUS-WIDE 2026-09-08 (job 818): 19,236 `-1.00` rows, residue
**116** (0.6 %); `result_value` 99.57 % declared, but `estimated_value` **0 of 29** — a second cause
wearing the same value, split out as unit 5, now DONE — publisher-invented sentinels with no withholding block, already handled by 366's negative rule and NOT to be labelled withheld. **Unit 2 DECIDED: option (b), a quality marker applied
PER ROW conditioned on the notice's declaration, never as a blanket rule on the number.** Earlier: unit 1 sampled 2026-09-08 (see "Unit 1 — the census"): 8/8 `-1.00`
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

## Unit 1 corpus-wide (job 818, 2026-09-08) — and unit 2's disposition, DECIDED

Section 11 of the weekly report, first run (job 818: ok, 4,573 s, 0 unmeasured):

| field | rows | in a withholding notice | **residue** | tenders |
| --- | --- | --- | --- | --- |
| `result_value` | 19,201 | 19,118 | **83** | 18,097 |
| `estimated_value` | 29 | **0** | **29** | 6 |
| `framework_maximum` | 6 | 2 | **4** | 2 |

**19,236 rows, residue 116 — 0.6 %.** The marker is real and it is trustworthy: 99.57 % of the
dominant class (`result_value`) sits in a notice that declared a withholding. The sampled 4-of-4 was
not luck.

### The per-field split found a SECOND cause wearing the same value

`estimated_value` is **0 for 29**. Not one of those notices declared a withholding — so those −1s are
**not the eForms withheld marker at all**, whatever they are. `result_value` is 99.6 % declared and
`estimated_value` is 0 % declared: two different phenomena that an aggregate over `cents = -100` would
have averaged into one 99.4 % and hidden completely. Grouping by field was worth doing.

6 tenders carry those 29 rows, so the class is hand-readable. **It is not part of unit 2** and gets its
own unit below rather than being swept into the withheld disposition — marking them `withheld` would
assert something the notice never said, which is the error this whole issue is about, one layer along.

### Unit 2 DECIDED (owner, 2026-09-08): option (b), per-row, conditioned on the declaration

Emit the fact **with a quality marker**, not suppressed and not NULL:

- **(a) don't emit** loses the distinction between "withheld" and "never published", which the source
  takes care to state — and ADR-0013 D5 already models it one layer down, so throwing it away here
  would be discarding information the notice explicitly carries.
- **(c) NULL** is the same loss with an extra ambiguity: NULL already means "no rate resolved" on the
  sibling `eur_cents` column (ADR-0014 D4).
- **(b) marker** keeps the parse layer faithful (ADR-0004: store as published) while making the
  canonical layer stop *asserting* a €−0.01 contract. It is also the same `quality TEXT` column issue
  366's Leg A proposed for sentinels — arriving here for a second, independent reason, which is decent
  evidence the column is the right shape rather than a convenience.

**The condition is per row, not per value.** A `-1.00` whose notice declares a `FieldsPrivacy`
withholding for the field the fact came from is marked `withheld`; one that does not is NOT marked
withheld — it stays whatever the residue investigation concludes. A blanket rule on the number −1
would re-commit this issue's own mistake: treating a value as self-describing when the notice beside
it says what it means.

The mechanics are already available: the BT-195 code names the SOURCE field, and the projection's
`AMOUNTS` mapping (`BT-161` → `result_value`, `project.rs:135`) already translates that to the
canonical name — so the exact per-row test is a lookup, not new vocabulary.

## Unit 5 (new) — the undeclared residue, 116 rows

Split by field because they are plainly not one thing:

- **`estimated_value`, 29 rows / 6 tenders, 0 declared.** The interesting half. Read all six by hand:
  is this a different portal's "not stated" convention, a unit error, or a genuine negative estimate?
- **`result_value`, 83 rows / (subset of 18,097 tenders), undeclared.** Most likely notices that
  withheld the value without emitting the `FieldsPrivacy` block, or where the block names a different
  field. Sample ~10 through the same per-notice probe unit 1 used.
- **`framework_maximum`, 4 rows / 2 tenders.**

Until read, these stay OUT of the withheld disposition. Section 11 makes the residue a standing number,
so it is visible if it grows.

## Unit 5 DONE (2026-09-08): the `estimated_value` residue is a publisher-invented sentinel, not a marker

**Located, bounded.** 16 windowed probes over `tender_id` (500,000 wide, `cents = -100 AND field =
'estimated_value'`, indexed range + filter) — all 16 answered, none shed, and they returned **exactly
29 rows**, matching section 11's count. Six tenders: **525230** (4 rows), **555999** (1), **631695**
(3), **858517** (12), **1081472** (2), **1094340** (7). Every row EUR, every `eur_cents` also −100.

**They are eForms, not a legacy convention** — which was the hypothesis worth killing first. 858517's
versions are `eforms:eforms-sdk-1.7`; 525230's are `eforms:eforms-sdk-1.12` and `eforms:eforms-de-2.0`.

**And they publish −1 with no withholding block at all.** Notice 24158422 (858517 seq 2):

| check | result |
| --- | --- |
| `FieldsPrivacy` sections | **0** |
| published amounts | `BT-27-Lot` 0, `BT-27-Lot` 0, `BT-27-Lot` 0, **`BT-27-Procedure` −100** |

A buyer writing **−1 into BT-27 directly** as "not stated", while its own lots publish 0, without using
the mechanism the format provides for exactly that. So the `estimated_value` −1 is a publisher-invented
sentinel that merely *resembles* the SDK marker.

### Disposition: nothing new is needed, and they must NOT be labelled `withheld`

- **Issue 366's negative-sentinel rule already handles them correctly.** The head election refuses ALL
  negatives regardless of declaration, so these 29 rows are already excluded from
  `current_value_eur_cents` and from the value bounds that read it. No new mechanism.
- **They must not be marked `withheld`**, because the notice never says so. That is precisely the
  distinction unit 2's decision was framed around — a blanket rule on the number −1 would treat the
  value as self-describing when the notice beside it is the thing that gives it meaning. Here the
  notice says *nothing*, so the honest reading is "an unexplained negative", not "a withheld value".

**This completes unit 2's scoping**: mark `withheld` only where a `FieldsPrivacy` declaration names the
field the fact came from; everything else stays a 366-refused negative. The `result_value` residue (83
rows) and `framework_maximum` (4) are untriaged and are the same shape as this class — likely the same
answer — but they were not read, so they are not claimed.

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
