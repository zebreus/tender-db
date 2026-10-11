# Issue 514, unit 1: register-number spellings that split one register identity (census, 2026-10-11)

This unit measures standing pairs of orgs that hold two spellings of one register number, for each of issue 469's five
schemes. It reads only, using 29 bounded `/v1/sql` calls against the serving DB (none returned 408). It changes no
code. The deliverable is `splits.json`. The scripts are `census.py` (classify and group) and `finish.py` (join the
mention reads, write `splits.json`). Every response is kept verbatim in `raw/NNN-*.out`, and `raw/journal.jsonl` holds
the exact SQL, duration and HTTP status of each call.

## Per-scheme result

**In-scheme** means both orgs' stored identifiers start with the scheme's own token (`GBMPR…`, `GBNIC…`, `GBUKPRN…`,
`GBCHC…`, `GBSC…`), and both derive the same candidate number under the rules below. A **split** is two or more
distinct orgs on one (scheme, candidate).

| Scheme | Identities scanned | With a candidate | Mentions on scanned | **Splits** | Orgs in splits | Mentions in splits | Names agree (hand-read) |
|---|---|---|---|---|---|---|---|
| GB-MPR (FCA) | 135 | 134 | 1,922 | **15** | 30 | 492 | 15 same |
| GB-NIC (CCNI) | 19 | 13 | 296 | **1** | 2 | 2 | 1 same |
| GB-UKPRN | 522 | 469 | 4,188 | **5** | 10 | 295 | 4 same, **1 differ** (10007157) |
| GB-CHC (E&W) | 748 | 617 | 3,197 | **14** | 29 | 61 | 13 same, 1 partial (1179472) |
| GB-SC (OSCR) | 10 national + 1 VAT | 8 | 64 | **0** | 0 | 0 | — |
| **Total** | 1,435 | 1,241 | 9,667 | **35** | 71 | 850 | 33 same, 1 partial, 1 differ |

Every org read has `provisional = 0`.

**Merged identifiers can't be read.** `organization_merged_identifiers` is not on `/v1/sql`'s allow-list: call 20
returned HTTP 400, "not in the queryable public surface". As a proxy, every scanned org's mention raw literals were
read (calls 18-19 and 21-29). No in-scheme org carries a mention spelled as another spelling of its own number. So no
merge or bind has folded any in-scheme spelling pair yet; the 0 holds for all five schemes.

## Split shapes (in-scheme)

**MPR, 15 splits.** All 15 name one entity.
- `NNNNNR` vs `IPNNNNNR`, the Companies House spelling published under GB-MPR: 10 splits.
- Bare 4-digit vs `RS00NNNN`: 4 splits (7104 Nottingham CHA, 7528 Cheshire Peaks & Plains, 7814 Beyond Housing,
  8376 Connexus).
- Label `Reg No 30938R` (Riverside): 1 split.
- The top three by mentions:
  - 28038R Clarion: 138 + 6
  - 30938R Riverside: 116 + 1
  - 16337R Metropolitan Housing Trust: 104 + 2

**NIC, 1 split.** `NIC102575` vs `Charity Commission No NIC102575` (Radius Housing).

**UKPRN, 5 splits.** Each is a bare UKPRN vs a label-polluted one (`UKPRN 10007798`, `UKPRN: 10005790`,
`GB-UKPRN10004116`).
- The largest is 10005790 Sheffield Hallam: 182 + 1 mentions.
- **10007157 is not name-safe.** The bare org (31558903, 16 mentions, literal `GB-UKPRN-10007157`) has the head name
  "Sheffield Hallam University", but UKPRN 10007157 is The University of Sheffield, which is what the labelled twin
  says. A spelling fold behind 470's name gate must refuse this pair. Folding it ungated would put Hallam-named
  mentions on Sheffield's identity.

**CHC, 14 splits.**
- Bare vs label (`Registered Charity No.`, `Charity Number`, `no.`, `Registration No`): 9.
- Label vs label: 2 (1003906 Birmingham Mind, 1122831 Arcacia).
- Bare vs a compound of charity number and company number: 3 (1123520, 1165512, 1171446).
- The largest is 216250 Barnardo's: 9 + 1.
- 1179472 is a three-way split (Living Well Consortium ×2, Living Well UK). The rename is unconfirmed, so its names
  are "partial".

