# 372 — a WITHHELD eForms field is projected as if it were data: `-1.00` becomes an amount and `unpublished` becomes a currency

Status: ready-for-agent — **UNIT 2 BUILT, DEPLOYED AND VERIFIED ON PROD 2026-09-09**
(`4633443` amounts, `796473b` bids, `436f73e` read layer + `/v1`, `d7bb7db` the report's
`marked` column, **`ce6d2c2` the migration without which the whole thing was INERT**).
**UNIT 4 ALSO BUILT, DEPLOYED AND VERIFIED (`4c7ad40`)** — the statistics satellite; the
award criteria turned out to need nothing. Live on `4c7ad40`; see "Unit 2 VERIFIED ON PROD"
and "Unit 4" below, both with negative controls. Remaining: unit 3 (`currency =
'unpublished'`, shaped by the `NOT NULL` constraint) and the standing-row re-fold, still the
open owner decision shared with 366. See "Unit 2 BUILT". Remaining: unit 3 (`currency =
'unpublished'`, and the column is `NOT NULL` — see the constraint finding), unit 4 (the other
satellites, whose channel map the fixture probe now gives), and the standing-row re-fold, still
the open owner decision shared with 366. Earlier: **unit 1 CORPUS-WIDE 2026-09-08 (job 818): 19,236 `-1.00` rows, residue
**116** (0.6 %); `result_value` 99.57 % declared, but `estimated_value` **0 of 29** — a second cause
wearing the same value, split out as unit 5, now DONE — publisher-invented sentinels with no withholding block, already handled by 366's negative rule and NOT to be labelled withheld. **Unit 2 DECIDED: option (b), a quality marker applied
PER ROW conditioned on the notice's declaration, never as a blanket rule on the number — and it must cover
BOTH `tender_version_amounts` AND `tender_version_bids`, see "SCOPE CORRECTION".** Earlier: unit 1 sampled 2026-09-08 (see "Unit 1 — the census"): 8/8 `-1.00`
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


## Unit 2 BUILT (2026-09-09) — four commits, gate green each time, none deployed yet

Option (b) as decided: the fact is emitted with a marker, per row, conditioned on the notice's
own declaration. Nothing keys on the number `-1`.

| commit | what |
| --- | --- |
| `4633443` | `Fact::Amount` + `tender_version_amounts` gain `quality`; the declaration lookup; the head election refuses a marked fact |
| `796473b` | the bids satellite: `BidState` + `tender_version_bids` gain `quality`, `read_results` marks a bid whose LotTender declared BT-720 |
| `436f73e` | `/v1` stops publishing the placeholder as a value; the lot headline value stops picking one |
| `d7bb7db` | section 11 counts the rows actually marked, so the anchoring gap is measurable |

### The mechanism, verified against real XML rather than read off the schema

`withheld_source_fields(parsed)` returns the notice's declarations as `(section, source field)`
pairs — the FieldsPrivacy block's **parent** section paired with the id in the declaration's
parentheses. The test at emission is then one set lookup on `(value.section_id,
stem(value.field_id))`, needing **no new vocabulary**: the correspondence is the identity.

That pairing was the fix's load-bearing assumption, so it is asserted against the committed
withheld fixture (`withheld_declarations_pair_with_the_section_holding_the_suppressed_value`).
All five of its blocks anchor exactly that way. The probe output is worth keeping:

| block's parent | declaration | the suppressed value in that section |
| --- | --- | --- |
| `ND-ReceivedSubmissions#0` | `BT-195(BT-759)` = `rec-sub-cou` | `BT-759-LotResult` = `Number(-1.0)` |
| `ND-ReceivedSubmissions#0` | `BT-195(BT-760)` = `rec-sub-typ` | `BT-760-LotResult` = `Code('unpublished')` |
| `ND-LotAwardCriterion#0` | `BT-195(BT-541)` = `awa-cri-num` | `BT-541-Lot-WeightNumber` = `Number(-1.0)` |
| `ND-LotAwardCriterion#0` | `BT-195(BT-734)` = `awa-cri-nam` | `BT-734-Lot` = **`Text('unpublished')`** |
| `ND-LotAwardCriterion#0` | `BT-195(BT-539)` = `awa-cri-typ` | `BT-539-Lot` = `Code('unpublished')` |

**This is unit 4's map, obtained for free.** The marker lands on three value channels — a
number, a code and a TEXT literally reading `unpublished` — and the same lookup serves all of
them, because the declaration names a field and does not care what channel the field uses. Unit
4 is therefore an application of unit 2's mechanism, not a new investigation.

Also note `BT-5421-Lot` = `per-exa` sitting in `ND-LotAwardCriterion#0` **undeclared**. That is
the per-row precision made concrete: the weight NUMBER is withheld while the weight TYPE beside
it in the same section is published, and a section-wide or value-shaped rule would have marked
both. `a_declaration_marks_only_the_field_it_names` pins it.

