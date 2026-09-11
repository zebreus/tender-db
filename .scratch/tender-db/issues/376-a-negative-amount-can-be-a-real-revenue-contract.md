# 376 — a negative amount is not always junk: revenue-side contracts publish one, and we drop the value entirely

Status: ready-for-agent — UNITS 1 AND 2 DONE 2026-09-10 (unit 2: no modelled direction, because the
field could not be populated honestly — see the decision at the end, with the three things that reopen
it). Units 3 (the 17 sub-€10k negatives, needs a gated archive read) and 4 remain. Unit 1 DONE (both stated reasons corrected: 366 Leg A now
rests on the column's domain rather than on "no procurement has a negative value", and 372's unit 5
heading is scoped to the 29 rows at −100 it actually measured). Units 2, 3, 4 remain.
Was: needs-triage (filed 2026-09-10 by the owner while hand-reading issue 372's residue; the
finding is a MEANING correction, not a code defect, and it makes two other issues' stated reasons
wrong even though their behaviour may be right)
Kind: data-model gap (the canonical layer has no way to say "money flows TO the buyer"), plus two
stale justifications on 366 and 372
Relates to: 366 (Leg A refuses every negative on the reasoning "No procurement has a negative value" —
that sentence is false), 372 (whose unit 5 calls the whole residue "publisher-invented sentinels"),
ADR-0004 (store as published — the published figure survives in `amounts`, which is why this is
recoverable)

## Observed

Issue 372's standing-row drain left **118 negative amount rows with no withheld declaration**, over 71
tenders. Unit 5 had characterised that residue as publisher-invented sentinels. Reading all 118 rather
than sampling them says otherwise:

| | rows |
| --- | --- |
| exactly **−1.00** | **43** |
| other magnitudes (**57 distinct values**) | **75** |

By field: `result_value` 59, `estimated_value` 35, `framework_maximum` 24.

**The 75 are not sentinels of any shape.** Sorted by magnitude, with titles:

| cents | ccy | title |
| --- | --- | --- |
| −15,115,926,500 | NOK | Main bank agreement for Tromsø, Karlsøy, Balsfjord … |
| −2,834,607,600 | DKK | Offentligt udbud vedr. Behandling og **afsætning** … |
| −2,107,154,720 | DKK | Afhentning, sortering/behandling og **afsætning** … |
| −2,000,000,000 | DKK | Udbud af **bankforretninger** |
| −1,756,800,000 | DKK | Kontrakt om **afsætning** af neddelt umalet inde… |
| −1,180,000,000 | EUR | Levering **restafval** voor saturatie AEC |
| −600,000,000 | DKK | Daglige **bankforretninger** |
| −318,764,800 | EUR | ehem. Bayernkaserne, **Grundstücksvermietung** |
| −104,287,500 | EUR | **Verwertung von Altpapier** 2026/2027 |
| −76,110,800 | EUR | De ophaling en **recyclage van oude metalen** |

Waste sold for processing, scrap metal, waste paper, residual waste delivered to an incinerator,
land leasing, and bank agreements. **These are revenue-side contracts: the supplier pays the
authority, not the other way round.** The minus sign is the publisher saying so, in a field with no
sign convention.

**They are ordinary award notices, so nothing declares it.** The six largest are all
`eforms:eforms-sdk-1.7/1.10/1.13`, notice subtypes **29 and 30** — standard contract-award notices,
not a concession subtype. There is no schema-level marker to key off; the sign and the subject matter
are the only signal.

**And the value disappears.** Of those six:

| tender | amount rows | positive rows | served head value |
| --- | --- | --- | --- |
| 519784 (Tromsø bank) | 1 | 0 | **NULL** |
| 484080 (waste treatment) | 1 | 0 | **NULL** |
| 880231 (bank business) | 3 | 0 | 0 |
| 360984 / 796248 / 1049912 | 3–4 | 2–3 | the positive sibling |

So a substantial multi-year bank agreement serves `value: null` and appears in neither `min_value` nor
`max_value`. The published figure is still in the `amounts` array (ADR-0004 doing its job), which is
the only reason this is recoverable rather than lost.

## Why this matters more than 118 rows

**Issue 366 Leg A's stated reason is false.** It refuses every negative amount on the grounds that
*"No procurement has a negative value. −1.00 alone is 15,529 rows and is a documented publisher
convention for 'not stated'."* The first sentence is wrong: a concession or revenue contract has one,
and the corpus contains them. 366 has already had this leg's REASON corrected once (the −1 turned out
to be the SDK's withheld marker, not a convention); this is the second time the disposition survived
while the justification under it did not.

