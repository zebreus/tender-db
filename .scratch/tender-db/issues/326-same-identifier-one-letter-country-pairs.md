# 326 — Same identifier, country codes one letter apart: a typo class the review campaign is deciding by hand

Status: DONE 2026-09-01 — census re-cut by cluster, BG/LT/SK evidence arms added,
survivor rule built and tightened, 312 rows moved (`105c519`, job 550) and the
200 resulting merge groups folded (`8c8175f`, job 553). The residue is deliberate
and named below: 153 clusters still `nobody-asked` (DE 32 and ES 23 of those
permanently — German register numbers carry no checksum and a Spanish CIF has a
letter), 36 clusters still multi-country because their stranger codes carry no
evidence of a slip, and 102 duplicate identities inside the 629 groups R2's
consortium and legal-form guards deliberately refuse.
Kind: data-quality / correctness (organization layer)
Relates to: 314 (the campaign that surfaced it), 325 (the other rule-shaped
class in the same cohort — zero overlap), 319 (country normalization)

A Slovak IČO `41734602` appears twice: once under **SK**, once under **SG**.
Same digits, same normalized name. Slovakia and Singapore differ by one letter.

That is not a border case. It is a transcription slip, and the cohort is full of
them:

| pair | what it is |
| ---- | ---------- |
| SK / SO | Slovakia → Somalia |
| SK / SG | Slovakia → Singapore |
| SK / SR | Slovakia → Suriname |
| SK / SI | Slovakia → Slovenia |
| CZ / CR | Czechia → Costa Rica |
| BG / BF | Bulgaria → Burkina Faso |
| BG / BI, BG / BR, BG / BT | Bulgaria → Burundi, Brazil, Bhutan |
| LT / MT, LT / LV, LT / LU | Lithuania → Malta, Latvia, Luxembourg |
| NL / NO | Netherlands → Norway |

## The measurement

Computed locally against the stored 589-case same-name cross-border packet
(no prod read):

* **339 of 589 cases (57.6%)** have an **identical** identifier carried across
  two different countries.
* **76 of 589 (12.9%)** have such a pair whose country codes are **exactly one
  letter apart**.
* Of the 76, **18 had already been reviewed by agents in slice 1, and all 18
  came back `wrong-country`. Eighteen of eighteen.**

A predicate with an 18/18 agreement rate against the reviewers is not a
heuristic; it is the thing the reviewers are computing by hand, at two agents
per case.

## Zero overlap with issue 325

The other rule-shaped class in this cohort (325, the VAT-prefix sniffer minting
a country from any word) covers **40** cases. The intersection with these 76 is
**empty**. Together they are **116 of 589 — 19.7% of the cohort** decidable by
predicate rather than by judgement.

## Which side survives is the hard half

Flagging the pair is easy. Choosing the survivor is not, and two obvious
tie-breakers were measured and are **not** sufficient on their own:

**The checksum anchor decides only 9 of the 76.** For the rest the probe has no
arm for either country — SK, LT and BG have no scheme in `checksum_anchors` at
all — so the anchors that do fire land on CZ/SI/PT/GR by shared arithmetic and
say nothing. This is exactly the vocabulary gap issue 314's `country_probed`
now exposes: a Slovak IČO anchoring `["CZ","SI"]` is not evidence for Czechia,
it is evidence that nobody asked about Slovakia.

**The mention spread decides most but not all, and it can point the wrong way.**
Heavy/light ratio: median 6×, p25 2×, p75 21×. 43 of 76 have a ≥5× spread, but
15 of 76 sit within 2×. And the pair census contains `BT (heavy) vs BG (light)`
three times — Bhutan outweighing Bulgaria — so "the heavier side is the real
one" is false as stated. A rule built on it alone would confidently invert those
three.

The workable discriminator is likely *format plausibility*: an 8-digit IČO is a
Slovak or Czech shape and not a Somali one, and the register-format tables the
matcher already carries know that. That is design work, not a measurement.

## Why this is worth building