### The blind spot, stated and then measured rather than left as a caveat

The rule is EXACT on the section. A block a publisher hoisted away from the value it suppresses
— issue 195 saw `FieldsPrivacy` written under the root extension, 4× on sdk-1.9 — marks nothing.

That direction is the safe one: an unmarked withheld row behaves exactly as it does today,
refused by 366's negative-sentinel rule. But "safe" is not "known", so section 11 now carries
`marked` beside the notice-wide `in-wh-notice`, and **`in-wh-notice` minus `marked` is the
anchoring gap**. `a_hoisted_privacy_block_marks_nothing_rather_than_guessing` pins the current
behaviour so that widening the rule later has to change a test that says why.

Read that column on recently folded rows only. Standing rows carry no marker until re-folded, so
a low corpus-wide `marked` would say nothing about the rule — the report's own prose says so.

### `/v1`: suppression, not a flag beside the old number — a decision worth naming

A withheld amount or bid now reports `value: null` with `quality: "withheld"`.

The additive alternative (keep `value: {cents: -100}`, add a flag) was rejected: a consumer that
never heard of `quality` would **still** be told the value is −0.01, and this issue is precisely
about a layer copying a placeholder as data. Under the shipped shape a reader who ignores the key
sees "no value", which is true. The field or bid is still named either way, so *which* value is
missing stays identifiable — the distinction option (b) exists for. This does change output for
the affected rows once they are re-folded; that is the correction, not a regression.

Second find in the same pass: **a lot's headline value could be a withheld figure.** In
`read::summarise`'s pick, `-100` outranks the `i64::MIN` default, so a lot whose ONLY amount was
withheld showed −0.01 as its value — winning by being the only row rather than by being a
figure. Now excluded.

### The head election refuses a marked fact for the RIGHT reason

`head_value_eur_cents` gained `quality.is_none()` beside the existing `!sentinel_amount(cents)`.
This changes **no election today** — `sentinel_amount` already refuses −1 — and it is recorded
here so that is not mistaken for redundancy: it is the arm that states the reason, and it starts
mattering the moment a publisher withholds a field without writing −1 into it.

### One risk checked before touching the shape

`Fact` is `postcard`-serialized into the Phase-2 bucketed fold's spill files, so widening
`Fact::Amount` changes that wire format. Safe: `project.rs:2279` does `remove_dir_all` on the
bucket directory at the START of every fold, so no bucket outlives a run. Checked rather than
assumed, because a stale bucket read with a new codec is a silent corruption rather than an
error. `bucket_row_survives_the_postcard_codec` now carries a marked amount as well as an
unmarked one, so the codec is tested on both readings.

### Unit 3 has a constraint the earlier write-up did not record

`tender_version_amounts.currency` is `TEXT NOT NULL`. So "never store `unpublished` as a
currency" cannot be done by NULLing the column — the options are a placeholder code, dropping the
row (which unit 2 rejected for the amount itself), or relaxing the column. `/v1` no longer
*publishes* the currency of a withheld row, since the whole value object is now `null`, so the
user-visible half of unit 3 is already addressed for declared rows; the 141 stored rows are what
remains, and the `NOT NULL` is the thing that shapes the choice.


## The fix shipped INERT, and the gate could not have told me (2026-09-09)

Between `d7bb7db` and `ce6d2c2` this was three commits of nothing. `CREATE TABLE IF NOT
EXISTS` never evolves an existing table, so `quality` reached only databases created after
it — and **every test in the workspace builds its database fresh**, where the CREATE TABLE
carries the column. So 113 suites went green while the box answered:

```
{"error":{"message":"Parse error: no such column: quality","status":400}}
```

Worse than inert: `436f73e` deployed read paths that SELECT `quality`, so `/v1` tender detail
and the bids listing were erroring against the live database until `ce6d2c2` landed.

The mechanism was already there — `MIGRATIONS` in `store/src/lib.rs`, with a doc comment
stating this exact hazard, and `tax_basis` on this very table as precedent. I did not use it.

**Found by probing prod, not by the suite.** The probe was only being run to answer a
different question (does the marker fire on real notices?); the first query returned the
error above. Had I trusted the green gate and moved on, this would have sat broken.

Guarded now by `store/tests/satellite_column_migration.rs`, which does the one thing the rest
of the suite structurally cannot: pre-creates both satellites in their PRE-column shape
through a raw connection — the prod shape — then opens the database the way the binary does,
leaving the ALTER as the only route. **Verified against its own negative**: with the two
migrations removed it fails at the `SELECT quality` probe (exit 101). A test that passed
either way would have been worth nothing here.