**The disposition is probably still right, and that is the point.** Mixing +€100 M of spend and
−€150 M of revenue in one ordering is meaningless, so excluding them from `current_value_eur_cents` is
defensible. But it should be excluded *because the column means "what the buyer pays"* and these are
not that — not because "no procurement has a negative value". The difference decides what to build
next: a wrong-value rule wants a filter, a wrong-SIGN-domain rule wants a field.

## Units

1. **Correct the two stated reasons** (366 Leg A, 372 unit 5) so neither claims these are junk. Cheap,
   and it stops the third re-derivation of a justification that has now failed twice.
2. **Decide whether the head column should say "expenditure".** Options, none costed:
   - Leave as is: revenue contracts have no head value, disclosed in `amounts` only. Honest, lossy.
   - Elect `abs()` for them — **no**, it would sort a €150 M revenue contract beside a €150 M spend.
   - Add a sign/direction to the canonical amount so the value ordering can be scoped, and the
     bounds can say which side they mean. The real fix, and the expensive one.
3. **The <€10 k negatives are still unexplained** — 17 rows, on ordinary service contracts
   (`SERVICIOS DE MANTENIMIENTO INTEGRAL DE EQUIPOS` at −€3,842.76, `Architekten- und
   Ingenieurleistungen` at −€5,047.17). Those do not fit the revenue reading and may be genuine sign
   errors or corrections. Several are the same Spanish buyer, so it is a repeated publisher behaviour
   rather than a one-off. Needs the archive-member read (gated) that 366 unit 5 also needs.
4. **The −1.00 half (43 rows) stays where 372 put it**: undeclared withheld markers. Nothing here
   changes that reading.

## Done when

- no issue's prose claims a negative published amount is necessarily junk;
- the revenue class has either a modelled direction or a written decision that it does not get one;
- the <€10 k class is read and dispositioned.

*One issue because:* the 118 rows split into three causes wearing one sign, and the reason 366 and 372
both mis-read them is the same — the sign was treated as a defect signal rather than as a datum.

*Not done here:* no archive member was read. Every claim above is from the canonical layer and the
notice metadata, so "the publisher meant revenue" is a well-supported reading of the subject matter,
not a confirmed reading of the source XML.

## Unit 2 DECIDED (2026-09-10, owner): no modelled direction — option (a), and the reason is populability

The issue framed this as "is direction worth modelling". That is the wrong question, and asking it that
way is what makes option (c) look like the principled choice. The right question is **whether a
`direction` field could be populated honestly**, and the evidence already on this issue says no.

**The sign is the only signal, and publishers are not required to use it.** The six largest revenue
contracts are ordinary award notices — eForms subtypes 29 and 30, no concession subtype, no schema-level
marker. So the only thing separating a revenue contract from a spend contract is that *this* publisher
chose to write a minus. Nothing obliges the next one to. A Danish authority publishing a bank agreement
as `2000000000` positive is indistinguishable, in this corpus, from a Danish authority spending the same
sum.

**So a field populated from the sign would be systematically incomplete in an unmeasurable way.** Every
row would get `direction`, but only ~75 would get it from evidence; the rest would get `expenditure` from
a default. A consumer reading `direction: expenditure` cannot tell those apart, and would reasonably read
it as a published fact. **That is worse than having no field**: an absent column is honestly absent,
while a defaulted one manufactures a fact out of a publisher's silence. It is the same failure as the
head column electing a MAX over unflagged facts (issue 366) — a derived value that looks like a
measurement.

**And the population is unmeasured in the direction that matters.** We know 75 rows carry a negative and
read as revenue. We have no idea how many revenue contracts carry a POSITIVE figure, and no instrument
that could find them: no marker, no subtype, no field. Sizing the class the field would serve is not
merely undone, it is not currently possible. Building a model of a population you cannot size is how you
get a rule calibrated on one era (this issue's own sibling failure — 364's `Procedure-Buyer`-only
calibration).

**What is done instead, and it is not nothing:**

- The published figure is already served. `/v1/tenders/{id}` serialises every amount fact verbatim,
  sign included (`crates/app/src/v1/json.rs`, `detail`), so a consumer who cares can see it. ADR-0004
  earns its keep here.
