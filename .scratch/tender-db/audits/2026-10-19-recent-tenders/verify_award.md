# Verify: skeptic pass on audit_award.md's discrepancy D1

- Date: 2026-10-19, 22:04Z to 22:08Z. Agent tag in `api_requests.log`: `agent=verify_award`.
- GETs: **1** of the 3 allowed (#1, `/v1/tenders/8851944`, 200, no 408). The raw body is in `raw/verify_award/`.
  Everything else is re-read from saved bodies: `raw/award/*`, `raw/ted/*`, `raw/verify_ted/*`, and the
  reference copies `raw/docs.html` and `raw/openapi.json` (md5 877f49b7… and 9acc9651…).
- Method: I tried to explain the claimed discrepancy away in three ways: (a) the auditor misread the
  source, (b) a later notice in the chain supersedes it, (c) a documented rule or caveat covers it in
  /docs, the OpenAPI descriptions or CHANGELOG.md. Code was read only to confirm intent, never as
  documentation.
- Process note: I ran one read-only `git log -1` by mistake while locating source files. It changed
  nothing, but the task rules out git. No other git command was run.

## Verdict

| Tender | Field | Confirmed | One-line reason |
|---|---|---|---|
| 8851944 | `parties[]`: no `Lot-ReviewOrg` for LOT-0000 | **true** | Source read correctly, no later notice, and no documented rule. The drop is an undocumented code rule. |

## D1: tender 8851944, `parties[]` has no review body

### (a) Did the auditor misread the source? No.

The saved `raw/award/content_47411306.json` shows:
- LOT-0000 carries `OPT-301-Lot-ReviewOrg` = `TPO-0001` with `is_ref: true` and `scheme: "touchpoint"`.
- `TPO-0001` is a section of kind `Touchpoint` whose `parent_section_id` is `ORG-0001`. ORG-0001 is the
  buyer, "Dirección General de Racionalización y Centralización de la Contratación-Junta de
  Contratación Centralizada".
- The touchpoint publishes `BT-500-Organization-TouchPoint` "Tribunal Administrativo Central de
  Recursos Contractuales (TACRC) o Tribunal Superior de Justicia (TSJ).", plus BT-512 28000,
  BT-513 Madrid, BT-507 ES300 and BT-514 ESP.

Every detail of the auditor's source quote matches. In eForms a touchpoint is "an addressable
sub-office/contact of an Organization", and role references may point at ORG or TPO sections (repo
`docs/research/eforms-data-model.md` §2). So the reference is a valid way to name the review body.

**Even the strictest reading of the source leaves a gap.** Suppose the touchpoint is read as part of
ORG-0001 rather than as a separate organization. Then the notice says ORG-0001, reached at TPO-0001,
is the LOT-0000 review organization. The API does not serve that either: org 6614 appears only as
`Procedure-Buyer`. So the source assigns a review-org role on LOT-0000, and `parties[]` assigns none,
however the touchpoint is read.

### (b) Does a later notice supersede it? No.

- The saved detail and GET #1 (22:06Z) are byte-identical: `version: 1`, and `versions[]` has one
  entry, caused by notice 47411306.
- Parties carry forward role by role (/docs, OpenAPI `parties`), but no second notice exists, so
  nothing is carried forward and nothing is replaced.

### (c) Does a documented rule or caveat explain it? No.

| Place | What it says | Explains the drop? |
|---|---|---|
| OpenAPI `TenderDetail.parties` | "The organizations this version names, and the role each plays." eForms roles are served as published: `Lot-ReviewOrg` means `review-body`. Its other caveats are head vs mention names, identifier-first binding, `Tenderer` not meaning winner, `is_buyer`, and unfolded names kept verbatim. | No. Nothing says that a role whose target is a touchpoint is left out. "Organizations this version names" could be read narrowly as Organization sections only, but the review body here is an organization in fact, and the role is in the served vocabulary. |
| OpenAPI, whole file | 0 hits for "touchpoint". All 17 hits for "ouch" are "vouch". No `OPT-301` hits. | No |
| /docs, Tender detail ("A party names its organization twice…") | Head vs mention names, carry-forward, binding by identifier, `is_buyer` | No |
| /docs, Notice content | "Use it when the projected tender is not enough: to see a field the canonical layer does not model" | No. This is a general pointer, not a rule. The canonical layer does model the role (see the controls below), and nothing in /docs tells a reader that a modelled role disappears when the notice points it at a touchpoint. |
| /docs, whole page | 0 hits for "touch", "notice-layer", "Lot-Review", "ReviewInfo" or "AddInfo" | No |
| CHANGELOG.md | 0 hits for "touchpoint", "TPO", "OPT-301", "not an Organization" or "Organization section". The "touch" hits mean "touched". | No |
| Repo sources of the served texts | `crates/app/data/openapi.json` has the same md5 as the served copy, and its `parties` description is identical. `crates/app/src/v1/docs.rs` contains no "touch". | No (nothing undeployed would change this either) |
| Issue tracker | No issue about touchpoint-targeted roles. The only "touchpoint" hits in issues 99 and 368 use "touch points" in a different sense. | Not a known, tracked gap |

