# 232 — the text era projects titles but almost no buyers, values or winners (3.79M versions)

Status: buyer FIXED in code 2026-08-18 (`0074b61`) and riding the 244 redo sweep (section 1 buyer 0.5%→63.0% mid-sweep); CY/TW follow-on LANDED 2026-08-22 (rides the NEXT era pass); `value` closed as
NOT-A-BUG (the era publishes no amount field); `winner` open, blocked on a `CO` archive study
Kind: projection mapping gap, largest single era by volume
Blocked by: —
Relates to: 11 (the text-era profile), 176 (the per-era headline-fields matrix — fixture-level,
which is why this never showed there), 172 (codelist/currency drift, same era), 230 (the
measurement that found this), 187 (INTERNAL_OJS award linkage — the neighbouring island)

## What

The first full-corpus data-quality measurement (job 731, 32 id windows, 1258 s) puts the text
era at **3,786,955 tender-versions — the largest era in the corpus, ~27 % of all versions** —
and reports:

    era                    versions   title  buyer  value    cpv deadline winner
    text 1993–2010        3,786,955  100.0%   0.5%   0.1%  95.5%    84.4%   0.1%

Title 100 %, CPV 95.5 %, deadline 84.4 % — the era projects well on three of six fields. Buyer
**0.5 %**, value **0.1 %**, winner **0.1 %**.

## Why this is a finding and not just "old data is thin"

A 1993–2010 OJ tagged-text notice names its contracting authority — that is the point of the
publication. So "the buyer isn't in the source" is not a plausible explanation for 99.5 % of
3.79M notices, and it should not be assumed to be one. Compare the neighbouring islands in the
same report: `INTERNAL_OJS 2008` measures **91.5 % buyer** on the same kind of legacy content,
and `TED_EXPORT r2.0.8` measures 98.4 %. Whatever the text-era parser or projection does with
authority names, the eras on either side of it do something different and better.