The campaign's own numbers argue for it. Of slice 1's 62 `wrong-country`
verdicts, only **3** rested on the checksum evidence the packet was built to
carry. The rest were decided on things the packet reports but does not reason
about: an identical identifier across a one-letter country pair, a recognizable
national format (`A82473349` is a Spanish CIF, so the NL row is the wrong one),
a positive anchor on the *sibling* row (`5569584120` validates as a Swedish
orgnr, so the NO row is wrong), or an issue-325 parse artefact.

So the agents are doing pattern work a predicate can do, and the anchor
machinery — the expensive part — is carrying 3 of 62 decisions.

## Proposed shape

1. A `same-identifier-country-typo` census: identical identifier, distinct
   countries, edit distance 1 on the code. Report the pair, both mention counts,
   both anchor sets, and whether either country's register format admits the
   value's shape.
2. A survivor rule that fires only when the evidence is decisive (format admits
   exactly one of the two), and otherwise **abstains** and leaves the case in the
   review queue. An abstention is a correct answer here; a coin-flip is not.
3. Exclude the decisive subset from the remaining review slices, exactly as
   325's 40 should be excluded — the point of the campaign is the residue that
   needs judgement.

## What is NOT claimed

That all 76 are typos. `LT / LV` (8 cases) is a real neighbour pair where both
countries plausibly hold the same registrant, and `SK / SI` likewise. The 18/18
review agreement is a strong prior, not a proof, and it is drawn from the 18 that
happened to fall in slice 1 rather than from a random draw of the 76. Step 1 is
the census; the survivor rule is only as good as what the census shows.

---

## Built and measured corpus-wide (2026-08-31, `57d5224`)

`country-typo-census` job, deployed and run on prod with the queue idle:

| | |
| --- | --- |
| country codes holding identifier-bearing rows | 222 |
| pairings exactly one letter apart, both sides present | 2,275 |
| rows ranged over on the rarer sides | 541,225 |
| **hits — same identifier across a one-letter pair** | **726** |
| the checksum names a survivor for | 87 (12.0%) |
| neither side's country was even asked | 630 (86.8%) |

The sizing prediction held exactly (222 / 2,275 / 541,225 were the estimates),
and the 12% decidable rate independently reproduces the cohort measurement's 9
of 76. So the shape of the problem is confirmed and the class is **ten times
larger than the review cohort's slice of it** — 726 corpus-wide against 76 in
the 589 same-name cases.

Report caps at 400 rows; everything below is read off those.

## The pair view was the wrong unit. It is a CLUSTER.

Grouping the 400 carried pairs by identifier: 311 distinct identifiers, of
which 273 sit under 2 country codes — but 38 sit under 3 to **8**:

```
831496285         6 codes: BG BW VA VE VG VU     „Петрол“ АД
123531939         6 codes: BG BO GW GY VA VU     „ТЕЦ Марица изток 2“ ЕАД
103267194         6 codes: BG BI BT GA VA VU     „Софарма Трейдинг“ АД
```

One Bulgarian company, one Bulgarian EIK, a Cyrillic name — and six country
codes, of which **BG is one and the rest are spray**. The census reports that as
pairwise edges (BG/BI, VA/VU, BG/BT…) because pairs were the unit I chose, and
the pair view hides the thing that actually decides it: *the true code is
already in the cluster.* A repair driven off clusters can pick the survivor by
majority-plus-format; a repair driven off pairs cannot, because a VA/VU pair
does not contain the answer at all.

**So the census's own unit should change**: group by identifier first, then
report the country set. The one-letter test stays as the *filter* that says a
cluster is corruption rather than geography, but it is no longer the key.

## And there is a legitimate multi-country class that looks identical

The four WIDEST clusters are not typos at all:

```
2021003831        8 codes: 1A DE KE MD MZ SE UA UG   "Embassy of Sweden" / "Regeringskansliet"
43271911          7 codes: BD BF DE KE UA UG US      "Ambassade Royale du Danemark"
026481435420100   6 codes: BE BF BI ML MR NE         "Enabel — Agence belge de développement"
408712            8 codes: BA BF CH CO JO RO TD TJ   "Direction du développement et de la coopération"
```

