# TED empirical identifier checks

Empirical verification (2026-07-19) of the identifier assumptions the canonical
layer depends on (ADR-0001, ADR-0003, docs/research/eforms-data-model.md §6),
run against real TED notice chains. All checks scripted on the VPS:
scripts in `/opt/tender-db/checks/` (10_harvest_chains.sh, 20_download_chains.sh,
30_analyze.py, 31_reanalyze.py, 40/41_linkage*.sh, 50_withheld_scan.py,
51_republication.py), data in `/opt/tender-db/samples/chains/` (`_meta/` holds
all intermediate JSON).

## Method / corpus

- **Chain corpus**: Search API v3 (anonymous), query `notice-version>=02`
  Jan–Mar 2026 → 500 republished ("changed") notices → 483 distinct procedures;
  every notice of each procedure fetched via batched
  `procedure-identifier IN (…)` queries. 313/483 chains contain both a
  competition and a result notice. 116 procedures (biased toward long chains,
  multi-CAN, veat) fully downloaded: **551 notice XMLs**.
- **Daily-scale corpus**: full scans of archived daily packages OJ S 125/2024
  (3 373 files), 125/2025 (3 666), 136/2026 (3 722) — **10 743 eForms notices**.
- Search API notes: `procedure-identifier`, `notice-version`,
  `framework-agreement-lot` are valid query fields; there is **no DPS query
  field** (`dynamic-purchasing-system-lot` etc. rejected); `SORT BY x ASC` is
  rejected (only `DESC` parses); `IN (…)` batching works and is the efficient
  way to expand procedures (20 UUIDs/query stayed well under the 250 limit).

## 1. LOT-id stability — HOLDS for normal procedures, BREAKS under FA/DPS round publications

Checked: 116 procedures / 551 notices; 465 CAN LotResult→lot references
(BT-13713) against the union of the same procedure's competition-notice lot
ids; 6 true corrigendum version pairs; 31 same-notice-type version pairs; title
similarity for every lot id shared between chained notices.

- **Normal procedures: 0 violations.** 428/465 CAN lot references resolve to a
  competition-notice lot id; **all 37 failures are in one framework procedure**
  (below). Lot-id reuse with different content (title similarity < 0.5):
  10 lot instances, **all in FA procedures, 0 elsewhere**.
- **True corrigenda don't restructure lot ids**: of the 6 version pairs where
  the later version is a change notice of the same type (same BT-701,
  `efac:Changes` present), **0** changed the lot id set and **0** reused an id
  for different content. (Small n; the daily-scale BT-13716 check in §4 —
  599/599 LOT section refs naming existing lots — supports the same
  conclusion.)
- **FA call-off pattern (violation)**: procedure
  `461b9ccb-cb5a-493a-9a67-a2f01844ecb4` (HU, `fa-w-rc`): CN 769333-2023
  defines a single lot LOT-0001 = the framework itself ("Magasépítési
  munkálatok – keretmegállapodás 2023"); each of 7 subsequent CANs
  (258990-2024, 415516-2024, 626874-2024, 242043-2025, 439377-2025,
  673898-2025, 8027-2026) **redefines LOT-0001..LOT-000N as that round's
  call-off contracts** ("Balerina lakás felújítása", …). Lot ids here are
  notice-local labels, not procedure lots.
- **DPS pattern (violation)**: `839e06db-…` and `99009b5e-…` (HU DPS): 9 CANs
  each over 2024–2026, every round renumbers its purchases from LOT-0001 with
  entirely different subject matter per round. Same shape as the FA case.
- **Tranche-versioned CANs (no id violation, but version-semantics trap)**:
  18/31 same-type same-BT-701 consecutive version pairs have *different lot
  sets* — these are not corrections but award tranches: each version carries
  only the lots awarded in that round. The union of tranche lots exactly
  equals the CN's lot set (`122b7dfb-…` HU, plain multi-lot procedure: CAN v2–v7
  union = 24/24 CN lots; `5d63b431-…` IT framework: versions 4→24 union = 37/37).
  Lot ids stay procedure-scoped; what breaks is "later version supersedes
  earlier" — v(n+1) does **not** contain v(n)'s content.

