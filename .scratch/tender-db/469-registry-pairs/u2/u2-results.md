# Issue 469 U2 — the register-sample read (2026-10-11)

Workflow `wf_62179bde-1b3`:
- five fetchers built register caches (`registers/*.json`);
- a reviewer and an independent challenger read every one of the 124 `registry_sample` pairs of job 2163, using
  `rubric-u2.md`;
- the join is `u2_synth.py`, giving one record per pair in `u2-read.json` with mention mass, roles, 448-survivor
  status, both register verdicts, the verdict and the class.

## Register fetchers (all worked from the agent container)

| Scheme | Source | Method |
|---|---|---|
| UKPRN | UKRLP, `https://ukrlp.education.gov.uk` | Plain GETs with no token: `/search/provider/<UKPRN>` (legal name, trading names, address) plus `/search?SearchTerm=<UKPRN>&UKPRNStatusFilter=active` for status |
| CHC | Charity Commission bulk extract | JSON `publicextract.charity.zip` (57 MB; name, status R/RM, `charity_company_registration_number`, linked charities) and `publicextract.charity_other_names.zip` |
| MPR | FCA Mutuals register | Register-wide CSV `https://fcastoragemprprod.blob.core.windows.net/societylist/SocietyList.csv` (7.3 MB, 32,449 rows; `Full Registation Number`, status, Act), plus the per-society details pages for transfers and renames |
| SC | OSCR | `https://www.oscr.org.uk/download/charity-register` (zip of CSV) |
| NIC | CCNI | Details pages |
| NHS | NHS ODS ORD API | `https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/<CODE>`: role, organisation vs site (Rels), successors |

Companies House pages were used for the cross-references (`registers/companies-house-extra.json`).

## Results

