# Skeptic pass on audit_ted.md §4 (2026-10-19)

Agent `verify_ted`. I tried to explain away the one claim in `audit_ted.md` §4. First I re-read the saved
bodies in `raw/ted/`, the saved `/docs` and OpenAPI, CHANGELOG.md, and the ingest code that produces the
value. Then I made 2 new GETs (the cap was 3), logged as `agent=verify_ted #1`–`#2` in
`api_requests.log`, with the bodies in `raw/verify_ted/`. There were no 408s and nothing was retried.

## Claim

On `/v1/notices/47414082/content` (Tender 8852009) and `/v1/notices/47413608/content` (Tender 8852005),
the `PROCEDURE` section labels the procedure-level contract nature `BT-23-Lot`. The notice's SDK
assigns that element to `BT-23-Procedure`.

**Verdict: CONFIRMED. Severity is low, and the error is in the content endpoint only.** It is also
systemic, not limited to these two notices (see below).

## What the saved bodies show

| notice | profile (OPT-002) | subtype (OPP-070) | `PROCEDURE` (kind `Notice`) | `PAR-0000` (kind `Part`) |
|---|---|---|---|---|
| 47414082 | eforms-sdk-1.14 | 4 | `{field_id: BT-23-Lot, list: contract-nature, code: services}` | `BT-23-Part` services |
| 47413608 | eforms-sdk-1.13 | 4 | `{field_id: BT-23-Lot, list: contract-nature, code: services}` | `BT-23-Part` services |

Neither notice has a `Lot` section, and neither carries `BT-23-Procedure` anywhere. Every other
procedure-level ProcurementProject term on the same section has the right `-Procedure` id:
`BT-21-Procedure`, `BT-24-Procedure`, `BT-262-Procedure` and `BT-22-Procedure`.

## Testing each escape route

1. **Did the auditor misread the source? No.** The auditor's SDK facts check out against the
   repo's own inventories (`crates/ingest/sdk/fields-1.13.0.json` and `fields-1.14.0.json`):
   * `BT-23-Procedure` has xpath `/*/cac:ProcurementProject/cbc:ProcurementTypeCode`, with no
     predicate. It is mandatory on subtypes 1–37, which includes 4.
   * `BT-23-Lot` has xpath `/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject/cbc:ProcurementTypeCode[@listName='contract-nature']`.
     It is **forbidden on subtypes 1, 2, 3, 4, 5, 6, E2, X01 and X02** in both minors.
   * The raw XML was unavailable, but the served row itself shows where the value comes from. It
     sits in the root section (`PROCEDURE`), and the notice has no Lot element to have been mirrored
     from. Its `list: contract-nature` is the element's `@listName`, the same channel that gives
     `BT-02` its `list: planning`. So the element is the root
     `cac:ProcurementProject/cbc:ProcurementTypeCode[@listName='contract-nature']`. Under the notice's
     own SDK, that element is `BT-23-Procedure`.
2. **Does a later notice in the chain supersede it? No.** Both Tenders are `version: 1`, and each
   has a single caused-by notice. In any case, the content endpoint is per notice: it serves what
   this notice's parse holds.
3. **Do /docs, the OpenAPI or CHANGELOG.md document it? No.**
   * OpenAPI `getNoticeContent` says: "verbatim from the parse layer — field ids in the source's own
     vocabulary (eForms BT/OPT ids …; **the notice's `profile` names which**)". The profile is
     `eforms-sdk-1.13`/`1.14`, and in that vocabulary this element is `BT-23-Procedure`. The id
     served is one the profile forbids on subtype 4. The endpoint's promise is therefore broken,
     not just stretched. `NoticeContent` ("Each value carries its source field_id") adds no caveat.
   * /docs → Notice content says "source field ids like TED business terms, not canonical
     projections". It has no rule that a field id's suffix may disagree with its section, and the
     text contains no `BT-23`, alias, graft or suffix rule for content ids.
   * The closest thing in /docs is its illustrative example. It shows `BT-01(c)-Procedure` inside
     section `LOT-0001`. That is the opposite direction: a procedure block published per lot, kept
     under the business term the SDK gives that block. It is offered without any rule. It does not
     cover this case, where the element sits at the SDK's own declared location for
     `BT-23-Procedure` and is renamed to a different, forbidden term.
   * CHANGELOG.md has no entry for `BT-23`, an alias or graft, suffixes, or content field ids.
     Its only `/content` mentions are about text encoding and dates.
4. **Does the code's own rationale cover it? Not even that.** The only rationale is the code
   comment on `ALIASES` in `crates/ingest/src/eforms/index.rs`, which is not public documentation.
   That comment states the invariant "*a field the SDK already defines at the target always wins*".
   The mechanism defeats it:
   * `insert_field(..., gap_only = true)` checks for an existing field only on the exact branch,
     predicates included.
   * The grafted step `cbc:ProcurementTypeCode[@listName='contract-nature']` is a new sibling of the
     SDK's predicate-free `cbc:ProcurementTypeCode`, so it is filled with `BT-23-Lot`.
   * `Branch::matching` then sorts the matches "most predicates first" (`sort_by_key(Reverse(preds.len()))`),
     so the grafted `BT-23-Lot` wins over the SDK-declared `BT-23-Procedure`.

   The result is an unintended side effect, not a documented rule.

## Scope check (2 new GETs)

Tender 8851996 (`00720743-2026`) is subtype 7, SDK 1.13, with 4 lots. Its caused-by notice is
47413157. `/v1/notices/47413157/content` serves `BT-23-Lot` in `PROCEDURE` **and** in
`LOT-0001`…`LOT-0004`, and no `BT-23-Procedure` anywhere. So on a TED eForms notice the root
contract nature reads as `BT-23-Lot`, whatever the subtype and minor. The repo's internal issue 465
records the same shape for notice 23555356 (`BT-23-Lot=services` twice, once at the procedure),
though that is internal notes, not documentation. A consumer reading the content endpoint by field
id finds no `BT-23-Procedure` on these notices.

## Not affected

The Tender projection is unaffected. `/v1/tenders/{8852009,8852005}` serve `nature` `services` at
the procedure and at PAR-0000, and 8851996 serves it at the procedure and on all four lots.
`contract_nature()` in `crates/ingest/src/project.rs` accepts both ids and takes the scope from the
section. The value `services` is correct everywhere. Only the business-term id on the content
endpoint is wrong.

## Explained

None. No documented rule or caveat explains this difference.