**Verdict: LOT-XXXX is procedure-scoped and stable outside FA/DPS (0/~100
non-FA procedures violated; 428/428 refs resolve). Under FA/DPS award
publications (observed in HU eSender EKR data) lot ids can be per-round
labels. Import defensively: a result-notice lot section whose id/content
doesn't match the competition notice's lots must be treated as a round-local
contract description, not as a restructuring of the procedure's lots.**

## 2. Procedure linkage via BT-04 — HOLDS, with known gaps

- **XML vs API**: `cbc:ContractFolderID` == Search-API `procedure-identifier`
  in **551/551** downloaded notices. Grouping by the API field is grouping by
  BT-04.
- **Presence**: 10 422/10 743 eForms daily-scan notices carry BT-04 (97.0%).
  **Every missing one is a planning or BRIN notice** (PIN subtypes 1, 4, 5, 7,
  8, E1, T01 and X01) — 100% of competition/result notices had BT-04.
- **CAN→CN linkage rate**: 250 CANs (published 2026-05-01..07, types
  can-standard/-social/-desg/-tran): 250/250 have a procedure-identifier;
  of the 249 distinct procedures, **199 (79.9%) have ≥1 competition notice on
  TED under the same BT-04**; 45 (18.1%) are result-only (direct awards /
  negotiated without call, or competition predating TED's eForms/API
  coverage); **0 have a planning notice under the same BT-04** (planning links
  are BT-125 references, and PINs mostly have no BT-04 at all).
- **Voluntary ex-ante (veat)**: 250 sampled (Apr 2026): 250/250 carry BT-04.
  Of 100 procedures expanded: 36 also have a CAN under the same BT-04, 2 have
  a competition notice, the rest are veat-only so far. Ex-ante → award
  linkage works through BT-04 when both exist.
- **Cross-border / multi-buyer**: only 9/250 CANs had multiple buyer
  countries; 8/9 of those procedures link to a competition notice — no
  anomaly beyond small n.
- **Edge cases found**:
  - Cross-era: eForms change notice 193213-2025 references legacy CAN
    600272-2023 **by publication number**; that 2023 notice has no
    procedure-identifier in the API. Chains crossing the eForms boundary
    connect only via publication-number references.
  - **46/483 chains consist of a single v≥02 notice** — v01 was corrected
    before OJ publication and never appeared on TED. Version 01 (and
    contiguous version numbers generally) cannot be assumed to exist;
    `5d63b431-…` shows TED versions 4,5,18,20,…,24 with gaps.
- **BT-701/BT-757 are NOT a reliable logical-notice identity** (bonus
  finding): 10/116 procedures set BT-701 equal to BT-04; 8 version chains
  span *different notice types* (CN v1 → CANs v2..v7 under one BT-701, HU);
  and in `1076b709-…` each successive change notice got a **new BT-701 while
  BT-757 kept incrementing across them** (v2..v6 over 5 distinct notice
  UUIDs, all referencing the original 940-2026). The publication number
  (OPP-010) is the only universally reliable per-publication key.

**Verdict: use BT-04 as the Tender grouping key — it held in 100% of
observable cases for competition/result/veat notices. Model planning notices
and legacy-era references as linked by explicit reference (BT-125 /
publication number), not by BT-04. Key Notices by publication number; treat
(BT-701, BT-757) as evidence, not identity.**

## 3. Framework / DPS semantics — rounds are repeated CANs under one BT-04

Sample: 16 FA and 7 DPS procedures among the 116 downloaded.

- **No new procedure UUIDs per round**: 0 observed. Repeated award rounds
  appear as **more result publications under the same BT-04**, in two shapes:
  1. a separate CAN per round, each with its own BT-701 (461b9ccb: 7 CANs;
     839e06db, 99009b5e: 9 CANs each, 2024→2026);
  2. one CAN republished with incrementing BT-757 where each version carries
     only that round's lots (122b7dfb, 5d63b431 — the "tranche" pattern of §1;
     happens in plain multi-lot procedures too).
