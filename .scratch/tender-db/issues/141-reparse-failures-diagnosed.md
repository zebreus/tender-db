# Diagnosis: why the sdk-1.3 / sdk-1.7 quarantine tails fail to re-parse (task 39 / issue 140)

Method: 10 members sampled from prod quarantine (3 fetches), extracted from the
raw archive tars, and fed through the exact reprocess path
(`ingest::profile::dispatch` → `ingest::process::parse_payload`) via
`crates/ingest/examples/diag39.rs` in this clone. Each verdict below is from
that harness, on the real member bytes.

## Per-sample verdicts

| member (fetch, period) | SDK | CustomizationID in XML | outcome | exact error |
|---|---|---|---|---|
| 20221115_220/00634805_2022.xml (43, 2022-11, RECLAIMED contrast) | 1.3 | `eforms-sdk-1.3` | PARSED (22 sections / 133 values) | — |
| 20221115_220/00634806_2022.xml (43, 2022-11, RECLAIMED contrast) | 1.3 | `eforms-sdk-1.3` | PARSED (14/95) | — |
| 20230602_105/00331117_2023.xml (36, 2023-06, outstanding) | 1.3 | `eforms-sdk-1.3` | QUARANTINED | `unclaimed-content`: unclaimed element at `.../efext:EformsExtension/efbc:TransmissionTime` |
| 20230602_105/00331135_2023.xml (36, outstanding) | 1.3 | `eforms-sdk-1.3` | QUARANTINED | same (`efbc:TransmissionTime`) |
| 20230602_105/00331145_2023.xml (36, outstanding) | 1.3 | `eforms-sdk-1.3` | QUARANTINED | same (`efbc:TransmissionTime`) |
| 20240723_142/00440077_2024.xml (23, 2024-07, outstanding) | 1.7 | `eforms-sdk-1.7` | QUARANTINED | `ambiguous-field`: `.../cac:ContractExecutionRequirement/cbc:ExecutionRequirementCode` could be any of [BT-736-Lot, BT-743-Lot, BT-744-Lot, BT-764-Lot, OPT-060-Lot] |
| 20240723_142/00440233_2024.xml (23, outstanding) | 1.7 | `eforms-sdk-1.7` | QUARANTINED | same |
| 20240723_142/00440266_2024.xml (23, outstanding) | 1.7 | `eforms-sdk-1.7` | QUARANTINED | same |
| 20240723_142/00441060_2024.xml (23, RECLAIMED contrast) | 1.7 | `eforms-sdk-1.7` | PARSED (51/225) | — |
| 20240723_142/00441675_2024.xml (23, RECLAIMED contrast) | 1.7 | `eforms-sdk-1.7` | PARSED (19/103) | — |

CustomizationID strings are byte-identical between failing and reclaimed
members of the same minor (`eforms-sdk-1.3`, `eforms-sdk-1.7`; all UBL 2.3).
`sdk::resolve` maps them all correctly — **hypothesis (a), a ProfileID /
customization-string variant, is refuted** for these samples.

## sdk-1.3: hypothesis (b) confirmed — one missing field, BT-803(t)

- Every failing 1.3 member dies on exactly one element:
  `efbc:TransmissionTime` inside the TED envelope extension
  (`ext:UBLExtensions/.../efext:EformsExtension`). Removing only that element
  makes the notice PARSE (minimization done, flips to 20 sections / 117
  values).
- `fields-1.3.0.json` has `BT-803(d)-notice` (`efbc:TransmissionDate`) but not
  `BT-803(t)-notice` (`efbc:TransmissionTime`). BT-803(t) enters the vendored
  line at **1.5.0** (present in 1.5–1.15). `fields-1.0.0.json` has neither
  BT-803 half.
- The transmission stamp is written by the TED **publication pipeline**, not
  the buyer: prod shows the 1.3 rows split perfectly by period —
  2022-11 → 7/7 reclaimed (no Transmission elements in the XML at all);
  2023-05 … 2024-01 → 4,827/4,827 outstanding (TransmissionDate+Time present).
  TED evidently started stamping BT-803 into published notices between 2022-11
  and 2023-05, onto notices still declaring older customizations.
- So the 1.3 tail is "constructs from a later SDK stamped onto an
  older-minor notice" — the vendored 1.3 inventory is verbatim-correct, the
  published envelope is simply newer than the notice's declared minor.
