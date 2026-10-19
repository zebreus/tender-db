# 545 triage — unit 1 measured (2026-10-19)

Read-only. 10 /v1/sql requests (one an ssh transport failure before the query ran, rc=255, retried; no 408),
1 public GET. Plans read locally on plan.db (python sqlite3 3.45.1; the copy lacks the two prod org indexes,
so `organizations_identifier_id` and `organizations_name_norm_id` were created on it first): *.plan.txt.
Every join read is `CROSS JOIN`, driven from the identifier-index range, GROUP BY led by `o.identifier`.

## Verify (public GET /v1/tenders/8664993) — still OPEN
Tenderer and winner = 30913706 UK Research & Innovation, mention_name `Jeol (U.K.) Limited`; buyer 30914819.

## Exhibit (q1): UKRI 30913706, 12 distinct (name, scheme, raw) rows, 3,594 mentions, all PDQJ
Census-rule strays: Jeol (F), STFC (U, a UKRI council), `UK Reseach and Innovation` (V),
`... - Cabinet Office - Public Procurement` (V), `United Kingdom Research and Innovation` (V).
1 of 5 strays is the 545 class. BGS x3 and Individual x2 are not strays (held more than once).

## Populations (q2, q7)
GBPPON* orgs: 14,338 (about 790 per second letter, ids 30913554..31970150). GBCOH* orgs: 31,486.

## Slices (q3, q8): every mention name of every org in an identifier-index range; census stray rule
replayed locally (alt key from r524judge/keys.py, keys-agree approximated as equal after dropping § forms)
- PPON slice GBPPONPC..GBPPONPE: 1,577 orgs (11.0%), 991 with 2+ mentions, 72 GB-PPON strays on 58 orgs
  (+1 GB-NHS). Contains the exhibit (Jeol on UKRI).
- COH slice GBCOH04..GBCOH05: 2,091 orgs (6.6%), 1,163 with 2+, 87 GB-COH strays + 23 GB-PPON strays
  (the latter all variants: PPON mentions on altid-merged COH orgs).
- Extrapolation: about 655 GB-PPON strays on PPON-keyed orgs (census 1,162 corpus-wide) and about 1,310
  GB-COH strays on GBCOH orgs (census 1,690). The rest sit on orgs keyed some other way (bare numbers, VAT).

## Hand classification (q3_strays_judged.json; COH from q8_strays.txt)
PPON 72: V 40 (holder's own variant/rename), U 12 (sub-unit/parent/hosted/predecessor), T 8 (titles,
"DO NOT USE"), C 1, **F 11**:
- supplier-side (Tenderer) 8: Jeol/UKRI, Derek Lewis/Dewi's, Pensions Policy Institute/IFS, Healthcare 21
  NI/Primary Healthcare 21, Equal Education Partners/Torfaen, Portsmouth Hospitals Univ/NHS England,
  QCI (UK)/Placing Talent, Watkins Coaches/RCT CBC.
  Destination identified elsewhere: 5 (Jeol 31620129 GBCOH939456; Derek Lewis 30928138 GBCOH01102211;
  HC21 15757939/30915822; EEP 31582974 + 30928130; Portsmouth 30971975 GBPPONPBDT9319TTHZ).
  Provisional only: PPI 31587812, Watkins 2479435. Absent: QCI (UK).
- buyer-side 3, all Tees Valley group bodies on each other's PPON: TVCA on STDC's, STDC on TVCA's
  (both identified), Hartlepool DC on TVCA's (provisional 31610040).
COH 87: **F 6** (Bedford BC on Comensura's number as buyer, identified PPON 30914266; DDM Health
  GBCOH07975193 x2 rows; Diabetes UK PPON 31562654; Land Use Consultants GBCOH2549296 + 2 rows;
  Autocross Euroshell absent; London Cactus absent); about 5 more ambiguous (R Developments/Rosslee,
  MSS Water vs Acorn Chemical, Orton Mech vs Orton Electrical, Gowing vs Gowing & Hunt, Southern
  Landscape vs Southern Land); rest V/U.

## Gaps the stray rule does not see
A foreign name held TWICE is no stray: PPON slice had 4 such supplier-side keys (CEBR x2 on IFS,
CEBR identified 30928327/31542054; Pagabo x2 on CNTW; WAVEMAKER t/a WPP 650 x2 on MoJ; Charlton Park
Academy x2 on HM Land Registry). So the strays catch about 2 in 3 of the class's keys.
Section collision: FTS section ids are `ORG-<scheme>-<id>`, so two parties publishing one identifier
share one mention row (Bedford BC buyer + Comensura tenderer on 47300761 both bind the Bedford-named
mention).

## Sizes (order of magnitude, 17 sampled F events; Poisson-wide)
F share of GB strays: 11/72 PPON, 6/87 COH, about 1 in 10. Corpus: about 100 PPON F + 90 COH F mentions
on GBPPON/GBCOH orgs, plus the twice-held keys (about 70 mentions PPON) => roughly 150-250 mentions.
About 60% have an identified destination; destinations are often fragmented (Jeol UK 3 identified rows,
LUC 3, DDM 2, EEP 2).

## Prefilter (prefilter.py)
Drop a stray whose distinctive tokens (vote key, len>=3, minus a generic list) appear in the head or any
other name of the org: 183 strays -> 45 to review; supplier-side F recall 14/14; misses only the
Tees Valley buyer trio (share "tees").
