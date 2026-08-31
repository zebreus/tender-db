# 326 — Same identifier, country codes one letter apart: a typo class the review campaign is deciding by hand

Status: OPEN — measured 2026-08-31 on the stored issue-314 cohort, not yet built
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