**SC: no split.**
- All 8 OSCR-shaped values are `SC` + `SC0NNNNN`, one org each.
- `SCSC314764` (APUC) and `SCSC330639` (Breastfeeding Network) are Companies House Scottish company numbers
  published under GB-SC (unmapped).
- The VAT-minted form `GBSC` + digits, i.e. GB-SC without its `SC`, is read in call 6. One row stands, `GBSC1234568`
  (Proactis Tenders), and it is a placeholder, not OSCR-shaped.

## Spelling classes (counts over the in-scheme identities)

- **MPR (135):**
  - R-suffix (old Acts): 71
  - bare 4-digit: 27
  - IP+R (CH mirror): 19
  - RS0-padded (`RS00NNNN`): 11
  - IP0-padded (`IP0NNNNN`, R inferred): 4
  - RS-suffix (`2211RS`): 1
  - label + R (`REGNO30938R`): 1
  - RSA typo (`RSA007814`, unmapped): 1
- **NIC (19):**
  - `NIC` prefix: 7
  - bare: 3
  - `NI` prefix: 2
  - label + `NIC`: 1
  - E&W CHC-shaped numbers under GB-NIC (216250 Barnardo's, 277424 Refuge, 1106452 Thurrock MIND; 1014860 is Triangle HA, a Northern Ireland body, so it is probably a CCNI number with a stray digit): 4
  - FCA `32427R` under GB-NIC (Saffron): 1
  - `FH363`: 1
- **UKPRN (522):**
  - bare 8-digit: 463
  - label-polluted: 6
  - 6-digit (DfE URN-like): 21
  - PPON pasted: 13
  - digit-count typo, a 0 dropped or doubled: 7
  - company number: 4
  - other: 8
- **CHC (748):**
  - bare: 575
  - label-polluted: 36
  - compound: 4
  - **linked suffix in the literal: 2**
  - glued digits with no suffix in the literal (CH number missing its 0, company number, UKPRN): 23
  - nonconforming: 28
  - FCA numbers under GB-CHC (R 19, bare 9, IP 9, RS 2): 39
  - 8-digit company number: 17
  - CH royal-charter `RC…`: 14
  - PPON: 5
  - 5-digit: 3
  - company-number label: 1
  - OSCR under GB-CHC: 1
- **SC (11):**
  - SC-prefix: 8
  - CH Scottish company number: 2
  - VAT-minted placeholder: 1

## Linked charities

Only **2** standing CHC identities carry a glued linked suffix in the published literal. Neither has its main charity
standing as an org.
- `GB-CHC-1099485-2` is stored as `GBCHC10994852` (Cambridgeshire Community Services).
- `GB-CHC-1169762-2` is stored as `GBCHC11697622` (Sheffield Teaching Hospitals).

So no linked-suffix split exists. Both are kept as their own `base-N` candidates and flagged `linked`.

The other 23 values whose stored digits run past a valid CHC base were checked against their mention literals. None
shows a suffix:
- `GB-CHC-2204085` (SLM) and `2115499` (Alzheimer's Society) are Companies House numbers missing their 0.
- `GB-CHC-10004599` (NCG) and `10004775` (Norwich University of the Arts) are those bodies' UKPRNs. A `GBUKPRN…` org
  stands for each of them.
- The rest are 8-digit company numbers.

Unit 2 must therefore read the raw literal to tell a linked suffix apart. The stored value cannot.

## Cross-prefix view (level B: reported, not counted as in-scheme splits)

The same candidate number also stands under other prefixes: Companies House mirrors (`GBCOHIP…`, `GBCOHRS…`),
legacy bare literals from non-FTS publishers (`IP16337R`, `RS007639`, `CHARITYNO298028`, `NIC100166`, `UKPRN…`),
and another scheme's token. Counting those adds the following.

| Scheme | Rows added | Groups with 2+ orgs | Orgs | Mentions | Names (auto, 9 hand-read) |
|---|---|---|---|---|---|
| MPR | 320 | 91 | 216 | 3,730 | 85 same, 3 overlap (29996R, 28985R, 24218R), 3 differ |
| NIC | 7 | 2 | 4 | 43 | 2 same |
| UKPRN | 1 | 6 | 12 | 300 | 5 same, 1 differ |
| CHC | 15 | 20 | 42 | 88 | 19 same, 1 partial (1179472) |
| SC | 3 | 0 | 0 | 0 | — |

**MPR is dominated by the Companies House mirror.**
- Prefix mixes over the 91 groups:
  - GB-COH + MPR: 32
  - MPR + bare: 12
  - GB-COH + bare: 10
  - all three: 9
  - involving a GB-CHC-labelled FCA number: 26
- The largest groups:
  - 7843 Anchor Hanover: `GBMPR7843` 232 + `RS007843` 10
  - 16791R Housing 21: `GBCOHIP16791R` 195 + `IP16791R` 2
  - 31693R: `GBCOHIP031693` The Guinness Partnership 176, beside Guinness Developments on GB-MPR and GB-CHC. This is
    a **differ**: a subsidiary on its parent's number.
  - 30637R Plymouth Community Homes: `GBCOHIP030637` 165 + `GBCOHIP30637R` 2
  - 7544 VIVID: `GBMPR7544` 145 + `GBCHC7544` 20
- The 3 MPR differs:
  - 31693R Guinness (above)
  - 30224R Homes Plus vs Housing Plus Group (a subsidiary on its parent's number)
  - 14044R IDS Trustees vs the Industrial Dwellings Society
- Renames read as same:
  - Affinity Sutton → Clarion
  - CHS Group
  - Sovereign Network Group
  - WKCIC → Capital City College Group (the one UKPRN level-B addition)

**The `GB:coh` arm already folds part of this.** Legacy bare `RS0NNNNN` and `IP0NNNNN` orgs carry 8 absorbed
`GB-COH-RS…` and `GB-COH-IP0…` literals, through the E1 key at crosswalk.rs:417-424. `IP` + `NNNNNR` keys nothing
there, because `coh[2..]` is not all digits, so `GBCOHIPNNNNNR` and the bare `IPNNNNNR` stay split.

**The `IP0NNNNN` → `NNNNNR` decode** (Companies House's padded spelling, R inferred) is name-corroborated in every
pair observed:
- 30441R L&Q
- 30476R EMH
- 31192R Dimensions
- 31272R Care Plus
- 31457R Leading Lives
- 30938R Riverside
- 31012R Plus Dane
- 31452R Rochdale Boroughwide
- 30637R Plymouth, which stands as both `IP030637` and `IP30637R`

It is still flagged `r_inferred`. The 31693R Guinness group is a subsidiary problem, not a decode problem.

**CHC level B adds two kinds of group.**
- CHC numbers published under GB-NIC:
  - 216250 Barnardo's, which becomes a three-way split
  - 277424 Refuge
  - 1106452 Thurrock MIND
- Legacy bare `CHARITYNO…` literals:
  - 298028 Victim Support
  - 207076 RSPB / RSPB NI
  - 222377 Mencap
  - 284670 Farleigh

## Near-misses: refused by the rules, listed with the identity they resemble

These are typos, not spellings, so the candidate rules leave them unmapped. Each has a name-agreeing holder of the
number it resembles.
- `GB-MPR-ip6243r` (Hastoe, **30 mentions**), issue 514's own example. Under the IP+R rule it maps to `6243R`, which
  is not Hastoe. Hastoe is `16243R`, held by `GBMPRIP16243R` (1) and `GBCOHIP16243R` (1).
- `GBMPRRSA007814` (Beyond Housing, 1) resembles 7814. Three orgs already hold that number: `GBMPR7814` 6,
  `GBMPRRS007814` 2, `GBCOHRS007814` 8.
- UKPRN with a 0 doubled or dropped. Each has a standing correct twin:
  - `100007158` Southampton (4) → `10007158` (74)
  - `100007167` York (1) → `10007167` (88)
  - `100000747` Blackburn College (1) → `10000747` (4)
  - `100009085` (1) → `10009085` (1)
  - `1007141` UCLan (1) → `10007141` "University of Lancashire" (21; renamed)
- Legacy lookalike or label forms:
  - `IPI6637R` Metropolitan HT (13)
  - `IPR16337R` (1)
  - `IPSNO27793` Greenwich Leisure (R dropped)
  - `IPI5355R`
  - `GBCOHIP2744OR` Creative Support (letter O)
- `GBCHC31452` Rochdale Boroughwide (3) has its R dropped, beside `GBCHC31452R` and `GBCOHIP031452`.
- `GBCHC7483` "Anchor Hanover Group" is a digit transposition of 7843 published under GB-CHC.

No bare-vs-R collision stands. No candidate group mixes `NNNN` and `NNNNR`, so the "never bare↔R" rule decided
nothing on today's data. It stays as the guard.

## Method

**Index check.** Confirmed in crates/store/src/canonical.rs before any read:
- `organizations_identity ON organizations(country, identifier_kind, identifier)`, a plain non-unique index
  (canonical.rs:12835-12836).
- `organization_mentions_org ON organization_mentions(organization_id)` (canonical.rs:455 and 12842).

**How a published literal becomes the stored value.** `normalise_identifier` (crates/ingest/src/project.rs:8656-8866)
keeps ASCII alphanumerics and upper-cases them, so `GB-MPR-RS007639` becomes `GBMPRRS007639`.
- A `GB` prefix followed by a letter run of 3 or more is not a VAT id. It becomes a `national` identity scoped by the
  mention's country (GB).
- The exception is GB-SC without its `SC` (`GBSC012345`, a 2-letter run), which mints `kind = vat`. Call 6 scans
  that range.
- `GB-NIC-NI 100770` becomes `GBNICNI100770` (national).
- No label prefix in `LABEL_PREFIXES` starts with `G`, so `GB…` literals are never label-stripped.

**Candidate rules** (census only; nothing here is a shipped key):
- **MPR**
  - Bare `[1-9]NNN` stays as is.
  - `RS` + 6 digits whose significant part is 4 digits or fewer becomes that bare number (`RS007639` → `7639`).
    `RS0` + a 5-digit significant part is left unmapped.
  - `NNNNNR` and `NNNNRS` stay as is.
  - `IP` + `NNNNNR` becomes `NNNNNR`.
  - `IP0` + `NNNNN` [R] becomes `NNNNNR`, flagged `r_inferred`.
  - **Never bare↔R.**
  - `RSA…`, `IPI…`, `IPR…`, `…O…` and R-dropped forms are near-misses (unmapped).
  - Label words are stripped first (`REGNO`, `IPSNO`, …).
- **NIC.** Strip label words (`CHARITYCOMMISSIONNO`, …). Then `NIC1NNNNN`, `NI1NNNNN` and bare `1NNNNN` all become
  `1NNNNN`. A CHC- or FCA-shaped value under GB-NIC is assigned to that scheme as cross-scheme.
- **UKPRN.** Strip `UKPRN` and `GBUKPRN`, repeatedly. Then `1` + 7 digits.
- **CHC.**
  - Strip label words, longest first.
  - In a compound, take the charity number.
  - Then `[2-9]NNNNN` or `1NNNNNN`.
  - Digits past a valid base become `base-N`, its own group, flagged `linked`, **only** when the mention literal shows
    the glued suffix.
  - A CHC value that is FCA- or OSCR-shaped is assigned to that scheme as cross-scheme.
- **SC.** `SC0NNNNN`.

**Name agreement.** An automatic token comparison over legal-form-stripped names gives `same`, `subset`, `overlap` or
`differ`. Every in-scheme split, and 9 level-B groups, were then read by hand (`MANUAL` and `MANUAL_B` in
finish.py).

## Every call (prod `echo "<SELECT>" | ssh root@zebreus.click /root/sq.sh`, sequential)

No call was a 408, and none was retried. The only error was call 20's deliberate 400. Every call took 1.6-2.5 s.

| # | Shape | Rows |
|---|---|---|
| 1 | `SELECT id, identifier, name, provisional FROM organizations WHERE country='GB' AND identifier_kind='national' AND identifier >= 'GBMPR' AND identifier < 'GBMPS' ORDER BY identifier LIMIT 10000` | 135 |
| 2 | same, `'GBNIC'`..`'GBNID'` | 19 |
| 3 | same, `'GBUKPRN'`..`'GBUKPRO'` | 522 |
| 4 | same, `'GBCHC'`..`'GBCHD'` | 748 |
| 5 | same, `'GBSC'`..`'GBSD'` (national) | 10 |
| 6 | same with `identifier_kind='vat'`, `'GBSC'`..`'GBSD'` (SC dropped → VAT) | 1 |
| 7 | national `'GBCOHIP'`..`'GBCOHIQ'` (CH mirror of FCA numbers) | 137 |
| 8 | national `'GBCOHRS'`..`'GBCOHRT'` | 37 |
| 9 | national `'IP'`..`'IQ'` (legacy bare) | 85 |
| 10 | national `'RS'`..`'RT'` | 18 |
| 11 | vat `'GBIP'`..`'GBIQ'` | 0 |
| 12 | vat `'GBRS'`..`'GBRT'` | 0 |
| 13 | national `'UKPRN'`..`'UKPRO'` | 1 |
| 14 | national `'CHARITY'`..`'CHARITZ'` | 13 |
| 15 | national `'NIC'`..`'NID'` | 7 |
| 16-17 | `SELECT organization_id, COUNT(*) FROM organization_mentions WHERE organization_id IN (<158 ids>) GROUP BY organization_id` × 2 (split members, near-misses and linked candidates; 316 orgs) | 158 + 158 |
| 18-19 | `SELECT organization_id, raw_identifier, scheme, COUNT(*) FROM organization_mentions WHERE organization_id IN (<158 ids>) GROUP BY organization_id, raw_identifier, scheme` × 2 | 194 + 165 |
| 20 | `SELECT org_id, identifier, identifier_kind, rule FROM organization_merged_identifiers WHERE identifier = 'GBMPR7639'` (PK-prefix seek) | **HTTP 400**: table not on the allow-list |
| 21-29 | the call-18 shape over the remaining 1,237 in-scheme orgs, 150 ids per call | 9 calls |

Every range read came in under the 10k cap, so no pagination was needed. The total mention rows behind calls 16-29
are about 9.7k in-scheme plus about 4.3k for the split and near-miss set. All are seeks on
`organization_mentions_org`.

## Caveats

- **Provisional orgs are invisible here.** An org whose literal the v2 gate refused is provisional with no
  identifier, so it never enters an identity-index range. The issue's `216250-1` exhibit stands as no org identity
  today.
- **Legacy bare literals were scanned only under the prefixes that carry a scheme's number**: `IP`, `RS`, `UKPRN`,
  `CHARITY…` and `NIC`. Others were not scanned: bare 6-7 digit CHC numbers, which are indistinguishable from other
  digit runs, and bare `SC0NNNNN`, which collides with Companies House Scottish numbers.
- **The snapshot is live.** The reads ran while the text reparse and project drain were running, so counts can move
  by a few.
- **Name verdicts are a heuristic plus a hand read of names only.** No register lookup was made in this unit.
  10007157 and the three level-B differs are read from the names and published numbers alone.

## Pointers for unit 2 (the per-scheme canonical form)

- **In-scheme folds that would close all 35 splits:**
  - MPR: strip `IP` from `IPNNNNNR` (10); `RS00NNNN` → `NNNN` (4); label (1)
  - UKPRN: label (5)
  - CHC: label and compound (14)
  - NIC: label (1)
  Except for 10007157 (UKPRN) and 1179472 (CHC), names agree. So the name gate must stay, as in 470: it holds back
  exactly those two splits.
- **Reach comes from the CH mirror, not the in-scheme spellings.** The MPR cross-prefix mass (91 groups, 3,730
  mentions) is about 8× the in-scheme mass. That is a register↔Companies House crosswalk for societies, nearer to
  469's issue T than to a spelling fold, and it carries the subsidiary-on-parent's-number hazard (Guinness, Homes
  Plus).
- **Linked charities need no fold.** There are 2 literals, with no main charity standing beside either.
