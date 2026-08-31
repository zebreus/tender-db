# 325 — The VAT-prefix sniffer mints a country from any word that starts with two country letters

Status: PREVENTION FIXED 2026-08-31 — the arm is tightened and the minted
code canonicalised. REPAIR of the 3,789 standing rows is still open (step 4).
Kind: data-quality / correctness (organization layer, ingest)
Relates to: 86 (fixed the OTHER half of this same site and left this arm loose),
314 (the review campaign that surfaced it — 40 of its 589 cases carry one), 319
(country normalization), 300 Stage 1 (the gate this feeds)

`CHARITYNO298028` is filed under country **CH**. The org is "Victim Support", a
British charity. "Charity No. 298028" begins with the letters `CH`, and that is
the entire reason the row says Switzerland.

## The mechanism

`normalise_identifier` (`crates/ingest/src/project.rs`, the VAT arm):

```rust
let vat_prefix: String = value.chars().take(2).collect();
let is_vat = VAT_COUNTRIES.contains(&vat_prefix.as_str())
    && value[2..].chars().any(|c| c.is_ascii_digit());
```

`any(...)` — a digit ANYWHERE in the remainder. So any string whose first two
letters spell a VAT country and which contains a digit somewhere becomes a VAT
identifier scoped to that country. The identifier is stored as `kind = 'vat'`,
and the minted country lands on the organization row.

The discipline this arm needs is already written down **eleven lines above it**,
for the register-prefix arm:

> a 2-letter tag (FN, VR, PR) also requires a following digit, so it cannot
> swallow an unrelated word that merely begins with its letters.

That is exactly the failure here, one arm down. Issue 86 fixed the direction
where a *scheme tag* minted a country (`HRB Dresden` → HR/Croatia). This is the
mirror image: a *word* minting a country. 86 is RESOLVED-VERIFIED and its fix
was correct; it simply never tightened the arm it fell through to.

## The measurement (prod, 2026-08-31, via `/v1/sql`, queue idle)

Two shapes, both `identifier_kind = 'vat'`:

| class | predicate | org rows | countries |
| ----- | --------- | -------- | --------- |
| **W** (word) | three letters immediately after the two-letter prefix | **1,276** | 32 |
| **G** (guid) | 32 chars, pure hex | **2,930** | 3 (EE 1,088 · DE 1,013 · BE 829) |

**4,206 of 80,832 `kind='vat'` org rows — 5.2%.** Class W alone carries
**30,343 mentions**.

