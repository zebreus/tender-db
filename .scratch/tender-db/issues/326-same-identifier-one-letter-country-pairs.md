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