**The rule exists only in code.** `crates/ingest/src/project.rs:6111-6113` (`bind_organizations`):
"A reference to something that is not an Organization section (a touchpoint, a lot, a result) is
notice-layer detail, not a party." The `let … else { continue }` skips the reference. No eForms
ingest path mints a mention or party for a `Touchpoint` section. The other matches for "TouchPoint"
in `crates/ingest/src` are XPath entries in `eforms/index.rs`. So this is intended behaviour, but it
is undocumented.

### Controls and scope, all from saved bodies with no new GETs

| Notice | Tender | Role reference | Target kind | Served? |
|---|---|---|---|---|
| 47410357 (AT) | 8851923 | Lot-ReviewOrg, Lot-ReviewInfo → ORG-0003 | Organization | yes (org 47) |
| 47413608 (NO) | 8852005 | Part-ReviewOrg → ORG-0002; Part-ReviewInfo → ORG-0001 | Organization | yes (398, 4643720) |
| 47414082 (LV) | 8852009 | Part-ReviewOrg → ORG-0002; Part-AddInfo, Part-EmployLegis, Part-EnvironLegis, Part-FiscalLegis → ORG-0001 | Organization | yes (88, 4545) |
| **47411306 (ES)** | **8851944** | Lot-ReviewOrg → TPO-0001 (TACRC o TSJ) | Touchpoint | **no** |
| **47413157 (ES)** | **8851996** | Lot-ReviewOrg → TPO-0001 (Tribunal Administrativo de Contratación Pública de la Comunidad de Madrid); Lot-AddInfo → TPO-0002; both on LOT-0001 to LOT-0004 | Touchpoint | **no** (served `parties[]` = `Procedure-Buyer` only) |

What the controls show:
- **Role scope.** The drop depends only on the target kind. Every ORG-targeted role reference in the
  saved bodies is served, and every TPO-targeted one is dropped. It is wider than `Lot-ReviewOrg`:
  8851996 also loses `Lot-AddInfo` on four lots.
- **Country scope.** Two of the two Spanish notices among today's saved bodies use the touchpoint form
  for the review body. That supports the auditor's "probably systematic for Spanish notices", but two
  notices from Madrid buyers are a small sample, not a measurement.
- **Second tender is outside D1.** 8851996 was not one of the award audit's picks, so it shows the
  pattern but is not a second D1 finding.

### Severity

The auditor's "low" holds.
- **Nothing wrong is served.** No organization appears under a role it does not hold. The award,
  winner, bids and contract on 8851944 are unaffected.
- **The omission is silent.** A consumer asking for the review body (`Lot-ReviewOrg` / `review-body`)
  gets "none named" for these tenders. The OpenAPI gives no hint that `/v1/notices/{id}/content` is the
  place to look.
- **Two fixes are open.** Either document the rule in the OpenAPI `parties` description and the /docs
  Tender detail, or mint touchpoint-targeted roles as parties. The second is harder: a touchpoint has
  no identifier of its own (here only name and address), so it would bind by name.

## Result

- **D1 (8851944, `parties[]` review body): confirmed = true.** The served value contradicts the
  source, since the source names a LOT-0000 review body and `parties[]` has none. The auditor read the
  source correctly. No later notice exists. No text in /docs, the OpenAPI or CHANGELOG.md explains the
  drop: the only rule is a code comment.
- **Corrections to the claim:** none of substance. It understates the scope a little, since any
  touchpoint-targeted role is dropped (e.g. `Lot-AddInfo` on 8851996), not only `Lot-ReviewOrg`.

## Files

- `raw/verify_award/tender_8851944_recheck.json`: GET #1, byte-identical to `raw/award/tender_8851944.json`
- Re-read from earlier agents' saved bodies: `raw/award/content_47411306.json`,
  `raw/award/content_47410357.json`, `raw/award/tender_8851923.json`,
  `raw/verify_ted/content_47413157.json`, `raw/verify_ted/tender_8851996.json`,
  `raw/ted/content_47413608.json`, `raw/ted/content_47414082.json`, `raw/ted/tender_8852005.json`,
  `raw/ted/tender_8852009.json`, `raw/docs.html`, `raw/openapi.json`