- **What a round looks like in data**: a result publication whose lot
  sections describe the round's contracts. Under the HU FA/DPS pattern the
  lots are round-local (ids reused); elsewhere they reference the CN's lots.
  One LotResult references exactly one lot; multi-contract LotResults are
  rare: **2/693** LotResults referenced more than one SettledContract.
- **BT-1252 (previous-procedure reference for direct awards) is rare and
  format-unstable**: 6 notices in 10 743 daily-scan notices used a
  direct-award-justification reference (5 UUID-valued, 1
  publication-number-valued). In the chain sample, notices 93-2026 and
  85492-2026 (proc `def685e2-…`) reference the prior CN 758595-2025 **by
  publication number**, which belongs to a *different* procedure UUID
  (`2c09df79-…`) — real cross-procedure links exist but are heterogeneous.

**Verdict: no dedicated round/mini-competition entity exists in the data and
none is needed as a first-class canonical entity — a "round" is derivable as
one result publication (or one version-tranche) under the framework's BT-04.
What IS needed: result-notice lot handling that tolerates round-local lot
ids (§1), and version handling that does not assume supersession (§1) or
contiguous version numbers (§2).**

## 4. Change notices / BT-13716 — present in only ~58%, correct when present

- **Daily scale**: 1 395 change notices across the 3 daily packages
  (408 + 463 + 524 ≈ 12–14% of each day's notices). **813 (58.3%) carry ≥1
  BT-13716 changed-section id; 41.7% carry none** (only a change reason
  BT-140/description BT-141) — the changed-section mechanism cannot be the
  sole change-scoping signal.
- **Chain sample** (224 change notices): section-id vocabulary over 647 refs:
  `LOT-NNNN` 599, `PROCEDURE` 26, `PAR-NNNN` 9, `ORG-NNNN` 7, `RESULT` 5,
  `RES-NNNN` 1.
- **Do section ids match the previous version?** All 599 LOT refs name lot
  ids that exist in the procedure's notices. The only 9 mismatches are
  **Part/Lot scheme confusion** in 2 procedures (e.g. 627625-2025 references
  PAR-0001..5 while every notice version defines LOT-0001..5) — the numeric
  part matched, the prefix didn't.
- **ChangedNoticeIdentifier** (224 values): 176 (78.6%) `UUID-vv` format,
  48 (21.4%) publication-number format; **223/224 resolve** to a notice of
  the same procedure on TED (the 1 failure is the cross-era case in §2).

**Verdict for ADR-0001 change scoping: compute change scope by diffing full
notice versions; use BT-13716 as a hint only (58% coverage), trusting its ids
(599/599 correct) but normalising LOT/PAR prefix confusion. Parse both
reference formats for the changed-notice pointer.**

## 5. Withheld fields (BT-195–198) — real but rare; republication mechanic NOT observed

- **Frequency** (daily scans): 10/3 355 (0.30%) of eForms notices in 2024,
  72/3 666 (1.96%) in 2025, 76/3 722 (2.04%) in 2026 carry `efac:FieldsPrivacy`.
  Dominant fields: `win-ten-val` (BT-720 winning value), `rec-sub-cou/typ`
  (submission statistics), `not-val`, `ten-val-hig/low`; reasons `eo-int`,
  `oth-int`, `fair-comp`, `max-val`. 0/551 chain notices had any — withheld
  fields cluster in result notices outside our chain-biased sample.
- **BT-198 (available-from date) is usually absent**: present in only
  76/848 privacy blocks (9.0%); when present, typically 5–10 years out
  (2030–2036).
- **Republication test**: all 82 archived withheld notices from the 2024/2025
  daily packages re-fetched live on 2026-07-19: **0 changed in any way**
  (same VersionID 01, byte-equivalent privacy blocks) — including the 3 whose
  BT-198 dates passed long ago (00383916-2024, due 2024-06-27;
  00430331-2025, due 2025-07-02/03; 00431184-2025, due 2025-07-03): the
  values are still withheld. No withheld-then-revealed transition was found
  across notice versions in the chain corpus either (n=39 version pairs).

**Verdict: model withheld fields as the planned generic satellite table
(entity, field id, reason, text, available-from) and treat BT-198 as
informational. Do not build a reveal-tracking mechanism: in practice TED
neither updates the published XML in place nor (observably) republishes when
the date passes; if a reveal ever arrives it will be an ordinary new notice
version and needs no special machinery.**

## 6. eForms SDK 2.0 state (side-check)

As of 2026-07-19 (github.com/OP-TED/eForms-SDK): latest stable is **1.15.0**
(2026-07-15); **2.0.0-alpha.2** was released 2026-03-26 (alpha.1 earlier),
milestone "SDK 2.0" open (2 issues), branches `develop` (1.16 line) and
`efx-2` active. The announced 2.0 scope is an **EFX-2 language overhaul**
(new template/rule grammars, `WITH…COMPUTE`, sequence types, field
`:privacy*` properties) plus "changes to the SDK metadata contents" — i.e.
it threatens `fields.json`-shaped tooling (ADR-0002's completeness checker)
more than the UBL notice structure or identifier semantics; no
identifier-model breaking changes are announced. Watch the metadata shape
when 2.0 betas land; 1.16 continues in parallel.

## Implications for tender-db

- **BT-04 as Tender key: confirmed** (551/551 XML==API; 100% presence on
  competition/result/veat). Add explicit-reference linking (not BT-04) for
  planning notices and for chains that cross the legacy/eForms boundary.
- **(BT-04, LOT-id) as Lot key: confirmed for regular procedures, needs a
  defensive rule for result notices**: when a result notice's lot sections
  are not a subset of the competition notices' lots (or ids collide with
  different content), import them as round-local award descriptions attached
  to the procedure — never rewrite canonical Lots from a result notice. The
  trigger condition is detectable (id not in CN lot set, or content
  mismatch), and in our sample occurred only in FA/DPS-style HU publications.
- **Notice identity**: store OPP-010 publication number as the Notice key;
  keep (BT-701, BT-757) as attributes. Do not assume: version 01 exists on
  TED, versions are contiguous, versions share a notice type, or a later
  version supersedes an earlier one's content (tranche CANs). "Supersedes"
  should be derived: same BT-701 + `efac:Changes` present ⇒ correction;
  otherwise treat versions as additive publications.
- **Canonical versioning (ADR-0001)**: change scoping must be diff-based;
  BT-13716 is a 58%-coverage hint with trustworthy ids (599/599) modulo
  LOT/PAR prefix confusion. Change-notice back-references need two parsers
  (UUID-vv and publication number).
- **No round entity needed** in the canonical model; rounds are derivable
  from result publications under one BT-04.
- **Withheld fields**: satellite table suffices; no reveal machinery.

## Open questions

- Are the FA/DPS round-local lot ids specific to the Hungarian (EKR) and
  similar eSenders, or broader? (All violating procedures in this sample were
  HU; the IT framework used the id-stable tranche pattern instead.) A larger
  targeted FA/DPS sweep would give a per-country violation rate.
- The 18% of CAN procedures with no competition notice on TED: how much is
  genuinely direct-award vs. competition published before July 2016 /
  pre-eForms? Needs the bulk archive (legacy notices carry no BT-04) and
  OJS-reference matching to answer.
- Do national portals (oeffentlichevergabe.de) publish the TED-missing
  intermediate versions (the 5d63b431 gaps, the never-published v01s)? If
  yes, cross-source merge (ADR-0003) will see version sequences TED lacks —
  the interleaving must key on (BT-701, BT-757) + source, not assume TED
  completeness.
- Whether a BT-198-due reveal *ever* occurs (our past-due n=3, all
  unrevealed). Re-run `51_republication.py` periodically once ingestion is
  live; it is cheap.
- BT-1252 formats (UUID vs publication number): n=8 total observed. If
  direct-award linking becomes a product feature, measure at bulk scale.