- The API caveats now SAY this rather than calling every negative a withheld marker, which is what they
  said until today. Two places were wrong in the user-facing direction: the amounts caveat called
  `-1.00` a "publisher sentinel" (it is the SDK's withheld marker, issue 372) and said nothing else, and
  the `min_value`/`max_value` caveat glossed the whole negative class as "the SDK's withheld marker,
  ~15,500 rows". A reader following those would have discarded the Tromsø bank agreement as junk.
- `current_value_eur_cents` keeps excluding negatives, on 366 Leg A's corrected reason: the column's
  domain is **what the buyer pays**, and a revenue contract is not that. That is a statement about the
  column, not about the row.

**What would reopen this.** Any of these, and the third is the one to watch:

1. A source that publishes a direction, a concession/revenue subtype, or a signed-amount convention we
   could read as a fact rather than infer. The GB FTS arm (issue 342) is the nearest candidate corpus
   with a different schema.
2. Evidence that the negative sign is *reliable* — e.g. an archive read showing publishers who use it do
   so consistently across their notices. That would not fix the positive-published class, but it would
   turn the 75 from a reading into a fact.
3. **The class growing.** 75 rows is a rounding error against 267M money rows; the decision is partly a
   size judgement, and size judgements expire. The weekly report's section 11 already carries the
   undeclared-negative residue as a standing number, so growth is visible without anyone remembering to
   look.

*Not decided here:* unit 3's 17 sub-€10k negatives on ordinary service contracts. Those do not fit the
revenue reading and are still unexplained; they need the gated archive-member read that 366 unit 5 also
wants, and they may turn out to be sign errors rather than either class.

## Unit 3 ANSWERED (2026-09-11) — two publishers, not seventeen mysteries

The 17 sub-€10k negatives were re-found windowed (80 windows, **0 failed**, exactly 17 rows, all EUR:
13 `result_value`, 3 `framework_maximum`, 1 `estimated_value`). Then two structural questions were
asked of them, and both answered without the archive read this unit was waiting on.

**Does the negative sit beside a positive?** For **12 of 16 tenders it is the ONLY amount on the head
version.** For 4 it sits beside positives — and there it is plainly not a value:

| tender | negative | largest positive | buyer |
| --- | --- | --- | --- |
| 736357 | **−279.81** | 83,558,193.75 | Fira 2000, S.A. (ES) |
| 728550 | **−3.00** | 1,167,000.00 | Klinikum Freising GmbH (DE) |
| 836867 | **−3.00** | 319,297.22 | Centrale di Committenza … Fontanafredda (IT) |
| 411760 | −418,857.60 | 408,288.51 | IMOG (BE) — waste collection/processing |

**Who publishes the other 12?** Two buyers:

| family | tenders | buyer | subject |
| --- | --- | --- | --- |
| Spanish | **6** | **`Servicio Madrileño de Salud - Hospital Universitario La …`** | `SERVICIO(S) DE MANTENIMIENTO` — equipment maintenance |
| German | **4** | **`Studierendenwerk München Oberbayern`** | `Architekten- und Ingenieurleistungen` |

**All six Spanish rows are one buyer. All four German rows are one buyer.** The issue guessed
"several are the same Spanish buyer"; it is stronger than that — ten of the seventeen rows come from
exactly two publishers, each consistent with itself.

### Disposition

1. **The 10 are publisher SIGN ERRORS.** A hospital is not paid to have its equipment maintained and
   a student-services body is not paid to receive architectural drawings, so the revenue reading that
   explains this issue's main class does not apply. The magnitudes (€1,588 to €8,385) are plausible
   small service contracts. ADR-0004 means the minus is the publisher's, not ours. They stay excluded
   from the head column for the reason unit 2 settled: the column means what the buyer PAYS.
2. **`−3.00` twice, from unrelated buyers in different countries, beside six- and seven-figure
   positives, is a SENTINEL and not a price.** Two is thin, but two identical implausible values from
   unrelated publishers is the shape section 10's sweep exists to catch. It is below that sweep's
   repeat threshold, which is why nothing has flagged it.
3. **411760 belongs to this issue's MAIN class, not unit 3.** IMOG is a Belgian waste
   intermunicipality and its −418,857.60 is revenue-side, exactly like the Tromsø bank agreement. It
   appeared here only because it ALSO carries a −6,671.28 row.

**No archive read was needed.** The unit assumed one because the question was framed as "what did the
publisher mean"; asked instead as "is there a positive beside it, and who published it", the canonical
layer answers. Worth remembering before gating the next question on an expensive instrument.

