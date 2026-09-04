# 349 — the genericness wall reads fragmented Greek public bodies as generic: 22 of 155 agreeing GR groups denied, 20 of them one entity in 20–54 rows

Status: MEASURED 2026-09-04 (job 645, `d98d8e6`) AND FIX (c) BUILT — **315 of the 375 `agree-generic` groups are echo** (DE:vat 280/329, GR 15/22, DE:national 10/14, LT 9/9, AT 1/1): the identifier-bearing carriers alone sit under the cap. Not a Greek exception — the class itself. The E0 name rule now admits an echo key and denies only a key shared by over-cap identified rows (`admitted_echo` in the report; test `an_echo_generic_name_is_admitted_and_a_shared_one_denied`); the resolver/R3 walls are untouched and filed as issue 350. Was: MEASUREMENT BUILT 2026-09-04 (gate running) — the duplicate-identity census now probes every generic key it meets (`GENERIC_KEY_BREAKDOWN_SQL`: carriers, with identifier, with country, distinct identifiers, bounded at 1,000) and splits `agree-generic` groups into `echo` (identifier-bearing carriers alone under the cap) and `shared`, per scope, with up to 200 probes listed; test `generic_keys_are_split_into_echo_and_shared`. Then: deploy, run the census, read the 375-key split, choose (a) or (b). Was: ready-for-agent (filed 2026-09-04 from issue 348's first probe)
Kind: identity semantics (organization layer) — the wall's statistic on one scope
Relates to: 316/318 (the wall), 331 (asked exactly this, measured corpus-wide), 332 (closed negative corpus-wide — this is the exception it allowed for), 346 (how it surfaced), 329 (the E0 fold that loses these groups), 300 Stage 3/5 (NULL-country rescue — most of these carriers are that class)

## Observed

The wall calls a name key generic when more than `STOPLIST_CAP = 20` distinct
org rows carry it (`GENERIC_KEY_SQL`). Issue 332 checked the statistic
corpus-wide and found 99.5% of decidable over-cap keys are genuinely shared
names. Greek public bodies are the other 0.5%, and after issue 346 folded their
two casings into one key they cross the wall in numbers:

| key (n2) | carriers | of the ≤30 shown: NULL-country rows | rows with no identifier | distinct identifiers |
| --- | --- | --- | --- | --- |
| `ενιαια αρχη δημοσιων συμβασεων` | 54 | 0 | 9 | 18 (hand-typed variants of one authority code) |
| `γενικο νοσοκομειο σερρων` | 44 | 25 | 27 | 3 |
| `σταθερεσ συγκοινωνιεσ μονοπροσωπη α ε` | 41 | 0 | 3 | 28 |
| `γενικο νοσοκομειο βολου αχιλλοπουλειο` | 36 | 23 | 30 | 1 |
| `δημοσ θεσσαλονικησ` | 34 | 4 | 6 | 23 |
| `εθνικο και καποδιστριακο πανεπιστημιο αθηνων` | 32 | 7 | 12 | 17 |
| `πανεπιστημιακο γενικο νοσοκομειο ηρακλειου` | 29 | 23 | 25 | 4 |
| `δημοσ αγρινιου` | 28 | 23 | 26 | 2 |
| `πανεπιστημιο ιωαννινων` | 25 | 20 | 22 | 3 |
| … 13 more between 21 and 24 | | | | |

Every one is a single hospital, municipality, university or authority. The
carriers are its own rows: NULL-country identifier-less provisional rows minted
per spelling (`Δήμος Αγρινίου` / `Δημος Αγρινιου` / `Δήμος Αγρίνιου` …), plus
GR rows carrying the many hand-typed variants of its authority code
(`1000E009610001`, `1000E00961001`, `1007E009610001`, `100E009610001`,
`1015E009610001` — one authority, five codes). Greek notices publish buyer
names without a stable identifier far more often than the rest of the corpus,
so the same entity fragments into dozens of rows, and the wall — which counts
rows — reads that as "a name nobody chose to make unique".

Of the 155 GR:national same-identifier groups whose names agree, 22 are denied
by the wall; the distinctive 133 carry 2–20 rows (mode 5). The E0 fold (329)
loses the 22; the R3 rescue and the resolver's prevention hook are governed by
the same verdict.

A side finding from the same rows: org 311 (the Single Public Procurement
Authority) carries the satellite name `ΓΕΝΙΚΟ ΝΟΣΟΚΟΜΕΙΟ ΣΕΡΡΩΝ` from one
mention — a source-side mislabel that resolved to it. One row; noted, not filed.

## Proposal (measure first, then one of two)

1. **Measure the shape corpus-wide with the 348 probe**: for every over-cap key
   in the census's `agree-generic` bucket (375 groups), the share of carriers
   that are NULL-country or identifier-less, and the distinct-identifier count.
   Issue 332's 0.5% was measured before the tonos fold and before the wall's
   GR exposure grew; the exception may be larger than it was.
2. Then either
   - **(a) a fragmentation-aware count**: carriers = distinct org rows that
     carry an identifier OR a country (provisional NULL/NULL rows are one
     entity's echo, not evidence of a shared name) — cheap, one SQL change in
     `GENERIC_KEY_SQL` and the census's probe, epoch-neutral; or
   - **(b) per-scope caps**: a higher cap where the fragmentation rate is
     measured high (GR), the wall's semantics untouched elsewhere.
   (a) is the honest fix if the measurement says the carriers are echoes; (b)
   is the safe fallback if the identifier-bearing rows alone still cross 20
   (`ΣΤΑΘΕΡΕΣ ΣΥΓΚΟΙΝΩΝΙΕΣ`, 28 distinct identifiers, would).

## Done when

- the measurement is on this issue with the 375-key breakdown;
- the chosen fix is deployed, `org_match_keys` rebuilt if the key changed, and
  the census's GR `agree-generic` count is back near 4;
- the E0 dry plan grows by ~+20 GR groups, and a 20-sample of them reads clean.

## Side finding resolved (05:5x UTC audit probe)

The `ΓΕΝΙΚΟ ΝΟΣΟΚΟΜΕΙΟ ΣΕΡΡΩΝ` satellite on org 311 is one mention (notice
24389579, section ORG-0001) that published the hospital's name with the
authority's code `1000.E00961.0001` — a source-side slip, not a resolver error.
Org 311 itself is sound: 11,861 mentions, 182 name spellings that are all the
Single Public Procurement Authority (ΕΑΔΗΣΥ) or its predecessor ΑΕΠΠ, 10 raw
identifier spellings of one code, and it sits in ORG-0002/0003 on nearly every
Greek notice because it is the appeals body every notice must name. Two things
this tells the GR arm question: the "14-character authority code" class is the
**Greek public-sector e-invoicing code** (`Κωδικός Ηλεκτρονικής Τιμολόγησης`,
shape `NNNN.ENNNNN.NNNN`, dots dropped by the normaliser), typed by hand per
notice — hence the five variants of one authority's code — and the raw values
carry label prefixes (`Κωδικός Ηλεκτρονικής Τιμολόγησης Ε.Α.ΔΗ.ΣΥ.: …`) of the
issue-328 shape that the label-prefix repair's DE-centric lexicon does not strip.

## Measurement (job 645, 2 s): the class is echo, not Greek

| scope | agree-generic | echo | shared |
| --- | --- | --- | --- |
| DE:vat | 329 | **280** | 49 |
| GR:national | 22 | 15 | 7 |
| DE:national | 14 | 10 | 4 |
| LT:national | 9 | 9 | 0 |
| AT:vat | 1 | 1 | 0 |
| **all** | **375** | **315 (84%)** | 60 |

Over the 200 probed keys: echo keys have a median 44 carriers (max 1,000, the
probe's bound) but a median **5** with an identifier and 4 distinct identifiers;
shared keys a median 63 carriers, 28 identified, 26 distinct identifiers. The
echo sample is `stadt burghausen` (157 rows, 7 identified), `ricoh deutschland
gmbh` (115 / 18), `t systems international gmbh` (84 / 15), `hexal ag` (70 /
10), `rhein main verkehrsverbund gmbh` (35 / 4): prominent single entities whose
name-only mentions have been minting provisional rows for years. Even the
*shared* sample is mostly one entity with many identifier spellings — `kone
gmbh` (63 / 28, 26 distinct), `siemens healthineers ag` (30 / 29 / 27), `drees
sommer se` (69 / 57 / 55), `technische universität darmstadt`, `landratsamt
ortenaukreis` (328 / 21 / 19) — branch tax numbers, HRB vs VAT, and mangles,
not 26 different Kones.

**So issue 332's "99.5% genuinely shared" does not transfer to this class.** 332
measured over-cap keys corpus-wide, where the truly generic tokens dominate;
the `agree-generic` groups are, by construction, entities prominent enough to
hold the same identifier twice — and prominence is exactly what inflates a
carrier count. The wall measures how often a name was published without a
usable identifier, which for a big supplier or a city is "often".

## Decision: fix (c) — scope the correction to what E0 decides

For an E0 group the members ALREADY share an exact `(country, kind,
identifier)` triple; the name rule's job is only to keep the Organschaft and
Land-VAT shapes out, and those keep their own numbers elsewhere — they show up
as *shared* keys (over-cap identified carriers), never as echoes. So the E0
rule (4b) now runs the breakdown when the carrier count is over the cap and
admits the group if the identified carriers are under it (`admitted_echo`),
denying only shared keys. Expected: plan 1,873 → ~2,190; the newly admitted
groups get their own precision sample before any wet run (which is still
awaiting Lennart on 329).

Fix (a) corpus-wide — changing `GENERIC_KEY_SQL` itself — would also move the
R3 rescue and the resolver's prevention hook, where the evidence is name-only
and an echo of "caritas" across thirty different Caritas bodies IS a shared
name. That needs its own measurement and is issue 350; the wall stays as it is
there.