Embassies and development agencies: **one legal entity, one register number,
procurement filed from wherever it operates.** Sweden's Regeringskansliet under
Kenya, Moldova, Mozambique, Ukraine and Uganda is not a corrupted `SE` — it is
the Swedish embassy in each of those countries, and the country field is
recording the place of the procurement rather than the registrant's domicile.

This family would have been swept up by any rule built on the pairwise
same-identifier test, and it is exactly the class where a wrong "correction"
destroys real information. It has to be excluded, and the discriminator is
neither the checksum nor the mention spread — both fail on it (Enabel is 2
mentions under NE against 249 under BE, the same asymmetry the true typos
show). What separates it is the NAME: "Embassy of", "Ambassade", "Agence …
de développement", "Direction du développement et de la coopération".

## Both countries can be wrong

A further premise of the pair framing fails outright. In several clusters
NEITHER code is right:

* `GW` / `GY` both hold „ТЕЦ Марица изток 2“ — a Bulgarian power plant.
* `AO` / `AD` both hold the Polish Krajowa Izba Odwoławcza.
* `TJ` / `TD` both hold the Swiss development directorate.

"Pick the survivor from the two" is therefore not a sound rule shape. The
cluster view fixes this too: the true code is recoverable when it is present
somewhere in the cluster, and when it is not, the census must say so and stop.

## What the 400 rows look like

| | |
| --- | --- |
| names normalize identically | 253 |
| names share a 12-character prefix | 34 |
| names genuinely differ | 113 |
| identifier shorter than 6 characters | 12 |

The 113 "names differ" rows are mostly NOT false positives — they are the same
entity in another language, transliteration or era, which is *stronger*
same-entity evidence than a string match:

* `FK:Inmac WStore SAS` / `FR:inmac wstore` — one SIRET, `38805549300059`.
* `SS:Philips AB` / `SE:Philips AB Healthcare` — one Swedish orgnr.
* `SV:ÅF-Infrastructure AB` / `SE:Afry Infrastructure AB` — one orgnr, and ÅF
  was renamed AFRY.
* `BM:„Мобилтел“ ЕАД` / `BG:А1 България ЕАД` — one EIK; Mobiltel became A1.
* `GW:„ТЕЦ Марица изток 2“ ЕАД` / `GY:„TETs Maritsa iztok 2“ EAD` — Cyrillic
  against its own transliteration.

The real false positives are the short and letter-heavy identifiers: `9948`
shared by `US:Techno-Sciences, LLC` and `ES:VICENTE TARREGA PEREZ`, and
`RCCMRCDLA2017B…` shared by two unrelated Central African companies. Twelve of
400 carry an identifier under 6 characters. A shape floor removes them.

An aside, filed here rather than as its own issue because the placeholder
lexicon (issue 300 Stage 1) is where it belongs: identifier `408712` also
carries an organization row named **"BITTE NICHT ÖFFNEN - OFFERTE"** — "DO NOT
OPEN - OFFER". A tender document's cover instruction became an organization.

## Revised plan

1. **Re-cut the census by identifier**, not by pair: report the country SET per
   identifier, the name variants, the per-code mention counts, and the anchor
   evidence once. The one-letter test becomes the corruption filter, not the
   grouping key.
2. **Exclude the operational-footprint class** — embassies, development
   agencies, anything whose off-home codes are a place of business. Name-based,
   and it needs its own measured list rather than a guessed one.
3. **Floor the identifier shape**: at least 6 characters, and no letter-heavy
   register string (issue 325's `letter_run_after_prefix` is the same
   predicate).
4. **Survivor rule** only where the cluster contains a code the identifier's
   format and the name's language both support. Otherwise abstain and report.
   Abstention is a correct answer here; a coin-flip is not.

---

## The re-cut, corpus-wide (2026-08-31, `72fb9d4`, job 543)

`country-cluster-census`: grouped by identifier, one-letter demoted to a filter,
footprint class excluded by name, shape floor at 6 characters.

| | |
| --- | --- |
| org rows walked | 1,117,485 |
| distinct identifiers | 1,112,650 |
| country codes | 222 |
| **identifiers under MORE THAN ONE country** | **4,303** |
| …with some two codes one letter apart | 615 |
| …with the **heaviest** code one letter from another | 562 |
| …identifier under 6 characters | 1,067 |
| …an operational footprint (embassy / development agency) | 19 |