- Likely explains the monotone SDK gradient for the *envelope* part: the older
  the declared minor, the more envelope fields postdate it.

**Fix needed (not implemented):** make the TED transmission stamp claimable
regardless of declared minor. Options: (i) claim
`efbc:TransmissionDate`/`efbc:TransmissionTime` as TED publication plumbing in
the parser (analogous to the VALUE_ATTRIBUTES carve-out in
`crates/ingest/src/eforms/parse.rs`, which exists for exactly this "published
TED carries what the SDK inventory doesn't declare" class), storing them like
BT-803; or (ii) a per-minor supplement to the vendored inventory (ADR-0002
says the fields.json files are vendored verbatim, so a supplement, not an
edit). Expected yield: the whole 4,827-row 1.3 tail, if single-cause holds
corpus-wide (verified on 3 members from one fetch; the perfect period split
supports it).

## sdk-1.7 tail: hypothesis (c) — publisher-invalid construct, in NO SDK version

- Every failing 1.7 sample dies on
  `<cbc:ExecutionRequirementCode listName="permission">` under
  `cac:ContractExecutionRequirement`. **No SDK minor 1.0–1.15 defines
  `permission` as an ExecutionRequirementCode discriminator** — `permission`
  is the codelist of BT-63 (`cbc:VariantConstraintCode`) and BT-769
  (`cbc:MultipleTendersCode`). A publisher (the samples are Spanish-language,
  `languageID="SPA"`) copies a permission-codelist entry into a
  ContractExecutionRequirement block.
- Discrimination is exact within fetch 23: failing members have
  `ExecutionRequirementCode listName="permission"` (1–2 each); reclaimed
  members carry `permission` only on its legitimate homes
  (MultipleTendersCode / VariantConstraintCode) and PARSE.
- Mechanism in the walker: no predicate matches → relaxed claim by name →
  five candidate field ids (BT-736/743/744/764/OPT-060) → `ambiguous-field`
  quarantine (parse.rs, the relaxed-ambiguity guard).
- Removing only the permission blocks makes the notice PARSE (minimization
  done: 00440077 flips to 32 sections / 176 values).
- This is NOT a vendored-inventory gap: SDK 1.15 would reject it identically.
  It cannot be fixed by vendoring anything.

**Fix needed (not implemented):** a policy decision plus a small parser (or
per-era tolerance) change — e.g. treat
`ExecutionRequirementCode[@listName='permission']` as a recognised
publisher-error construct: claim it and store it as a code under a designated
field (or record a documented skip). Silent ignoring conflicts with ADR-0004;
the VALUE_ATTRIBUTES precedent shows the accepted pattern is
claim-and-store-honestly.

**Scope caveat:** the 1.7 diagnosis rests on 3 outstanding members from ONE
fetch (23, 2024-07); the 41,031-row 1.7 outstanding tail may contain other
constructs besides the permission quirk. The failure-mode distribution cannot
be read from the DB (re-parse failures are not recorded; `detail` is
quarantine-time stale). Sizing the permission quirk's share needs a bulk
re-parse pass over the outstanding members' archive bytes.

## Contradictions with established context

- Hypothesis (a) (ProfileID/customization variant) is refuted for both minors
  sampled: the strings are byte-identical between failed and reclaimed.
- The current failure reasons are `unclaimed-content` (1.3) and
  `ambiguous-field` (1.7) — not `unknown-customization`. The stored reason and
  detail describe only the original 2023-era quarantine, as suspected.
- The 1.3 tail is not "older minors carrying constructs newer inventories
  dropped" — it is the inverse: a *newer* envelope construct stamped onto
  older-minor notices.

## Provenance

- Samples: prod quarantine rows ids 3107908/3107909/3107910 (fetch 36),
  3107527/3107528 (fetch 43), 3534918/3534933/3534942/3534867/3534876
  (fetch 23); bytes extracted from /data/archive/ted/monthly/{2023-06,
  2022-11,2024-07}.tar on the prod box.
- Harness: crates/ingest/examples/diag39.rs in this clone (/tmp/diag-39),
  built with the repo dev shell; uses the same dispatch+parse entry the
  reprocess uses (`profile::dispatch` → `process::parse_payload`).
- Prod access was read-only (bounded SELECTs via /v1/sql + tar extraction);
  no DB writes, no service changes.