No real VAT scheme puts three letters straight after the country code (AT has
one `U`, ES one letter, FR two check chars, GB's `GD`/`HA` two), so class W's
predicate does not catch legitimate ids. Class G is a separate shape the letter
test misses (`EE0C5B32…` — the third character is a digit).

### Class G is proved by which countries CANNOT appear

Query the whole 32-char pure-hex population by country: FR 191,847 · DE 46,951 ·
CH 25,380 · EE 1,094 · BE 857 · AT 167 · … Now ask which of those have the
country equal to the identifier's first two characters: **only EE, DE and BE**,
because those are the only three whose codes are also valid hex pairs. `FR`
cannot be — `R` is not a hex digit — and FR is the *largest* population.

And the ratios settle it. EE: 1,088 of 1,094 (99.5%). BE: 829 of 857 (96.7%).
Chance would give 1/256 ≈ 0.4%. The EE and BE hex populations are not merely
contaminated by this rule — they are **generated** by it.

### Class W is proved by reading the names

| stored country | identifier | org name | what it actually is |
| -------------- | ---------- | -------- | ------------------- |
| CH | `CHARITYNO298028` | Victim Support | "Charity No." — a UK charity |
| CH | `CHARITYNUMBER1040303` | Citizens Advice Wandsworth | ditto ×24 |
| BE | `BERICHTSEINHEITID00002636` | traffiQ, Frankfurt | German "Berichtseinheit-ID" |
| BE | `BERLINCHARLOTTENBURG…` | villadata systemhaus GmbH | Amtsgericht Berlin-Charlottenburg |
| AT | `ATTOGE13295DEL28102022CIG…` | Istituto Nazionale di Fisica Nucleare | Italian "atto … del … CIG" |
| BG | `BGOBGZ2610262017` | Najwyższa Izba Kontroli | the Polish audit office |
| BG | `BGLFRZ76T09F712E` | Fabrizio Bigiolli | an Italian codice fiscale |
| FI | `FINANZAMTESBIELEFELD…` | (German tax office) | "Finanzamt Bielefeld" |
| FI | `FIRMENBUCHNUMMER441612F` | HelmCare GmbH | Austrian Firmenbuch number |
| FR | `FRANKFURTHRB105754` | (German company) | "Frankfurt HRB 105754" |
| FR | `FRRSFN75A45F839O` | Stefania Ferrante | Italian codice fiscale |
| IS | `ISCRITTAPRESSOLACCIAADIROMA…` | (Italian company) | "iscritta presso la CCIAA di Roma" |
| LI | `LIDERKONSORCJUM9661386113` | JMM Justyna Poźniak | Polish "lider konsorcjum" |
| SI | `SIDERIDRAULICSYSTEMSPACF…` | Sider Idraulic Systems SpA | Italian |
| DE | `DECRETODIRIGENZIALE1486762017` | (Italian body) | Italian "decreto dirigenziale" |
| ES | `ESTRADADOBAIRROSN2600614…` | (Portuguese) | "Estrada do Bairro s/n, 2600-614" |

The BE group is 715 rows — 56% of class W — and every sampled sub-group
(`BERICHTSEINHEITID` 595, `BERICHTSEINHEITSID` 14, `BERICHTEINHEITID` 6,
`BETRIEBSEINHEITID` 2, `BEREICHSEINHEITID` 2 …) names a **German** public body:
traffiQ Frankfurt, Vergabekammer Mecklenburg-Vorpommern, Landkreis
Vorpommern-Rügen, Stadtwerke Augsburg, Zweckverband Personennahverkehr
Saarland. All filed as Belgian.

## What the class is NOT

Some of it is accidentally **right**: `UKCOMPANYREGISTER03752719` (UK),
`USFEDERALTAXID132956718` (US), `AUSTRALIANABN36689395480` (AU),
`ELGEMI152311809000` (a Greek company), `SEBOLREG5566609490` (a real Swedish
orgnr). The publisher wrote the country's own letters at the head of a free-text
identifier and the sniffer got the right answer for the wrong reason.

So the claim is **not** "4,206 wrong countries". It is:

1. **The country is accidental in all 4,206** — derived from characters that are
   not a country prefix. Right answers are luck, not evidence.
2. **`kind = 'vat'` is wrong in all 4,206** — none of these are VAT numbers, yet
   all of them enter the VAT merge-key space and are read as VAT by
   `idgate::census`/`condemns` (which treat a value as "digits plus a vat
   prefix" when kind says vat).
3. **The wrong-country fraction is large but unmeasured.** In a 26-row spread
   across the alphabetically-first countries, 24 are demonstrably wrong — but
   that sample is not random, and the honest number needs one that is. That is
   step 1 below, not a claim to make now.

## Overlap with the issue-314 review campaign

Computed locally against the stored 589-case packet: **40 cases (6.8%) contain
at least one member whose country is this artefact** (21 class W, 19 class G).
Examples from the cohort: org 23642523 `EEE9F40A…` country EE, name "Gemeinde
Glattfelden" (Swiss); org 24487459 `BE2A168C…` country BE, name "Kanton Wallis
- DMRU" (Swiss); org 14351771 `FIRMENBUCHNUMMER441612F` country FI, name
"HelmCare GmbH" (Austrian).

**This does not replace the campaign** — 93.2% of the cohort is untouched by the
rule, and the campaign's own finding stands: cross-border same-name components
are mostly country contamination from causes this rule does not explain. What it
does mean is that ~40 cases × 2 reviewing agents are being spent hand-deciding
something a predicate decides, and that those 40 should be *excluded* from the
remaining slices rather than reviewed.

## Fix (proposed, not yet done)

1. **Measure the wrong-country rate on a random sample** of class W ∪ G before
   choosing a repair. The class splits into "accidentally right" and
   "accidentally wrong" and the split governs whether the repair is "re-derive
   the country" or "null the country and let the resolver re-decide".
2. **Tighten the VAT arm.** The remainder after the prefix must look like a
   registration number, not prose. Candidate predicate, mirroring the register
   arm's existing discipline: reject when `idgate::letter_run_after_prefix`
   sees a run of ≥3, and bound the length (SE's 12 digits is the longest real
   remainder; 30 hex characters is not a VAT number). `letter_run_after_prefix`
   already exists and is already the codebase's notion of "letters where digits
   belong" — do not write a second one.
3. **Decide what a rejected value becomes.** `national` with the country from
   the mention's own country field is the obvious answer, and it is what these
   rows should have been all along — but it is a merge-key change, so it goes
   through the Stage-1 gate census the way issue 86's did, not straight in.
4. **Re-parse the standing rows.** 4,206 org rows and ≥30,343 mentions. 86's
   precedent is the right shape: fix in code, materialise on a reprojection,
   verify the distribution afterwards (86's acceptance was "no false HR bucket
   anywhere in the distribution" — the equivalent here is no `CHARITYNO`/
   `BERICHTSEINHEIT`/hex-GUID head under a minted country).
5. **A tripwire.** This class is mechanically detectable — the two predicates in
   the table above are the whole test. It should be a scheduled count that
   alarms above a floor, so the next loose prefix arm announces itself instead
   of waiting for a review campaign to notice.

## Why this was found by hand and not by a rule

Worth recording, because it is the second time (issue 311's GUID class was the
first). The review campaign's reviewers wrote rationales, and three of slice 1's
100 rationales independently named the mechanism — one of them exactly:

> a VAT-prefix parser reading the first two characters of "BERICHTSEINHEITID…"
> would emit exactly "BE", which names the mistake mechanically rather than by
> inference.

The campaign's value here was not the 100 verdicts. It was that a reviewer
looking at one case could see a *rule* the aggregate measurements had not. That
is the argument for keeping rationales, not just verdicts.

## Measurement provenance

All counts above are from `/v1/sql` against the serving DB on 2026-08-31 with
the queue idle (`/health/deep` `last_job` finished, all checks ok), each a
single-table aggregate inside the endpoint's own 10 s cap — the bounded
data-page path of `docs/agents/prod-box-reads.md`. The 589-case overlap was
computed locally from the stored packet, touching prod not at all.

---

## Step 1 answered mechanically — no sampling needed

The issue's step 1 asked for "a random sample" to get the wrong-country rate.
That turned out to be unnecessary: **the publisher already tells us the country**,
on every mention, and `normalise_identifier` receives it as its `country`
argument before overriding it with the prefix guess. So the rate is a join, not
an estimate.

Measured on prod, queue idle, via `/v1/sql`:

| the mention's own country field, against the minted code | mentions |
| --- | --- |
| **DISAGREES** | **34,111** |
| AGREES | 946 |
| not alpha-2 | 8 |
| absent | 3 |

**97.3% of the mentions on these rows carry a stated country that contradicts
the code taken out of the identifier's first two letters.**

Per organization, and this is the number that governs the repair:

| | orgs | mentions |
| --- | --- | --- |
| all mentions name ONE country, and it **contradicts** the minted code | **3,789** | 33,962 |
| all mentions name one country, and it matches | 268 | 946 |
| the mentions disagree among themselves | **2** | 149 |

So **3,789 of 4,059 (93.4%) have an unambiguous repair already recorded in
their own mentions**, 268 were accidentally right, and exactly **two** rows are
genuinely ambiguous. The repair mapping is the mechanism, read back:

    BE -> DE  832     BERICHTSEINHEITID… German public bodies
    EE -> FR  781     32-char hex GUIDs on French entities
    DE -> FR  698     ditto
    BE -> FR  547     ditto
    EE -> DE  197
    DE -> CH  113     Swiss cantons and municipalities
    EE -> CH  104
    BE -> CH  104
    FR -> IT   36     Italian codici fiscali (FRRSFN75A45F839O…)
    NO -> FR   26
    LI -> PL   21     LIDERKONSORCJUM… Polish consortia
    DE -> IT   20
    PL -> IT   19
    SE -> PL   16
    GR -> IT   15

## A third defect in the same three lines: it re-contaminates issue 319

The arm stored `Some(vat_prefix)` **raw**. `VAT_COUNTRIES` holds both `EL` and
`GR`, both `UK` and `GB`, and `XI` — so the arm mints non-ISO codes, while every
other writer canonicalises to alpha-2 first (`m.country = canonical_country(&c)`,
three lines above the call site).

Issue 319 is RESOLVED and folded the country column to alpha-2. Prod today:

    EL  vat  222
    UK  vat   38
    XI  vat    2

**262 rows, every one `kind = 'vat'`** — i.e. minted by this arm *after* the
fold ran. A resolved issue with a live re-contamination channel is worse than an
open one, because nobody is watching it.

## The prevention fix (done)

`normalise_identifier`'s VAT arm now requires the two-letter prefix to be
followed by *the registration number itself*, not by the middle of a word:

* a digit somewhere in the body (unchanged),
* body length ≤ 14 — Sweden's twelve is the longest real one, and a 32-char hex
  GUID has a 30-character body,
* **no run of three consecutive letters in the body** — the sharp test, and the
  one that separates `ESX1234567X` from `ESTRADADOBAIRROSN…`. No European
  scheme puts three letters straight after the country code: Austria has one
  `U`, a Spanish CIF one letter, France two check characters, GB's `GD`/`HA`
  two.

…and canonicalises the minted code, so `EL…` lands under `GR` and `UK…` under
`GB`. The published VALUE keeps its own spelling — rewriting it would be the
value reshaping issue 300 Stage 1 deliberately keeps out of this function.

A rejected value now takes the `national()` path, which scopes it by **the
mention's own country** — exactly the field the measurement above shows to be
right 97.3% of the time.

### Tests

* `a_word_that_starts_with_a_country_code_does_not_mint_that_country` — fifteen
  REAL prod values with the country each one actually minted, asserting they are
  now `national`, scoped by the mention, value untouched.
* `the_tightened_vat_arm_still_admits_every_real_scheme_shape` — every European
  scheme that puts a non-digit right after the country code, which is precisely
  what a careless "must be followed by a digit" rule would have broken.
* `real_vat_ids_keep_their_country_prefix` — one expectation CHANGED: it asserted
  `EL094019245` → country `EL`. That was correct when written and is wrong after
  issue 319. Recorded in the test, not silently flipped.

**A note for whoever writes the next fixture here.** Five values in this test's
first draft were swallowed whole by the v2 gate and returned `None`:
`ESX1234567X` and `BERLINCHARLOTTENBURG12345` (ascending digit runs),
`FRXX999999999` and `GBGD001` (filler and short-VAT stub), `SE556602998601` (a
Luhn that does not close — SE:vat is a HARD scheme). Every one would have made
the test pass or fail on `idgate::condemns`'s behaviour instead of on this arm's.
**A synthetic VAT id is nearly always a gate-refused one.** Use a real
registrant.

## What is still open (step 4: the repair)

The 3,789 standing rows and their 33,962 mentions. Shape, following issue 312's
lesson and this repo's dry-first ladder:

1. A dry plan: per org, the minted code, the unanimous mention country, the
   mention count. Stored as a report and reviewed before any write.
2. A wet pass with pre-images (`country` is a published field, so each moved row
   needs a change event — issue 319's fold is the precedent, including its
   `expect_rows` tolerance gate).
3. The two ambiguous orgs get no automatic action.
4. Re-read the `EL`/`UK`/`XI` counts afterwards: they must go to zero and stay
   there, which is now true by construction at ingest but not yet for the stock.

A tripwire also remains unbuilt (step 5): the two predicates in the class table
are the whole test, and they should be a scheduled count so the next loose
prefix arm announces itself.

## Why the prevention deployed before the repair

Deployed `81e4b1d`. That opens a prevention-vs-stock split for as long as the
stock rows stand: a fresh mention of `BERICHTSEINHEITID00002636` now parses as
`(national, DE)` while the standing org row is `(vat, BE)`, so it mints a new
org instead of joining the existing one. Stated plainly because it is a real
cost, not a footnote.

Deployed anyway, for two reasons:

1. **The window is self-healing.** The repair sets both fields — `country` to
   the unanimous mention country AND `identifier_kind` to `national` — which is
   exactly what the new parse produces. After it runs, the stock row and any
   fragment minted in the meantime share one `(kind, country, value)` key, and
   the existing same-country same-identifier merge path folds them through a
   tested route. Nothing is lost, only deferred.
2. **The alternative accumulates.** Every day the old arm runs is another day of
   fresh rows filed under a country taken out of a word, and of fresh `EL`/`UK`/
   `XI` codes re-contaminating a column issue 319 already folded.

The repair must therefore set BOTH fields. A country-only repair would leave the
`kind` mismatch and keep the split open indefinitely — worth writing down,
because "fix the wrong country" is the obvious reading of this issue's title and
it is half a fix.