Reviewer and challenger agreed on all 124 pairs, and on 121 of them both said HIGH. The three MEDIUM pairs are CHC 1091035 Chartered Management Institute (a merge: the PPON is the charity's; one award names its services company), NHS 06H and NHS PWNJ-1991-NGDW.

| Scheme | Pairs | merge | keep | HIGH (agreed) | Register-confirmed | Classes |
|---|---|---|---|---|---|---|
| UKPRN | 30 | 29 | 1 | 30 | 28 | 1 consortium-agent, 1 mislabelled |
| CHC | 30 | 27 | 3 | 29 | 24 | 2 consortium-agent, 1 different-body, 3 mislabelled, 1 renamed |
| MPR | 30 | 24 | 6 | 30 | 22 | 6 consortium-agent, 2 renamed, 1 transferred, 1 mislabelled |
| SC | 2 | 2 | 0 | 2 | 2 | — |
| NIC | 2 | 2 | 0 | 2 | 2 | 1 is a triple (Choice Housing: the PPON holder is 448 survivor NP000408) |
| NHS | 30 | 28 | 2 | 28 | 26 | 2 site-vs-trust, 1 mislabelled (a PPON in the NHS field), 1 polluted name |

## Stop rules

### S2 passes: one repeated wrong class, already structural

`consortium-agent` appears 9 times. All 9 are ONE PPON, `PJXZ4423NDBT` (Northern Housing Consortium Ltd, CH
04361009). NHC publishes framework call-offs for members and states each member's register number beside its own
PPON:
- CHC-labelled 17434R Moat and 28687R Together;
- MPR 28414R Pioneer, 30938R Riverside, 31211R Flagship, 31419R South Lakes, 7522 Thirteen and 7711 Bernicia;
- UKPRN 10002639 Gateshead.

Two gates in the design already deny the class: cross-register (gate 8, one PPON partnered with two identities of one
scheme or with two schemes) and the consortium name veto (gate 3, "Consortium"). U4's tests must pin it in both
forms: one consortium PPON with two members of one scheme, and one with members of two schemes.

No other wrong class repeats. The other keeps are:
- `different-body` once: CETRA Housing Co-operative 28258R beside Capital Service Solutions Ltd, a managing agent. The
  value is mislabelled, so the shape gate refuses it.
- `site-vs-trust` twice, both NHS (see U7).

### S3 passes

- After the structural gates (registry-shape, triple, cross-register), 78 of the 94 non-NHS pairs are plannable.
- 77 of those 78 are register-confirmed by both readers (99%; the bar is ≥ 50%).
- The keep rate among plannable pairs is 0 of 78, and 0 of 1 among the unconfirmed (448's bar is 8.5%).
- The one unconfirmed pair is Thrive Homes 30398R. It reads `inactive` because its engagements were transferred to
  30183R Watford Community Housing Trust; it is a true pair.
- The renames are register-confirmed and true: Family Lives → Coram Family Lives; Metropolitan Housing Trust →
  Metropolitan Thames Valley; Chelmer Housing Partnership → Delta Housing (2026-03-31).

So S3's fallback ("nothing merges except register-confirmed pairs") is NOT triggered, and HIGH verdicts with a
challenger may admit unconfirmed pairs as designed. **U3 may start.**

## Inputs for U3 (per-scheme shapes, from U0 and U2)

- **UKPRN:** 8 digits starting with 1. `10000018991` (11 digits) is refused; the school's real UKPRN is 10075272.
- **CHC:** 6 digits `[2-9]\d{5}` or 7 digits `1\d{6}`.
  - `2204085` is a Companies House number missing its leading zero (SLM 02204085) and must fail the shape.
  - R-suffixed values (`10433R`, `17434R`, `28258R`, `28687R`) are FCA society numbers published as CHC and are
    refused.
  - A linked-charity suffix (`-1`) is refused, as decided in U0.
- **MPR:** `\d{1,5}R` (old Acts), `RS\d{6}` (new), and bare 4-digit values (`7522`, `7643`, `7711`, `7773`, `7814`,
  `7843`, `8112`, `8983`, `9518`).
  - Bare values are the RS series without its prefix: the FCA has `7522` as Thirteen Housing Group (RS007522),
    while `7522R` is an unrelated, deregistered allotment society. So bare digits must never be read as `…R`.
  - Bare and `RS00…` spellings of one society mint different identities; that is issue 514's.
  - `ip6243r` is a typo of `IP16243R` (Hastoe, FCA 16243R) and is refused.
- **SC:** `SC0\d{5}`.
- **NIC:** CCNI numbers are `1\d{5}`. Published as `NIC 100095` and `NI 100770` (Coalisland Training Services Ltd,
  CCNI 100770). U0 froze the prefix spellings.
- **NHS:** see U7.

## Inputs for U7 (NHS), checked against ODS

- 14 of 30 codes mint no identity: letter-only `QGH` / `QMM` / `QOC` / `QPM` / `QUA` / `QVV` / `QWU`, and the trusts
  `RBD` / `RBT` / `RBV` / `RCB` / `RCD` / `RCF` / `R0B` whose register org carries no identifier. These are issue
  513's.
- Of the 16 that mint, 2 are keeps of the `site-vs-trust` class:
  - `06H` is a SUB ICB LOCATION (an old CCG code, now `NHS CENTRAL EAST ICB - 06H`) beside the Cambridgeshire &
    Peterborough ICB's PPON.
  - `NCS04` is an NHS Property Services SITE (Regent House) beside NHS Property Services' PPON.
- **Shape cannot separate site from organisation.** `8HN02` (CQC, NON-NHS ORGANISATION), `A0V4D` / `H6A5H` (NHS
  SUPPORT AGENCY) and `G6V2S` (an NHS TRUST) are 5-character organisation codes, just like the site `NCS04`.
- So U7's confirmation must read ODS: Organisation level, Active, name matching. A shape gate alone is not enough.
  The 3-character `\d\d[A-Z]` CCG / sub-ICB form may become a structural deny.
- One value is a PPON pasted into the NHS field (`PWNJ-1991-NGDW`, the Business Services Organisation's) and is
  refused by shape.