**Audited the rest rather than assuming it was one slip.** All 17 canonical tables probed on
prod with their full declared column lists (`SELECT <every column> FROM <table> LIMIT 0`);
`quality` on these two satellites was the only drift. An isolated miss, not a pattern.

## Unit 2 VERIFIED ON PROD (2026-09-09, rev `ce6d2c2`)

Re-folded the four notices unit 1's census found declaring `BT-195(BT-161)` (jobs 825/826,
both ok, 4 tenders), then read the satellites back.

**Amounts — 4 of 4 marked.** Before the re-fold all four were `null` (standing rows, correct);
after, all four read `withheld`:

| tender | seq | field | cents | quality |
| --- | --- | --- | --- | --- |
| 34 | 4 | `result_value` | −100 | **withheld** |
| 189 | 4 | `result_value` | −100 | **withheld** |
| 254 | 4 | `result_value` | −100 | **withheld** |
| 288 | 2 | `result_value` | −100 | **withheld** |

**Bids — the count matched the prediction exactly.** Notice 25390373 declared
`BT-195(BT-720)-Tender` **three times** (`#0/#1/#2`, one per winning tender), and tender 34
seq 4 came back with **exactly three** marked bids — 151, 152, 153 — while bids 148/149/150 in
the same version, which carry no BT-720, stayed `null`. Tender 288 has one declaration and one
marked bid (636). 14 unmarked bid rows alongside the 4 marked ones.

That is the per-bid granularity the scope correction argued for, confirmed on real data rather
than on a fixture: an amounts-only fix would have left all four of these asserting a −0.01 offer.

### The negative control, which is the one that mattered

The whole design rests on keying the marker to the DECLARATION and not to the number. So the
test that could have falsified it: re-fold unit 5's undeclared class — notices 24158422 and
25734164, established there as publishing −1 with **zero** `FieldsPrivacy` blocks — and require
the marker to stay absent (jobs 827/828, both ok).

**16 rows, every one `cents = -100`, every one `quality = null`.** Same value as the four marked
rows above, no declaration, not marked. If the rule had quietly degenerated into "−100 means
withheld" — the tempting shortcut this issue exists to warn against — these 16 rows are where it
would have shown, and they are clean.

### No anchoring loss in this sample

The section-anchored rule's known blind spot (a `FieldsPrivacy` block hoisted away from the
value it suppresses, issue 195) cost nothing here: 4 of 4 amounts and 4 of 4 predicted bids were
reached. One sample of four TED notices is not a corpus rate — section 11's `marked` column is
what will give that on the next weekly run.


## Unit 4 DONE (2026-09-09, `4c7ad40`) — one surface to fix, three that never needed it

### The award criteria are a non-issue, and that is worth stating

The fixture's other markers — `BT-539` (criterion type, `Code('unpublished')`), `BT-541`
(weight, `Number(-1)`) and `BT-734` (name, **`Text('unpublished')`**) — looked like three more
surfaces. They are not: **those field ids reach no canonical satellite at all.** Verified by
grep on 2026-09-09 — they appear in no field map (`TEXTS`/`AMOUNTS`/`CLASSIFICATIONS`/`DATES`)
and no routing arm in `read_results`; the only hits in `project.rs` are my own unit-2 tests and
doc comments. So their placeholders stay in the parsed layer, which is exactly where ADR-0004
says they belong.

Recorded because the fixture makes them *look* like work, and "checked, nothing to do" is a
different claim from "not looked at".

### The statistics satellite, which did need it

`BT-759` (received-submission count) and `BT-760` (its type) both route to
`tender_version_result_stats(kind, count)`. Withheld, the SDK writes `-1` into the count and
`unpublished` into the code — so an unmarked row asserts **that −1 submissions of a type called
`unpublished` were received**. Junk on BOTH halves, which is why the marker sits on the row and
why EITHER declaration marks it.

Denser than the amount case — one CAN carries a statistics block per lot result:

| probe | result |
| --- | --- |
| `tender_id <= 100000`, `kind='unpublished' OR count<0` | **2,752 rows / 164 tenders** (~17 each) |
| of those, `kind='unpublished'` | 2,640 |
| of those, `count < 0` | 2,708 |
| `tender_id <= 20000`, in a withholding notice | **92 of 93 (98.9 %)** |

98.9 % declared matches the amount side's 99.57 %, so it is the same mechanism and equally
trustworthy. The unbounded form of the first probe hit the 10 s cap and was **not retried**;
section 11's new third arm produces the corpus total.

### Verified on prod, including the case the design decision was actually about

Re-folded notices 24173766 and 24089978 (jobs 829/830, ok). Both rows read `null` before:

| tender | kind | count | quality after | what it proves |
| --- | --- | --- | --- | --- |
| 1623 (×6) | `unpublished` | −1 | **withheld** | both halves suppressed |
| 3711 (×2) | **`t-esubm`** | −1 | **withheld** | the TYPE was published and only the COUNT withheld |