Heaviest code's mention share, corpus-wide: p25 66%, p50 **80%**, p75 95%, p90 99%.

### Verdicts over all 4,303 — the abstention is the answer for 97.8%

| verdict | clusters | |
| --- | --- | --- |
| `no-one-letter-pair` | 2,681 | 62.3% |
| `too-short` | 1,067 | 24.8% |
| `nobody-asked` | **430** | 10.0% |
| `anchor-names-one` | **95** | 2.2% |
| `footprint-excluded` | 19 | 0.4% |
| `asked-and-refused` | 11 | 0.3% |

Of the 615 one-letter clusters, 79 are excluded as footprint or too-short,
leaving **536 candidates**. Of those, **95 (17.7%) are decidable** and **430
(80.2%) are blocked by nothing but a missing scheme**.

## THE BOTTLENECK IS THE VOCABULARY, NOT THE SURVIVOR RULE

This is the finding that redirects the work. The plan above assumed the hard part
was choosing a survivor once a pair was flagged. It is not. 430 of 536
candidates fail for one reason: `checksum_anchors` has no arm for the country in
question — SK, LT and BG among them, which are exactly the countries the typo
pairs are drawn from. A Slovak IČO cannot be confirmed Slovak because nobody
ever taught the probe about Slovakia.

So the next unit is **not** a survivor rule that can only ever fire on 95
clusters. It is adding register-format arms for the countries this class actually
involves, which converts most of the 430 into `anchor-names-one` and only then
makes a repair worth building. Issue 314's `country_probed` field already
surfaces the gap; this measurement sizes it.

## Two readings the capped run got backwards

The first prod run (job 542) capped the tally as well as the carried rows, and
carried rows are sorted widest-first. Two conclusions drawn from it were wrong,
both in the same direction — the wide spray clusters are unrepresentative:

* **`with_heavy_one_letter` read 104 against 615**, suggesting the sharpened
  filter cut the candidate class by 83%. Corpus-wide it is **562 against 615 —
  a cut of 8.6%.** Most one-letter clusters are plain two-code pairs where the
  heavy code IS one of the two; the wide `BG BW VA VE VG VU` shape, where the
  heavy code is one letter from nothing, is the rare case that dominated the
  carried sample.
* **The heaviest code's mention share read p50 = 90%**; corpus-wide it is 80%,
  with p25 at 66%. Less lopsided than the tail implied, which matters because a
  majority-based rule would have been tuned against the wrong distribution.

The fix was the split the issue-325 repair already had: `cap` bounds what is
REPORTED, never what is counted. It also required reaching cluster details by
PRIMARY KEY — pass 1 now carries the org id, because a lookup by `identifier` is
a full scan (no index leads with that column) and 4,303 of those is not a census.

## Still true, and now sized

* The footprint class is real but **small: 19 clusters.** Worth excluding
  because a wrong "correction" there destroys real information, not because it
  is common.
* The shape floor earns its place: **1,067 of 4,303 (24.8%)** carry an
  identifier under six characters, against 12 of 400 in the pair census's
  sample. A short number collides by arithmetic.
* `no-one-letter-pair` at 62.3% is not a failure of the filter — it is the
  measurement that most same-identifier-across-countries pairs are NOT
  transcription slips, which is exactly what the filter is for.

## Which checksum arms to write — read off the corpus (job 545, `c75c8b0`)

The 430 undecidable clusters, cut by the **heaviest** code — the side the entity
actually lives on:

| | heavy | all codes | cumulative (heavy) |
| --- | --- | --- | --- |
| **BG** | **147** | 150 | 34.2% |
| **LT** | **110** | 128 | 59.8% |
| **DE** | **32** | 47 | 67.2% |
| **ES** | **23** | 31 | 72.6% |
| **SK** | **18** | 21 | 76.7% |
| **LV** | **15** | 91 | **80.2%** |
| EE | 14 | 33 | 83.5% |
| IE | 11 | 21 | 86.0% |

