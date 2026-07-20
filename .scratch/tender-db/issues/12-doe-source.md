# 12 — DÖE source: fetcher, eForms-DE + sdk-0.1 profiles, cross-source merge

Status: resolved
Blocked by: —

Goal: oeffentlichevergabe.de is a live second Source and German procedures
merge across Sources.

Scope:
- Fetcher: monthly + completed-day exports (eforms.zip), T+1 schedule,
  registry rows; archive under /data/archive/doe/.
- eForms-DE profile: SDK-DE deltas (+4 fields, national codelists,
  E-subtypes, DE→EU version fallback table per
  docs/research/eforms-de-profile.md); DEX satellite table.
- sdk-0.1 profile: the committed empirical path inventory as checklist
  (numeric + uuid channels); numeric-channel notices become single-notice
  Tenders; unknown new paths quarantine + inventory extension workflow.
- ADR-0003 merge: exact notice/procedure UUID equality merges DÖE+TED
  Tenders; per-field-class precedence; merge is a projection concern
  (re-projection can undo).
- Fixtures from the VPS sample months.

Acceptance: one DÖE day ingests; a known DÖE↔TED pair (e.g. 373130-2026)
resolves to ONE Tender with TED publication identity + DÖE national codes;
below-threshold islands appear as Tenders.

## Comments

2026-07-19 — The **fetcher half only** is done (`crates/ingest/src/doe.rs` +
a `doe` subcommand on the fetch CLI). No parsing, no profiles, no merge —
those remain the bulk of this issue.

Landed:
- `doe::monthly(base, year, month)` and `doe::day(base, date)` building the
  `eforms.zip` export URLs; source `doe`, kinds `monthly`/`daily`, rel_paths
  `doe/monthly/YYYY-MM.zip` and `doe/daily/YYYY-MM-DD.zip`.
- `doe::months_through(end)` — the backfill walk from `FIRST_MONTH` (2022-12).
- CLI: `fetch doe --day YYYY-MM-DD | --month YYYY-MM | --backfill`. Backfill
  skips closed months via the registry without downloading and always
  re-fetches the current month, which keeps accumulating at T+1.

Verified against the live API on the VPS (real fetches into /data/archive):
- `--day 2026-07-18` → Fetched, 1,829,595 B, 567 entries, `<uuid>-NN.xml`.
- `--month 2026-06` → Fetched, 93,887,752 B, 23,398 entries. Both counts
  match docs/research/german-portals.md exactly; ZIPs pass `testzip()`.
- Re-run of both → Unchanged; `--refetch` re-downloads and still Unchanged
  (hash match).
- 400s (today, future day, 2022-11) → `Outcome::Rejected`, exit 0, ~0.07 s.

One change outside the DÖE files was needed in `fetch.rs`: `download()`
retried *every* non-404 error three times with backoff, so a permanent 400
became a ~6 s retry loop. Client errors are now never retried, and 400 maps
to the new `Outcome::Rejected` alongside `NotFound`. This is the root fix
rather than special-casing DÖE, and it applies to TED equally.

Not done here, deliberately: a full `--backfill` run was not executed.
It is ~44 requests / ~3 GB, and german-portals.md §9 lists "undocumented
rate limits" as an open question to clear with
support@datenservice-oeffentlicher-einkauf.de first.

2026-07-20 — The **parse half** is done (branch
`worktree-agent-afd2efacf0e1784c9`): DÖE zip packages walk, and both DÖE
profile families parse into the notice-parsed layer with full claim
coverage. The cross-source merge + per-field-class precedence remain a later
projection slice, as scoped.