3711 is the one that mattered. `t-esubm` is a real submission type, so BT-760 was published
while BT-759 was withheld — the "either declaration alone marks the row" rule, confirmed on real
data rather than on my synthetic test. A rule keyed on `kind = 'unpublished'` would have missed
these entirely.

**Negative control (job 831/832):** re-folded notice 23828920 → tender 1606's five statistics
rows, every one a real kind with a real count, **all `quality = null`**. Two of them are genuine
**zeros** (`t-no-eea` 0, `t-oth-eea` 0) and stayed unmarked — which is the distinction that
matters here: 0 means "none of that type were received" and is a reading; −1 means "we are not
telling you" and is not.

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

### The `result_value` / `framework_maximum` residue: same class, and one claim I could NOT substantiate

Triaged the 83 + 4 rows left unclaimed above. A windowed probe (`tender_id` 0–500,000, `cents = -100`,
those two fields, `NOT EXISTS` a `FieldsPrivacy` section) returned its full `LIMIT 20` — they are spread
across the id space rather than clustered. Four sampled by era:

| tender/seq | profile | source |
| --- | --- | --- |
| 14243 / 9 | `eforms:eforms-de-2.1` | doe |
| 62892 / 7 | `eforms:eforms-de-2.0` | doe |
| 134115 / 1 (`framework_maximum`) | `eforms:eforms-de-1.1` | doe |
| 144695 / 1 | `eforms:eforms-sdk-1.8` | ted |

**Same class as unit 5's `estimated_value` rows:** eForms notices with NO `FieldsPrivacy` block at all,
publishing −1 directly. So the disposition is the same — already refused by issue 366's negative rule,
and NOT to be labelled `withheld`. Unit 2's scoping is unchanged by this.

**What I could not establish: the DÖE skew.** Three of four sampled are `source = doe`, which would make
this a national-portal convention rather than scattered publisher error — a genuinely useful thing to
know, and 3-of-4 is not evidence of it. The bounded route cannot settle it: a 2,000,000-wide `tender_id`
window carrying the version join, the notices join and the `EXISTS`, grouped by source, **hit the 10 s
cap** (2026-09-08, not retried per `prod-box-reads.md`); the three windows that did answer held 2 rows
between them, so narrowing further just fragments the count.

**So it moved into the instrument instead of staying a guess:** section 11 now groups by
`field · source`. In-process the joins are already paid for, so the split is free, and the next weekly
run answers the question rather than leaving a suggestive sample on the board. Recorded because the
temptation was to write "predominantly DÖE" from four rows.

## SCOPE CORRECTION (2026-09-08): the marker has a SECOND surface — `tender_version_bids`

Found while starting unit 2, by tracing where the census's other withheld code actually lands. The two
BT-195 codes the notices declare do **not** go to the same table:

| withheld source field | code | canonical destination |
| --- | --- | --- |
| `BT-161` | `not-val` | `AMOUNTS` → **`result_value`** in `tender_version_amounts` |
| **`BT-720`** | `win-ten-val` | **NOT in `AMOUNTS`** — routed at `project.rs:3875` into `raw.bids`, so it becomes a BID's own `cents` in **`tender_version_bids`** |

`BT-720` is the winning tender value. It reaches the canonical layer through the issue-177 context
routing, not through the amount table, so **section 11 has never seen it** — its query is
`tender_version_amounts` only.

**Measured, partially:** windowed probes of `tender_version_bids WHERE cents = -100` returned **184
rows across 136 tenders** in three of four `tender_id` windows. The densest window (0–2,000,000) **hit
the 10 s cap and was not retried**, so the true count is higher than 184 — this is a floor, not a
total.

### Why this matters for unit 2, not just unit 4

Unit 2 as scoped ("mark `withheld` where a declaration names the field the fact came from") would fix
`tender_version_amounts` and **leave every one of these bid rows asserting a −0.01 bid** — the same
defect, one satellite over, and now a known one rather than a suspected one. The census's own evidence
pointed here all along: notice 25390373 declared `BT-195(BT-720)-Tender` = `win-ten-val` **three times**
(`#0/#1/#2`, one per winning tender), which is exactly the shape a per-bid withholding takes.

So unit 2 covers BOTH satellites, or it is not the fix. The declaration lookup is the same mechanism —
the BT-195 field id names the source field either way; only the destination table differs, and that
difference is already encoded in the projection's own routing.

### And the count goes into the instrument, as before

The bounded route cannot total this (the dense window times out), so section 11 should carry a second
query over `tender_version_bids` rather than leaving "≥184" on the board. Same reasoning as the
`field · source` split one entry above: in-process the scan is affordable, and the next weekly run
replaces a floor with a number.

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