**Six arms — BG, LT, DE, ES, SK, LV — cover 80.2% of the class.**

### The all-codes cut names the wrong countries, and by a lot

The first reading counted every code in a cluster and ranked
`BG 150, LT 128, LV 91, DE 47, BI 34, BT 32, VU 26, VA 20, BF 16, VG 15`.
Burundi, Bhutan, Vanuatu, the Vatican, Burkina Faso — **there are no Burundian
registrants in this class.** `BI` is what `BG` gets mistyped into; `VU` is what
`VA` gets mistyped into. An arm for Burundi would validate nothing, and ranking
off that list would have sent the next unit to write a Somali register checksum.

Cut by the heavy side, **35 countries remain of 86** — 51 appear only ever as the
typo target (BI, BT, VU, VA, BF, VG, VE, VN, SO, BW, GA, BZ, GH, GN…).

**Latvia is the sharpest case and it inverts.** `LV` is third on all codes (91)
and sixth on the heavy side (15): Latvia is overwhelmingly what **Lithuania gets
mistyped into**, not a country whose registrants need validating. Sweden is
starker still — 33 appearances, **never once the heavy side.** So are CZ, NO and
GB.

## Next unit, now specified

1. Write checksum/format arms for **BG, LT, DE, ES, SK, LV** in
   `idgate::checksum_anchors` and their entries in `anchor_vocabulary`.
2. **Validate against the corpus, not against a spec I half-remember.** The
   `org-merge-health` scheme tally already measures pass/fail per scheme over
   every identifier-bearing row; a correct arm reads ≥97% pass on its own
   country's rows (the bar Stage 0 set), and a wrong one reads near chance.
   That loop is the reason to write these here rather than trust a formula.
3. Re-run `country-cluster-census` and measure how many of the 430 move to
   `anchor-names-one`.
4. Only then is the survivor rule worth building. Today it could fire on 95
   clusters; after the arms it should reach several hundred.

## The arms are in, and the decidable set went 95 → 357 (`62bfbe1`, job 546)

| verdict | before | after | delta |
| --- | --- | --- | --- |
| `no-one-letter-pair` | 2,681 | 2,681 | — |
| `too-short` | 1,067 | 1,067 | — |
| **`anchor-names-one`** | **95** | **357** | **+262** |
| `nobody-asked` | 430 | **153** | −277 |
| `anchor-names-several` | 0 | 19 | +19 |
| `asked-and-refused` | 11 | 7 | −4 |
| `footprint-excluded` | 19 | 19 | — |

**Of 536 candidates, 357 are now decidable — 66.6%, up from 17.7%.** That is
what makes the survivor rule worth building; before the arms it could have fired
on 95 clusters.

Written from specification and then **validated against prod's own rows before
deploying**: `bg_eik` 94.0%, `lt_kodas` 94.0%, `SK:ico` (riding `cz_ico`) 98.5%,
against a chance rate of about 9.1% for mod-11. A wrong arm reads near chance,
so the corpus is the proof.

### The arms nearly shipped a 50% capability loss

The resolver's anchor path and the R3 merge both require **exactly one** anchor.
Every scheme added to the shared `uniform_arm` table therefore makes some
previously-decidable value ambiguous and silently narrows a merge path. Measured
on 1,500 random corpus values per shape, with the arms in the shared table:

```
8-digit: single-anchor 800 -> 385    51.9% of anchored values LOSE the path
9-digit: single-anchor 662 -> 521    21.3% LOSE it
```

Slovakia is the extreme: `SK:ico` is the *same arithmetic* as `CZ:ico`, so adding
it under its own name doubles every Czech anchor. Halving the 8-digit merge
path's reach in exchange for 18 census clusters is not a trade to make silently,
and it only surfaced by asking who else calls `checksum_anchors`.

So the probes are **split by question**. `census_anchors` / `census_vocabulary`
answer *what is the evidence* and carry the new arms; `checksum_anchors` /
`anchor_vocabulary` answer *what may I merge on* and are untouched. The census
can afford ambiguity — it intersects anchors with a cluster's own codes and
reports the honest `anchor-names-several` — where a merge cannot. The evidence
probe is built ON TOP of the decision probe, so the shared arms cannot drift.

