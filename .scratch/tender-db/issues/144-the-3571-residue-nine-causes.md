# Diagnosis 4: the 3,571-row eForms residue after the eight parser fixes (rev 0dfe428)

Status: closed-diagnosis (historical record) — wave 4 (nine causes incl. cause F, which became
ADR-0010 and later issue 268's rounding reversal); acted on, counted in the ledger entry

Method: same as issues 141/142/143. Fifteen outstanding members sampled
stratified from prod quarantine (`reason='unknown-customization'`,
`reprocessed_at IS NULL AND skipped_at IS NULL`) across three fetches (26 =
TED monthly 2024-04, 446 = TED monthly 2025-01, 426 = DÖE monthly 2024-06),
extracted from the archive tars/zip (one extraction pass per archive), and fed
through `ingest::profile::dispatch` → `ingest::process::parse_payload` via
`crates/ingest/examples/diag39.rs` in this clone (/tmp/diag-39d, HEAD =
0dfe428 — all eight prior fixes are in).

Headline: the structural-quirk era is over. 8 of 15 samples fail on **values**
(`unrepresentable-value`), not structure, and the dominant single cause is
sub-cent amount precision — a deliberate representation policy, not a parser
gap. The rest is a long fragmented tail: seven further causes with 1–2 samples
each. No sample PARSED (the residue is genuinely outstanding at 0dfe428), and
no cause repeats issues 141–143's fixed causes.

## Per-SDK split of the 3,571 (prod, chunked grouped query, sums exactly)

| profile | outstanding | share | reclaimed by the 8 fixes |
|---|---|---|---|
| eforms-sdk-1.7 | 1,706 | 47.8% | 41,027 → 1,706 |
| eforms-sdk-1.10 | 615 | 17.2% | 896 → 615 |
| eforms-sdk-1.8 | 589 | 16.5% | 17,439 → 589 |
| eforms-sdk-1.9 | 241 | 6.7% | 1,693 → 241 |
| eforms-de-1.1 | 142 | 4.0% | **142 → 142 (zero)** |
| eforms-sdk-1.11 | 118 | 3.3% | 573 → 118 |
| eforms-de-1.2 | 99 | 2.8% | **99 → 99 (zero)** |
| eforms-sdk-1.6 | 29 | 0.8% | 1,130 → 29 |
| eforms-sdk-1.0 | 29 | 0.8% | **29 → 29 (zero)** |
| eforms-sdk-1.2 | 3 | 0.1% | **3 → 3 (zero)** |

(Nine 500K-id chunks after the un-chunked aggregate 408'd once, not retried;
totals add to 3,571 exactly. The zero-reclaim strata are consistent with the
diagnosis below: their causes are value-shaped, which the eight structural
fixes never touched.)

## Per-sample verdicts (harness, real member bytes, at 0dfe428)

| member (fetch, q-id) | SDK | lang | outcome (reason: gist) | cause |
|---|---|---|---|---|
| 20240412_073/00218183 (26, 3340577) | 1.7 | DEU | unrepresentable-value: BT-161 amount `234.184` | F |
| 20240408_069/00205992 (26, 3343331) | 1.8 | DEU | unrepresentable-value: BT-711 amount `200.6969` | F |
| 20250115_010/00026809 (446, 3892360) | 1.11 | — | unrepresentable-value: BT-161 amount `186.821` | F |
| doe 48b76669 (426, 4182920) | de-1.1 | DEU | unrepresentable-value: DE1 amount `336.134` (file also holds `336.13445`) | F |
| doe 5e5647bf (426, 4182877) | de-1.2 | DEU | unrepresentable-value: DE1 amount `133.186` | F |
| doe 8023db5c (426, 4180807) | de-1.2 | DEU | unrepresentable-value: DE1 amount `133.698` | F |
| doe f7b76488 (426, 4177461) | de-1.1 | DEU | unrepresentable-value: `not a decimal amount: .0` | G |
| 20240412_073/00217450 (26, 3342504) | 1.10 | POR | unrepresentable-value: BT-44 RankCode `_DEFAULT_VALUE_CHANGE_ME_` | H |
| 20250115_010/00028720 (446, 3891975) | 1.10 | DEU | unrepresentable-value: UBL-TenderValidityDeadline `no zone offset: 2025-09-09` | I |
| 20240412_073/00216776 (26, 3339833) | 1.7 | DEU/AUT | ambiguous-field: AwardCriterionParameter/ParameterCode ∈ [BT-5421,5422,5423] | J |
| 20240430_085/00257446 (26, 3406127) | 1.7 | DEU | same | J |
| 20240417_076/00226907 (26, 3390924) | 1.6 | SLV | ambiguous-field: CER/ExecutionRequirementCode ∈ [BT-736-Part,743,744,764] | K |
| 20240408_069/00206158 (26, 3343316) | 1.8 | DEU/AUT | unclaimed-content: ProcurementAdditionalType/ProcurementTypeCode | L |
| 20240412_073/00216080 (26, 3341365) | 1.9 | ITA | unclaimed-content: TQR/cbc:Description | M |
| 20250115_010/00026738 (446, 3892647) | 1.10 | DEU | unclaimed-content: Lot ProcurementLegislationDocumentReference | N |

Stored `detail` on all 15 rows is quarantine-time stale ("no vendored SDK
metadata for …"), as in every prior round.

## Causes, with minimization proofs (each flips ONE member to PARSED with ONE targeted edit)

### F (6/15; in 5 strata across BOTH sources): sub-cent amount precision — by-design quarantine, representation policy

Amounts with >2 fraction digits (`234.184`, `200.6969`, `336.13445`, …).
`value.rs::amount` stores integer cents exactly and rejects more than two
fraction digits deliberately; the store schema comment (store/lib.rs ~250)
says the same: "quarantined, never rounded". One sample carries the same
amount at two precisions (`336.134` / `336.13445`) — genuinely computed
sub-cent values (net-from-gross division shapes), not thousands separators.
Minimization: truncating the single offending amount to two digits flips
00218183 to PARSED (141/616) and doe 8023db5c to PARSED (37/122) — one
offender per sampled file, both TED and DÖE pipelines.
**Verdict: NOT a parser bug — correctly quarantined under the current,
twice-documented representation decision.** Closing this share of the tail
requires a policy change (sub-cent minor units, lexical storage, or a
documented rounding rule), i.e. a CONTEXT.md/ADR decision, not a gap-fill.
This is the tail's biggest lever (6/15 sampled; spans sdk-1.7/1.8/1.11 and
both DE profiles — plausibly ~40–50% of the 3,571, bounds wide at n=15).

### G (1/15): amount `.0` — XSD-valid decimal, parser rejects the empty integer part

`<cbc:EstimatedOverallContractAmount currencyID="EUR">.0</...>` (German DÖE
toolkit; sibling lots write plain `0`). `amount()` requires a non-empty whole
part, but `.0` is a lexically valid xs:decimal worth exactly 0.00 —
representable with no policy question.
Minimization: `.0` → `0` flips f7b76488 to PARSED (31/112).
**Fix direction (mechanical, low-risk): treat an empty whole part as 0 in
`value.rs::amount`.**

### H (1/15): BT-44 RankCode `_DEFAULT_VALUE_CHANGE_ME_` — template placeholder junk

A Portuguese platform ships a Prize block whose RankCode is an unfilled
template token (the ValueAmount sibling is real). Non-numeric, unrecoverable.
Minimization: deleting the RankCode element flips 00217450 to PARSED (18/78).
**Verdict: genuinely-malformed publisher content — keep quarantined per
ADR-0004** (a documented-skip policy for placeholder tokens is possible but
not warranted at this observed scale).

### I (1/15): zone-less TVP EndDate — fallout of the 0dfe428 UBL-TenderValidityDeadline fix

`<cac:TenderValidityPeriod><cbc:EndDate>2025-09-09</cbc:EndDate>` (no zone
offset), 26 occurrences in the member. Issue 143's cause-B fix made the leaf
claimable; the claim now succeeds and dies one layer later in the strict
timestamp parse (`no zone offset`). The fix moved the failure point from
`unclaimed-content` to `unrepresentable-value` for publishers who also omit
the offset. A profile-scoped precedent exists: `value.rs::offset_or_utc`
reads offset-less SDK01- dates as UTC.
Minimization: adding `+01:00` flips 00028720 to PARSED (303/1720).
**Fix direction: extend the offset-or-UTC relaxation to the
UBL-TenderValidityDeadline leaf (or all UBL- gap-fill dates) — the gap-fill
already concedes the SDK's shape isn't what publishers send; insisting on the
SDK's offset discipline for it is inconsistent.**

### J (2/15; 2/3 of the sdk-1.7 sample): AwardCriterionParameter/ParameterCode without `@listName` — BT-5421/22/23 with the discriminator dropped

`<efac:AwardCriterionParameter><efbc:ParameterCode>per-exa</efbc:ParameterCode><efbc:ParameterNumeric>100</efbc:ParameterNumeric></...>`
— every minor discriminates BT-5421/5422/5423 by
`ParameterCode/@listName='number-weight|number-fixed|number-threshold'`; the
attr-less form (Austrian vemap-platform notices, both samples) matches none →
relaxed by name → three candidates → `ambiguous-field`. Same
dropped-discriminator class as the fixed BT-773 TermCode (143 cause D).
Minimization: deleting the block flips 00216776 to PARSED (25/137); adding
`listName="number-weight"` ALSO parses (26/139) — the content is claimable.
**Fix direction: gap-fill claim of the attr-less ParameterCode (+ its
ParameterNumeric sibling) — as the BT-5421 weight shape (`per-exa` + numeric
is weight-shaped) or a `UBL-AwardCriterionParameterCode` leaf.**

### K (1/15): CER listName typo `reserved-executionn` — publisher typo, in no SDK

One Slovenian sdk-1.6 notice writes `listName="reserved-executionn"` (double
n) for BT-736. Unlisted listName → no exact predicate → relaxed → four
candidates → `ambiguous-field`.
Minimization: fixing the typo flips 00226907 to PARSED (13/88).
**Verdict: borderline. The honest reading is malformed publisher content
(keep per ADR-0004); the sdk-1.6 stratum is only 29 rows. Note the pattern:
the permission/conditions fixes claim CER listNames one at a time — each new
unlisted listName re-quarantines. If more surface, the real fix is a policy
decision on unknown CER listNames generally, not another single claim.**

### L (1/15): `@listName='eforms-contract-nature'` — codelist file-id written instead of the listName

Austrian (vemap) sdk-1.8 notices write BT-531's ProcurementTypeCode with
`listName="eforms-contract-nature"` — the TED genericode *file* name — where
every minor requires `listName='contract-nature'`; the values (`supplies`,
`services`) are legitimate BT-531 codes. Two occurrences per member
(procedure + Lot).
Minimization: renaming the listName flips 00206158 to PARSED (27/137).
**Fix direction: claim
`ProcurementAdditionalType/ProcurementTypeCode[@listName='eforms-contract-nature']`
as a gap-fill (BT-531's content under a mutated discriminator) — same shape
as the permission-quirk claim.**

### M (1/15; the only sdk-1.9 sample): BT-76 legal-form text in `TQR/cbc:Description` — wrong element name

An Italian notice puts the company-legal-form free text in
`cac:TendererQualificationRequest/cbc:Description` next to a (claimed)
`cbc:CompanyLegalFormCode`; every minor spells that text
`cbc:CompanyLegalForm`. The predicate-free TQR EXTRA branch (home of the
existing UBL-CompanyLegalFormCode/Form gap-fills) exact-matches the block but
has no direct Description leaf → `unclaimed-content` — the issue-143 cause-E
mechanism exactly.
Minimization: deleting the Description flips 00216080 to PARSED (18/124).
**Fix direction: one text leaf (`TQR/cbc:Description`, e.g.
`UBL-CompanyLegalFormDescription`) on the already-existing predicate-free TQR
branch in `EXTRA`.**

### N (1/15): Lot-level `ProcurementLegislationDocumentReference` — the eForms-DE shape on an sdk-1.10 TED notice

German TED notices declaring plain `eforms-sdk-1.10` carry Lot-level
`cac:TenderingTerms/cac:ProcurementLegislationDocumentReference/cbc:ID`
(`vob-a-eu`, 16 blocks) — a home no SDK minor declares (BT-01(c) is
procedure-level only) but which the vendored **eForms-DE** inventory declares
verbatim (`DE1-ProcurementProjectLot-TenderingTerms-ProcurementLegislationDocumentReference-ID`).
The German toolchain emits its national tailoring onto the EU customization —
the BT-803/OPT-060 "construct from another inventory in the vendored line"
class across *dialects* rather than minors.
Minimization: deleting the 16 blocks flips 00026738 to PARSED (94/431).
**Fix direction: gap-fill the Lot-level PLDR `cbc:ID` (+
`cbc:DocumentDescription`) mirroring the DE1 fields, as `UBL-` leaves on the
sdk profiles.**

## Sizing (honest: 15 members bound, they do not size)

- Cause F appears in 5 of 8 sampled strata and both pipelines (TED XML and
  DÖE zip) — the only cross-cutting cause. Point estimate 40–50% of the
  3,571; supportable claim: "F is the dominant cause and the only one present
  in both sources".
- sdk-1.7 (1,706): 2/3 J + 1/3 F, all DEU/AUT — the Austrian platform
  (vemap) is the plausible bulk emitter; J plausibly the largest structural
  residue (point estimate ~1K, wide).
- sdk-1.10 (615): three samples, three different causes (H, I, N) — this
  stratum is heterogeneous; no dominant cause claimable.
- de-1.1/de-1.2 (241): 3 F + 1 G, zero reclaim from the eight structural
  fixes — consistent with an all-value-cause stratum.
- sdk-1.9 (241): M on 1 sample; sdk-1.6 (29): K on 1 sample — single-sample
  strata, cause coverage not established.
- Not sampled at all: sdk-1.0 (29; 9 of them in quarantine ids <500K, i.e.
  the oldest era), sdk-1.2 (3). Undetermined.
- Corpus-wide shares cannot be read from the DB (stored `detail` is
  quarantine-time stale on all 15 sampled rows). Exact sizing needs a bulk
  re-parse of the outstanding members' archive bytes.

## Fix classes (direction only, none implemented)

| cause | class | verdict | est. impact |
|---|---|---|---|
| F | sub-cent amounts vs integer-cents representation | KEEP (by design) unless a representation ADR changes policy | plausibly ~40–50% of 3,571 |
| G | XSD-valid `.0` rejected | FIX: empty whole part = 0 in `amount()` | small (de-1.1 sample) |
| H | template placeholder in BT-44 | KEEP per ADR-0004 | small |
| I | zone-less date on our own gap-fill leaf | FIX: offset-or-UTC for UBL- gap-fill dates (SDK01 precedent) | part of 1.10's 615 |
| J | dropped `@listName` on BT-5421/22/23 | FIX: gap-fill claim (BT-773-fix shape) | plausibly ~1K of 1.7's 1,706 |
| K | listName typo (`reserved-executionn`) | KEEP (or a general unknown-CER-listName policy if more surface) | ≤29 (1.6 stratum) |
| L | codelist file-id as listName (`eforms-contract-nature`) | FIX: gap-fill claim as BT-531 content | part of 1.8's 589 |
| M | BT-76 text in wrong element (`TQR/Description`) | FIX: one text leaf on the existing predicate-free TQR branch | part of 1.9's 241 |
| N | eForms-DE Lot construct on EU customization | FIX: gap-fill Lot PLDR ID/Description mirroring DE1 fields | part of 1.10's 615 |

If every FIX lands and F/H/K stay quarantined, the expected steady state is a
tail consisting of F (documented representation policy) plus small
honest-keep residues — at which point the quarantine is doing exactly its
ADR-0004 job and the reclaim campaign can be declared complete, with F's
share explicitly owned by the amounts-as-cents decision.

## Provenance

- Prod reads: bounded `/v1/sql` SELECTs (id bounds; 9+4 chunked grouped
  counts; 11 sample-row selects; v_fetches). One un-chunked grouped aggregate
  hit the 10s cap (408, not retried). Three archives opened —
  /data/archive/ted/monthly/2024-04.tar, /data/archive/ted/monthly/2025-01.tar
  (one outer pass extracting 5 inner daily tar.gz, then one pass per inner
  tar.gz), /data/archive/doe/monthly/2024-06.zip (one unzip invocation);
  /root/diag4 cleaned up. No DB writes, no service changes.
- Harness: /tmp/diag-39d (fresh clone, HEAD 0dfe428) +
  crates/ingest/examples/diag39.rs copied unchanged from the issue-141/142/143
  clones; built and run in the repo dev shell. Sample bytes:
  /tmp/diag-39d-samples/, minimized variants /tmp/diag-39d-samples/min/.
- Mixed provenance note: the doe members come from a zip (bare UUID member
  names), the TED members from nested monthly→daily tars; both were fed to
  the harness as raw bytes exactly as the reprocess would read them.

## Comments

**2026-08-10 (orchestrator)** — the cause-F policy question this diagnosis left open is decided:
ADR-0010 confirms integer cents; F is a documented keep. Terminal relabel pass + dashboard
disclosure are owned by issue 184.

**2026-08-11 (orchestrator)** — the end-state this diagnosis defined is reached: after the six
mechanical fixes (825dd64) and the terminal relabel pass (job 608), the tail is F-dominated
(1,661 sub-cent keeps, owned by ADR-0010 and disclosed on the ledger) plus named small residues
(206 unclaimed-content, 23 duplicate-section-id, 14 ambiguous-field, 3 sdk-1.2 unknown).
The reclaim campaign is declared complete. Issue 184 has the close-out numbers.
