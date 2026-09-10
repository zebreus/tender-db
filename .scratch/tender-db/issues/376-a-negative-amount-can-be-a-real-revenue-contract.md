# 376 — a negative amount is not always junk: revenue-side contracts publish one, and we drop the value entirely

Status: ready-for-agent — UNIT 1 DONE 2026-09-10 (both stated reasons corrected: 366 Leg A now
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