### Two properties of these arms, stated because they are not obvious

* **`bg_eik` and `lt_kodas` are nearly the same function.** Same first pass
  (weights 1..8 mod 11), differing only in a ~1-in-11 fallback branch: `bg_eik`
  passes **93.8%** of Lithuanian rows and `lt_kodas` **84.2%** of Bulgarian ones.
  A joint pass is evidence for **neither**. Harmless here, because `BG`/`LT` are
  not one letter apart and a cluster holding both is `no-one-letter-pair` before
  anchors are consulted — but it would matter to anything reading a lone anchor
  as country evidence.
* **A single mistyped digit is not always caught: 98.35% caught, 1.65% missed**,
  over every position and substitution on 200 passing values. The two-pass
  fallback lets a slip move into the second weighting and validate by
  coincidence. A first draft of the test asserted 100% and was wrong. The misses
  spread evenly across positions, so it is a uniform 1-in-60 rather than a blind
  spot a typo could hide in.

### What stays undecidable, and why

Remaining `nobody-asked` by heavy country: **DE 32, ES 23**, EE 14, IE 11, LV 11,
NL 8, AT 5, GR 5, BE 4, HU 4.

DE and ES were always going to stay: **German register numbers carry no checksum
at all**, and a Spanish CIF has a letter, which this digits-only probe declines
wholesale. Together they are 55 of the remaining 153 and no arm can reach them.
The rest are a long tail where an arm buys ten clusters or fewer.

## Next unit

The survivor rule, now that 357 clusters carry a decisive anchor. Shape as
before: fire only where the evidence is decisive, abstain otherwise, and exclude
the footprint class. `majority_share` is carried for the abstain cases but must
not decide — the pair census caught it inverting.

## Step 2 built, dry plan reviewed, NOT applied (`de7f3fc`, jobs 547/548)

`repair-country-typos`: 4,303 clusters walked, **357 decisive**, **343 rows
planned**, 147 left unmoved because their code is not one letter from the
survivor.

Most of it is convincing. `LV → LT` ×75 on companies named `UAB "AE Medical"`,
`UAB „Barameda“`, `Uždaroji akcinė bendrovė „Kuršasta…"` — **UAB *is* the
Lithuanian company form**, so these are Lithuanian firms filed under Latvia,
exactly as the heavy-side cut predicted. Then the Bulgarian spray
(`BI/BT/BF/VG/BR/BO/BW/BZ → BG` ×119), `CR → CZ` ×14, `SR → SK` ×8.

### The dry review earned its place: two of the heaviest moves are not typos

| move | mentions | what it actually is |
| --- | --- | --- |
| `IE → IT` XL Insurance Company SE | 65 | one insurer registered in **three** countries (IT, GB, IE) |
| `SI → SE` Gorup – Audio Stojan Gorup S.P. | 17 | a **Slovenian** sole proprietor (`S.P.`) whose number passes the Swedish Luhn |

Both abandon a country that was **never asked** — no Slovenian scheme covers ten
digits, no Irish one eleven — so the move rests on the survivor's anchor alone.

### That axis is not a usable gate, and the measurement says so

Splitting all 343 moves on it: **5 asked-and-refused, 338 never-asked.** The
"safe" subset is essentially empty, and the cleanest moves in the plan — the
`LV → LT` UAB companies — are never-asked too, because no Latvian scheme covers
nine digits either. So the split ships as **information for a reviewer, not a
filter**; gating on it would discard the best evidence with the worst.

I also checked `hard_scheme` as a bar and it does not separate these: `SE:orgnr`
and `IT:piva` are both already hard. Hardness is checksum strength; what failed
here is country specificity.

### Two candidate tightenings, measured

**(a) Require every code in the cluster to be one letter from the survivor.**
A cluster holding a code that is neither the survivor nor a neighbour is a
multi-country registration rather than a slip — that is exactly XL Insurance's
`GB`. Removes **47 of 343**, leaving 296. Catches XL Insurance; misses Gorup,
whose cluster is a clean `SI`/`SE` pair.