**eforms-de-2.0/2.1** rides the existing eForms claim system with vendored
SDK-DE `fields.json` (gitlab.opencode.de SDK-eforms-de tags 1.12.6 / 1.13.3
/ 1.14.4 → `sdk/fields-de-2.0.0.json`, `fields-de-2.1.0-eu-1.13.json`,
`fields-de-2.1.0-eu-1.14.json`). `eforms::sdk::resolve()` is the DE→EU
version map: 2.1 tracks two EU bases keyed by `cbc:ProfileID`
(`eforms-de-2.1@eforms-sdk-1.13`/`@eforms-sdk-1.14`; absent ProfileID →
1.13, the empirically dominant base), 2.0 → 1.12. SDK-DE models ProfileID
as a real field (OPT-002-notice-DET), so an inventory-declared field now
wins over an IGNORED rule; the `defext` → `german-eforms-extension`
namespace is registered for the DEX fields (still zero wild instances);
the 14 national codelists need no special handling — codes are stored
as published with their `@listName`.

**eforms-sdk-0.1** is parsed against an *empirical* era checklist,
`sdk/fields-sdk-0.1.json`: 293 leaf fields + 11 section nodes generated
from a full scan of every sdk-0.1 notice in the sample history (244,790
notice versions, 2022-12→2026-07; 464 distinct element paths — the
research's 800-file sample saw 362). Field ids are the source's own element
paths under an `SDK01-` prefix (`SDK01-ContractingParty-Party-PartyName-Name`),
extension plumbing elided; sections: Lot (identified), ContractingParty,
TenderResult, WinningParty, Change, Location, criteria/requirements.
A path outside the inventory quarantines (`unclaimed-content`); extension
workflow documented in the file's `$comment` (re-scan, diff, review,
reprocess). Notice identity = `<cbc:ID>-<cbc:VersionID>` (uuid or numeric
channel), derived from the document; empty numeric-channel
`ContractFolderID` stays valueless by design. RegulatoryDomain is an open
code domain, stored as published (11 values seen incl. de-vob/de-vol/
de-uvgo/de-hhr). Full-corpus surprises vs the research: award amounts DO
occur (PayableAmount etc., with currencyID), multi-lot/Part lots exist in
some months, and dates may lack zone offsets — read as UTC for SDK01-
fields only (eforms::value, documented).

**Verification** (VPS, release build, scratch db `/opt/tender-db/doe12/verify2.db`,
real archived packages):
- doe daily 2026-07-18: 567/567 parsed, **zero quarantines** (2.9 s).
- doe monthly 2026-06: 23,398 notices → 23,290 parsed, **zero
  unclaimed-content** (3m12s). Split: eforms-de-2.1 12,138 / de-2.0 1,651 /
  sdk-0.1 10,094 (9,178 numeric + 916 uuid) / eforms-sdk-1.0 82. Remaining
  quarantines: 82 unknown-customization (`eforms-sdk-1.0` E2/E3 stream —
  EU SDK 1.0's fields.json uses descendant-axis + boolean-or predicates our
  xpath grammar doesn't model; deliberately not vendored, documented in
  sdk.rs) and 26 unrepresentable-value (3-fraction-digit amounts, ADR-0004
  by design).
- Fixes found on real data: three withheld-discriminator publisher patterns
  (FieldsPrivacy under legislation-reference/ProcessJustification whose
  discriminator is itself withheld) → ALIASES grafts; one out-of-inventory
  `cbc:CriterionTypeCode` → EXTRA (`UBL-SelectionCriterionType`).