The two candidate causes, and they are distinguishable:
1. **Parse side** — the text-era parser does not emit an authority/party section (or emits it
   under a stem the projection's `role_name` does not recognise), so no `organization_mention`
   is ever seeded. Check `notice_sections` for a text-era notice directly.
2. **Projection side** — the sections exist but `TEXTS`/`AMOUNTS`/`role_name` lack the text-era
   (`TXT-*`) stems for party, amount and winner, exactly as issue 177 found for r208 values and
   issue 29 found for sdk-0.1. Grep the mapping tables for `TXT-` coverage per field.

**DIAGNOSED from the code, same day — it is cause 2, and more absolute than "a missing stem".**
The text era emits **no organization role reference at all**, so there is no path by which a
buyer party row could be written:

- A party role is created in exactly one place: `project.rs:2413` matches
  `NoticeValue::Id { is_ref: true }`, skips `scheme == "ojs"` (chain edges), and asks
  `role_name(field_id)` for the role. `role_name` (project.rs:3355) understands three prefixes
  — `OPT-300-`, `OPT-301-`, `TED-` — and nothing else. There is no `TXT-` branch.
- The text parser can only ever produce ONE kind of ref: `text/parse.rs:165`'s `Type::Ref` arm
  hardcodes `scheme: Some("ojs")`. Every text-era reference is an OJS chain edge, and those are
  filtered out one line before `role_name` is reached.
- The authority field itself is not a ref: `text/rules.rs:64` declares `("AU", Prose(None))`, so
  `TXT-AU` arrives as a Text value. `ORG_NAME_FIELDS` does include `TXT-AU`
  (project.rs:307), so a mention gets a NAME — but a mention with no role never becomes a
  `tender_version_parties` row.

So the text era seeds organization mentions and then has nothing to attach them to. The fix is
not a new entry in a mapping table (there is no ref to map); it is deciding how a text-era
authority becomes a role — most likely synthesising a `buyer` role at the AU-bearing section,
which is exactly what issue 29's sdk-0.1 fix did for `ContractingParty` ("`read` synthesises a
`buyer` role at the ContractingParty section"). That precedent is the template.

**Still to explain: why 0.5 % and not 0.0 %.** ~19,000 versions DO carry a buyer, and on this
diagnosis none should. Candidates: transition-year (2009–2010) notices carrying TED-style
address blocks under `TED-*` ids while still profiled `text`, or notices whose Tender also holds
a non-text version whose parties resolved (parties are per version, so this should NOT leak —
worth confirming). Find one and read it before building the fix: if the 0.5 % arrives by a path
that already works, that path may be the fix rather than a new one.

## Why it hid

Issue 176's per-era matrix test asserts the headline fields for one fixture per era, and it
passes: a text-era fixture that DOES carry its fields projects them. That test proves the path
works for the fixture chosen; it says nothing about the share of 3.79M real notices that take
that path. Corpus-scale completeness is a different question and, until issue 230, nothing
could ask it — every query timed out. This is the first answer.

## Impact

Buyer rollups, buyer search and any authority-level analytics silently exclude the largest era
in the corpus — a user filtering by contracting authority sees 1993–2010 as nearly empty rather
than as unmapped. That is the same class of harm as issue 29's island Tenders, at five times the
volume.

## Acceptance

- The layer where the buyer is lost is identified from one traced notice and recorded here.
- The mapping (parse or projection, whichever it is) is fixed and unit-tested per the era.
- The era is re-projected and the data-quality report shows non-trivial `buyer`.
- `value` and `winner` are judged separately and honestly: pre-1999 amounts are national
  currencies (issue 172) and most text-era notices are not award notices, so those two columns
  need the right denominators before anyone calls them bugs. Section 3 of the report (results
  materialisation, measurable from rev `c731a05`) is where the winner question belongs.

## 2026-08-18, later — buyer FIXED in code; `value` is not a bug; `winner` needs an archive study

### Correction to the diagnosis above

The diagnosis said the era "seeds organization mentions and then has nothing to attach them to". That
is **wrong**, and the difference decides the fix: **no mention is seeded at all.**
`Projection::mentions` (project.rs:2484) only creates a mention for sections whose KIND is
party-bearing — `Organization`, or sdk-0.1's party kinds — and a text-era record has exactly ONE
section, `PROCEDURE` of kind `Notice`, because the era publishes no structure (`text/parse.rs`'s module
doc says so outright). `TXT-AU` sitting in `ORG_NAME_FIELDS` could never have mattered: there was
nothing for the name to attach to.

That rules out a projection-only fix, which is what the earlier note implied was possible. And
`organization_mentions` carries FOREIGN KEY (notice_id, section_id) REFERENCES notice_sections, so the
section has to exist in the parsed layer — it cannot be conjured at fold time.

### Fixed: the authority becomes an Organization (`0074b61`)

The parse layer now manufactures the section, exactly as the r209 walker already does for inline
address blocks (`Rule::Org`): open `ORG-1` of kind `Organization`, put the authority's name inside it,
and record the ROLE as an id-ref on the enclosing section. The ref is emitted as
`TED-ADDRESS_CONTRACTING_BODY`, which `legacy_role` already folds onto `buyer` — **so the projection
needed no new mapping, no new code path and no new vocabulary.** The only thing that changed is where
`TXT-AU` lands.

`AU` is safe to treat as a bare name: a single clean line in every vintage the fixtures cover (1993,
1993-rp, 1995, 2000, 2005, 2008), and the inventory declares it in all six.

The era matrix test (issue 176) gained the `buyer` column whose absence let this hide — it asserted
title/deadline/cpv/value and never looked at parties at all, which is exactly how a fixture-level test
stayed green while 99.5 % of the era had no buyer. All seven rows are `true`, each re-checked against
raw fixture bytes. Red-checked: with the AU routing disabled, `text-2008` fails on `buyer` and the
other six eras still pass.

### `value` 0.1 % is CORRECT — drop it from this issue

The text-era inventory has **36 field codes and not one of them is a monetary amount.** In full: AA
AB AC AU CC CO CT CY DD DR DS DT HD IA MA NC ND OC OJ OL ON OT PC PD PG PN PR RC RG RN RP TD TI TW TX
TY — `PG` is "Page in the OJ", the closest thing to a number, and there is nothing else. The era's
header record simply does not publish a contract value; any amount lives inside the `TX`/`OT`/`AB`
prose bodies as unstructured text.

So ~0 % is the honest reading, not a mapping gap, and chasing it would be effort spent against
nothing. This is the same trap issue 231 flags for sdk-0.1's CPV — check whether the era publishes the
field before mapping it. (Extracting values from prose is a different project entirely, and would want
its own issue and its own accuracy argument.)

### `winner` IS a real gap, but do not map `CO` yet

`CO` = "Successful contractor(s)", present 2000–2010, and it is `Prose(None)` — a text row on the root
that reaches nothing. So the gap is real. But the one committed fixture that carries it shows a shape
that would produce garbage if mapped as-is (`text/2005-can-154-2005.txt`):

    CO: Name and address of successful supplier, contractor or service provider:
        Grahams Engineering Ltd.
        NSG Environmental Ltd.

The head line is a **LABEL**, and the continuation lines are **two separate contractors**. `Prose(None)`
newline-joins all three into one blob, so a naive `CO` → winner-name mapping would create one
Organization named "Name and address of successful supplier, contractor or service provider:\nGrahams
Engineering Ltd.\nNSG Environmental Ltd." — a fabricated party, which is worse than a missing one.

Needed before any code: an archive study across the vintages `CO` spans (2000, 2005, 2007, 2008,
2010) answering — is the head line always a label, or do some vintages put the first name there? Is it
always one contractor per continuation line? Are addresses mixed in with names? Only then does the
rule (`Prose` → `PerLine`, plus label stripping) become a decision rather than a guess. One fixture is
not an era; this is the same discipline that made the sdk-0.1 and r208 value work land correctly.

### Also evidenced, also deferred

`TW` = "Town of the awarding authority" and `CY` = "Country (code)" both describe the authority, and
`ORG_COUNTRY_FIELDS` already lists `TXT-CY` — evidence the design anticipated a text-era org. Routing
them into `ORG-1` beside the name would give the mention a country (which helps organization identity
dedup). Deliberately left out of `0074b61` to keep one claim per commit; it is a small follow-on with
the evidence already in hand.

### One consequence of the fix, filed as issue 234

Landing this by re-parsing the era will mint roughly **one provisional Organization per notice**, and
that is worth knowing before the re-parse rather than after. `resolve_one_mention` reuses an
Organization only when the mention carries a usable identifier; without one it runs an unconditional
INSERT — no lookup, no key. The text era publishes no identifier anywhere in its 36-code inventory, so
every one of its 3.79M mentions would mint its own row.

The buyer would then be **present but not aggregatable** — "MAIRIE DE PARIS" as thousands of distinct
Organizations, so the authority rollups that motivated this issue still would not work, while this
issue's completeness number went green. Issue 234 carries the decision. Deliberately NOT bundled here:
that one is an identity policy that can silently merge distinct entities, this one is a mapping backed
by six vintages of fixture evidence, and landing them together would make a bad outcome
un-attributable.

### Status: the code is in, the corpus is not

The 3,786,955 stored notices keep their old parse layer until the era is re-parsed. That is a separate
scheduled unit and it wants the same rebuild window issue 100's DE-1.x cohort is waiting for
(`reparse` → one `project --rebuild`, ADR-0009). Until then the data-quality report will keep showing
0.5 % buyer for the text era, and that is expected rather than a sign the fix did not work.

### The CY/TW follow-on landed (2026-08-22, owner)

The "also evidenced, also deferred" item above is in: `home_authority_descriptors`, a post-pass in
`text/parse.rs` beside `claim_awarded_value`/`claim_award_date`, re-homes root `TXT-CY`/`TXT-TW`
into `ORG-1` when the record opened one. A post-pass because the era publishes header order —
`CY:` arrives BEFORE `AU:` in every fixture vintage, when no authority section exists yet. No
`AU:` → both stay on the root, unchanged.

The projection needed nothing: `TXT-CY` has sat in `ORG_COUNTRY_FIELDS` since before this issue.
End-to-end test (`the_text_authoritys_country_reaches_its_organization`) proves the mention AND
the organization carry `FR` from the 2008 fixture.

Worth more than it looks post-234: the reuse-before-minting scope requires name AND country —
without a country every text-era buyer mention takes the unconditional-INSERT path and the era's
next re-parse would re-fragment into ~3.79M provisional organizations, the exact shape 234
collapsed. With it, the era's authorities aggregate by (name, country) as they fold.

Timing: the 186-374 redo sweep (running tonight) predates this code, so the era's stored parse
layer gets countries on its NEXT full pass — which should be the one that also carries the
`winner` fix once the `CO` archive study lands, exactly as buyer+AU rode one pass this time. No
dedicated re-parse for CY/TW alone.

### Buyer acceptance: 100.0% (2026-08-23, run #335)

The redo sweep finished and the first full-corpus read shows section-1 buyer at **100.0%** for
the text era (was 0.5% when this issue was filed, 63.0% mid-sweep). The buyer half of this
issue is DONE end to end. CY/TW country homing (landed f6d7f76) still awaits the next era pass
+ deploy; `winner` is subsumed by 244's campaign (era winner-named now 100.0% where a result
materialised — the CO-archive study this issue deferred became 244's slices).