**(b) Refuse a survivor named only by a Luhn-family scheme** (`FR:siren`,
`SE:orgnr`). Luhn is a pure checksum with no country semantics — this file
already says a 10-digit Luhn pass "could be a SE orgnr or match PL:nip's shape" —
and it is what put Gorup in Sweden. **Not implementable from the current plan**:
the move does not carry WHICH scheme named the survivor, and a length proxy is
useless because 261 of the moves are nine digits where the namer is `BG:eik` or
`LT:kodas` (mod-11), not Luhn.

## Next unit, specified

1. Carry the naming scheme on each `CountryTypoMove` — a few lines, and it is
   what (b) needs.
2. Apply (a) and (b), re-run the dry pass, and re-read the heaviest moves the way
   this review did. The two known false positives are the acceptance test: both
   must be gone.
3. Then the capped wet run, followed by `match-org-identifiers --r2` — **every
   move lands on an identity the survivor already holds, on purpose**, so the
   merge arm is a required second step and not an afterthought.

## Step 2b DONE: tightened, reviewed, and APPLIED — 312 rows moved (`105c519`, jobs 549/550)

Two tightenings were specified. **One survived contact with the data and one did
not**, which is the useful part.

### (b) survived: a bare Luhn is not country evidence

`SE:orgnr` and `FR:siren` share one arithmetic, and `idgate` already notes a
10-digit Luhn pass "could be a SE orgnr or match PL:nip's shape". A survivor
named only by one of those carries no country information — it is what moved
"Gorup – Audio Stojan Gorup S.P." into Sweden. Refuses **16** clusters.

Expressing it required carrying the naming SCHEME on each move: a length proxy
fails, because 261 of the 343 moves are nine digits where the namer is `BG:eik`
or `LT:kodas`, not `FR:siren`.

### (a) did NOT survive: cluster shape cannot tell spray from a real registration

Refusing any cluster holding a code neither the survivor nor one letter from it
caught XL Insurance — and also refused **`Софарма Трейдинг АД`**, one Bulgarian
EIK under `BG` plus five junk codes with a single mention each. That is precisely
the corruption this repair exists for. My own earlier test caught the
over-reach.

### What replaced it: a measured weight veto

The moved row's own mention count separates them, and the distribution is
unambiguous over the 343 candidate moves:

```
p50 1   p75 2   p90 3   p95 4      256 of 343 carry exactly ONE mention
```

Both known false positives sit far out in that tail — **XL Insurance at 65,
Gorup at 17**. `TYPO_MOVE_MENTION_VETO = 5` is the first value above the typo
bulk. Refuses **14** rows.

So weight does not vote *for* a survivor — the anchor does that — but it does
veto a move *away from* a country the row has real standing in. Opposite
questions, and the rule holds both.

**The limitation is deliberate:** a heavily-published mis-countried row is no
longer auto-repaired. For a published field that is the right trade — the same
weight that would make it worth fixing is what makes it indistinguishable from a
real registration. Those land in `refused_row_has_standing` for review.

### Acceptance, then a 20-case name review, then the write

| | before tightening | after |
| --- | --- | --- |
| planned moves | 343 | **312** |
| heaviest row moved | 65 mentions | **4 mentions** |
| XL Insurance `IE→IT` | present | **gone** |
| Gorup `SI→SE` | present | **gone** |

Every namer is now a national scheme — `BG:eik` 136, `LT:kodas` 104, `SK:ico` 24,
`CZ:ico` 16, `IT:piva` 9 — and **no `FR:siren` or `SE:orgnr` survives**.

A random 20 of the 312 were then read by name, and all 20 are right:
`UAB "Impromedica"`, `Uždaroji akcinė bendrovė "Medita"`,
`Lietuvos Ir Latvijos Uždaroji Akcinė Bendrovė` → LT (UAB *is* the Lithuanian
form); `Институт по астрономия`, `„Контракс“ АД`, `Кооперация ПАНДА` → BG;
`Výskumný ústav detskej psychológie`, `Studio Jelínek s.r.o.` → SK/CZ;
`Direção-Geral da Segurança Social` → PT; `Steinkjer kommune` → NO.