- ted monthly 2026-06 (plain tar of the month's daily .tar.gz files — the
  walker now splits containers by file magic and descends nested tars
  in-stream, members surfacing as `06/<daily>.tar.gz/<dir>/<file>.xml`):
  first run exposed that process_package buffered a whole package's parsed
  records (6 GB RSS, OOM-killed at ~66k notices); it now streams through a
  bounded channel — ~150 MB RSS at any package size. Final fresh-db run:
  **78,480 members → 78,480 notices, 78,257 parsed, 22m43s** (1.12: 13,060 /
  1.13: 48,006 / 1.14: 17,410 / 1.11+1.7: 4). Dedup verified: re-walking
  2,196 already-ingested records wrote nothing (all duplicates). Note the
  issue-number gotcha: daily 2026-00136 is a *July* issue, so it is not
  inside the June monthly — TED monthlies contain issues 103–124.
  Monthly-scale surfaced 3 more publisher quirks now mapped (award-criterion
  fields restated on the parent AwardingCriterion; raw `cbc:WeightNumeric`;
  a real `cbc:TenderResultCode` beside the SDK's dummy OPT-999 block in
  UBL's mandatory `cac:TenderResult`). Remaining TED-side: 52
  unclaimed-content (0.066%, a long tail of ~11 distinct one-off UBL
  elements — InvitationSubmissionPeriod/EndDate ×18 the largest; follow-up
  for the TED eforms checklist, same EXTRA/ALIASES mechanism), 167
  unrepresentable-value (sub-cent amounts, offsetless dates — ADR-0004 by
  design), 4 unknown-customization (eforms-sdk-1.11/1.7, not vendored).

Blocker found for whoever takes the parsing half (see also issue 05/14):
the production DB `/data/db/tender-db.db` can no longer be opened by current
code — `Parse error: invalid expression in CREATE INDEX: parse_state`. The
store's migrations are `CREATE TABLE/INDEX IF NOT EXISTS` only, so a DB
created before issue 03/04 never gains the new `notices` columns and then
fails on the index over the missing column. Fresh DBs are fine. Needs either
a real column-migration path or a documented recreate step. My verification
therefore registered into `/data/db/doe-verify.db`; the archive files under
/data/archive/doe/ are real and will re-register (hash-idempotent) once the
production DB opens again.

2026-07-20 — **Cross-source merge (the last slice) done** and issue closed
(commit "Issue 12: merge a procedure's TED and DÖE readings into one
Tender"). ADR-0003 realised in the projection + store:

- Keyed (BT-04) Tenders now group by the procedure key **alone**, across
  Sources — a TED eForms procedure and its DÖE twin publish the same BT-04
  UUID, so the shared key merges them. The `tenders` table is now
  `UNIQUE(procedure_key)` (legacy `ojs:` keys are TED-only, islands stay
  per-notice), and `tender_identity` finds a keyed Tender by key alone,
  keeping the primary Source label current as backfill adds the TED twin.
- Per-field-class precedence: the merged chain folds by publication instant
  with a fixed Source tiebreak (`source_rank` ted > doe), so on an equal
  instant the TED reading folds last and wins the shared eForms fields and the
  publication identity; the Tender is labelled TED. German national content
  (national-codelist codes, DEX satellites) is not a canonical fact — it is
  retained in full in the notice layer — so no fact-level DÖE override is
  needed, only retention.

Verified by the `doe-ted-pair` fixtures (shared BT-04
`1af86e3c-411f-4c2e-aacc-ecac61717472`): one Tender, TED publication
identity, both readings as versions, DÖE national codes retained. At scale
(pre-backfill dress rehearsal), the TED + DÖE monthly 2026-06 notice layers
share **12,336 BT-04 procedure keys across both Sources** — each collapses to
one Tender under this rule.

The 52-element TED long-tail follow-up noted in the parse-half comment is
also closed (commits "Issue 12/18: map the TED eForms long-tail UBL elements"
+ "Fix: complete the TED long-tail mappings"): the one-off UBL leaves are
mapped via EXTRA/ALIASES. Dress rehearsal on the fixed binary: a fresh
reprocess of TED monthly 2026-06 yields **0 unclaimed-content** (78,480
notices → 78,309 parsed; residual quarantines are 4 unknown-customization for
unvendored SDK 1.11/1.7 and 167 unrepresentable-value, both ADR-0004 by
design), and ted daily / doe daily / doe monthly / ted monthly 2014-01 /
ted monthly 2005-01 are all 0 unclaimed too. The 2005-01 text monthly
exercised the tar → daily tar.gz → zip → concatenated-records double-nesting
(965 members → 20,720 notices, 0 unclaimed, ~91 MB RSS).

Open follow-up (not a blocker for these fixes): projecting ~100k notices ran
CPU-bound past 80 min in the rehearsal (per-tender transactions in
`apply_tender`); the full backfill will want this profiled/batched before the
multi-million-notice run.