**Applied: 312 of 312, 0 skipped, no partial.** Verified on prod afterwards:

* **297 duplicate identities created** — exactly one per moved identifier, so
  every move landed on the survivor's identity as designed.
* **36 clusters remain multi-country**, which is correct: those are the
  `left_unmoved` stranger codes (`GA`, `VA`, `VU` and the like) that are not one
  letter from the survivor.

## Residue: the R2 fold, and why it was not run

The 297 duplicates are deliberate — a moved row lands on the identity it should
always have had — and `match-org-identifiers --r2` is the arm that folds them.
Its dry run plans **200 groups** against **829** candidates (565 denied
consortium, 64 legal-form, 0 cap, 0 gate).

**It was not run wet, because its plan cannot be reviewed.** The
`r2-merge-plan` report carries only counters and a single sample row — no
per-group listing — so there is no way to tell how many of the 200 are this
repair's 297 and how many are pre-existing, nor to read the groups before
merging. Running a 200-group merge unreviewed on the tail of a 312-row repair is
not a trade worth making.

That is itself worth fixing: **the R2 plan should carry a capped per-group
listing** the way `country-typo-repair` and `minted-country-repair` do. Then the
fold is a normal reviewed step instead of a leap.

## The R2 fold, now that the plan can be read (`8c8175f`, jobs 552/553)

**Why it was stuck.** `plan_sample` is a fixed 1-in-199 content-stable
acceptance — the right shape for estimating precision over a huge plan, and the
wrong shape for reading a small one. Against prod's 200-group plan it yields
exactly **one** row, so the only choices were running a 200-group merge
unreviewed or not running it.

Fixed by adding a capped LISTING beside the sample (`R2_PLAN_LISTING_CAP` = 500;
`plan_listing_truncated` flags a plan that overflows it). The sample keeps its
unbiased-over-large-plans guarantee; the listing is complete whenever the plan
fits. A wet run records no listing — it is a review artifact for a plan not yet
applied — and the test pins that alongside the completeness.

### What the listing showed, which nothing else could

| | |
| --- | --- |
| plan groups | 200, **all 200 listed**, not truncated |
| **created by this repair** | **195** |
| pre-existing | 5 |
| group sizes | 189 pairs, 10 triples, 1 quad |

The members are plainly one entity per group — full name against abbreviation
(`„Застрахователна компания Надежда"` / `ЗК Надежда АД`), with and without the
legal form (`NKT A/S` / `NKT`, `Solari di Udine` / `Solari di Udine SpA.`), and
punctuation variants (`РОЕЛ-98 ООД` / `„Роел 98“ ООД` / `„Роел-98“ ООД`). The
widest group is the Bulgarian Ministry of Environment and Water four times over.

**The 5 pre-existing groups are all French**, and worth naming because they are a
different kind of merge: two SIRET *establishments* sharing one SIREN — `INRA` /
`INRAE Centre Occitanie Montpellier`, `Colas Baie d'Armor` / `COLAS France
ETABLISSEMENT COTE BASQUE`, `SPIE Sud-Est` / `SPIE BUILDING SOLUTIONS`. R2 keys
on the legal entity, so it folds establishments into one organization. That is
its documented E1 behaviour and predates this work; noted rather than changed.

### Applied, and the residue is R2 refusing on purpose

**200 groups merged, 212 org rows retired.** The duplicate identities this
repair created fell **297 → 102**.

The 102 are not a gap. R2's denial arithmetic accounts for every group exactly:

```
829 groups >= 2  -  565 consortium  -  64 legal-form  =  200 planned
                    (0 cap, 0 gate)
```

So every group R2 did not deny was planned and merged, and the 102 sit inside the
629 it deliberately refused — the consortium and legal-form guards that exist
because merging across them is a measured false-merge shape (the FI/SE-twin
case). Leaving them is R2 working, not R2 failing.

The 36 clusters still holding more than one country are likewise correct: those
are the `left_unmoved` stranger codes, which never had evidence of a slip.

Health green after the merge; the reclaim-without-ledger counter is zero.
