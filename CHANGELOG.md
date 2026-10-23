# Changelog — the public /v1 API

Behavior changes a client could observe, newest first. Additive fields and new
endpoints land without an entry unless they change how an existing request
answers; this file exists for the rare case where one does.

## Unreleased (issue 577) — notice content serves the field id the notice's own SDK declares, not a grafted one

`/v1/notices/{id}/content` and the `notice_*` tables on `/v1/sql` promise field ids in the source's own vocabulary.
Where the parser fills a gap the notice's SDK leaves with an id grafted from another mount or a synthetic `UBL-` id,
that id could sort ahead of, or replace, the field the SDK declares for the very same element. Notice 47414082
(sdk-1.14) served its procedure's contract nature as `BT-23-Lot`, and so did the root of 2,193 of 2,201 recent eForms
notices. The parser now binds the declared field there.
- EU and eForms-DE 2.x root contract nature: `BT-23-Lot` → `BT-23-Procedure`. A lot keeps `BT-23-Lot`, a part
  `BT-23-Part`.
- SDK 1.3–1.7 procedure exclusion-ground description: `BT-772-Lot` → `BT-67(b)-Procedure`.
- A lot's electronic ordering and payment indicators: `BT-92-Lot` and `BT-93-Lot` (were `UBL-ElectronicOrderUsage`
  and `UBL-ElectronicPaymentUsage`).
- SDK 1.0–1.11 lot selection-criterion name: `BT-749-Lot` (was `UBL-SelectionCriterionName`).
- SDK 1.0–1.8 lot procurement-document status: `OPT-050-Lot` (was `UBL-DocumentStatusCode`).
- eForms-DE 1.x lot values: the lot's own `DE1-ProcurementProjectLot-*` ids (were `DE1-ProcurementProject-*`,
  `DE1-TenderingTerms-*`, `DE1-TenderingProcess-*`, `UBL-*` and a few `BT-`/`OPT-` ids). The lot's
  procurement-document language moves from `notice_codes` (`UBL-DocumentLanguageID`) to `notice_ids`
  (`DE1-ProcurementProjectLot-TenderingTerms-CallForTendersDocumentReference-LanguageID`).
- A grafted id that names the right business term through its own discriminator is kept, its scope given by its
  section: `BT-01(c)-Procedure` in a `LOT`, `BT-1252-Procedure` on a lot's direct-award justification.
- A stored notice changes once it is re-parsed (issue 577 unit 3); until then it serves the ids it was parsed with.
  Canonical tenders are unchanged.

## Unreleased (issue 546) — legacy TED `country` is the place of performance only; `?country=` no longer matches a buyer's or winner's region

A legacy TED (r2.0.9) tender's `country` was the union of three things: its place of performance, its buyer's
address region and its winner's. Tender 8432519, works in FRK22 by a buyer in FRI12, served `["FRI12", "FRK22"]`,
so `?country=FRI` found it. eForms and DÖE serve the place of performance only, so `country` meant
different things in different eras. In the sampled r209 notices, about a third gained a `?country=` region match
this way, and about 1% a match in a wrong country.
- `country`, `?country=` and the `nuts` rows of field `place` (`classifications[]`, `/v1/sql`) are the place of
  performance only, in every era. 8432519 serves `["FRK22"]` once refolded (see the last point).
- The buyer's and winner's regions are not deleted: they stay on the notice layer, `notice_classifications` on
  `/v1/sql`, under field ids `TED-CA_CE_NUTS` (buyer) and `TED-TENDERER_NUTS` (winner), and as `TED-NUTS` in the
  notice's address sections.
- No fallback: a legacy tender none of whose notices publishes a place of performance (e.g. most F12/F13 design
  contests, which publish only addresses) serves `country: []`, as eForms does. A later notice that publishes only
  addresses (e.g. an F14 corrigendum) no longer replaces the place an earlier notice of the same procedure published.
- The sampled r2.0.8 notices (2013, and the 2019 ones) carry no address regions and serve what they served; the
  issue-546 refold's cohort count is the full check.
- Stored tenders change only after the issue-546 refold. Until then a legacy tender folded before this change may
  still serve its buyer's or winner's region. The refold announces each corrected tender as a `version: null`
  `changed` row (ADR-0017 D3).

## Unreleased (issue 551 B6) — `lot-sum-awards` is served again, guarded by two vouching guards

The B5 stop-rule entry below switched `lot-sum-awards` (path W v2) off on 2026-10-19: its drains changed about twice
as many Tenders as projected. The projection was at fault, not the quality bars, which had passed. It was never
re-based on the rate measured after the lot-key drains (41 ships of 277 walked, against M1's 30 of 276), and that
rate predicts about 2,265 of the 2,348 fires seen inside the population the projection was built on. The other 1,154
fires, a third of the 3,502, fell on Tenders outside that population: the cohort grew from 194,404 to 251,526 Tenders
with the lot-key drains and new notices, and no read had sampled them before B5. A read of 61 of the sums it had
served, against their notices, then found 59 right, 1 neutral and 1 wrong. The wrong one, tender 6465510, was
vouched by a single-lot award notice's own total, for a lot the sum did not include. R10's "never one lot's own
award" could not see it, because that award is stored without its lot or its value. B6 serves path W v2 again,
with two guards on the total that vouches.
- From the deploy on, `lot-sum-awards` is served again as the B4/B5 entry below describes it (v1's guards and
  R1–R10; every sum vouched by a total the buyer stated within 1 %), with two guards that remove vouching totals.
  **G-single**: a total from a notice whose round in the Tender's version has exactly one result, keyed or not,
  does not vouch. **G-cover**: an award notice's total (BT-161, BT-118, BT-1118, or a CAN-type form's TED
  `VAL_TOTAL`) or either end of a TED `VAL_RANGE_TOTAL` vouches only if its notice has a result on at least one of
  the summed lots (one that closes the lot or awards nothing counts; a result with no lot does not); estimate totals
  are unchanged. With no total left the sum is refused (R10), and the Tender keeps the value election's one figure
  (an award's Tender-level copy, or `null`). `value_derivation.vouched_by` names the first total, in chain order,
  that both guards keep. `lot-sum-vouched` is unchanged, byte for byte.
- E.g. 585917 serves EUR 250,000.00 again (`null` while switched off; vouched by its BT-118-NoticeResult), 5995030
  PLN 1,549,511.60 (its TED `VAL_TOTAL` reads PLN 1,546,390.00) and 8426318 GBP 3,000,000,000.00 (its TED
  `VAL_RANGE_TOTAL` high end), as at B4/B5: neither guard moves them. 6465510 keeps EUR 70,340.42, not B5's
  EUR 162,020.92: its only vouch, notice 20772908's TED `VAL_TOTAL` of EUR 161,290.32, comes from a notice with one
  result, on none of the summed lots, and either guard removes it. 7202436 keeps EUR 26,981.20, not B5's
  EUR 27,222.00: its only vouch, notice 22179136's EUR 26,981.20, comes from a notice with three results, none of
  them on a summed lot, so G-single keeps it and G-cover removes it. The Lyon-Turin Tenders 6450467, 6450469 and
  6450473 state no total and keep EUR 1,660,661,682.94.
- What was measured before it was served, on fresh fetches of the 3,502 Tenders B5 had served:
  - The port of record reproduced all 3,502 of B5's sums: S, currency, lot count and `vouched_by`.
  - The two guards refuse 35 of them. So **3,467 of the 3,502** serve their sum again, each with the S, currency,
    lot count and `vouched_by` B5 served.
  - The Rust code agrees with the port on 37,732 (row, guard set) pairs, with 0 disagreements: 9,433 population
    rows covering 8,731 distinct Tenders, each under the four guard sets.
  - Each guard was adopted by a rule fixed before its read: adopt it if at least 10 % of the sums it refuses read
    wrong or neutral. G-single: 3 of 30 read wrong (the first 30 of its 34 refusals), the 6465510 shape. G-cover:
    1 of 1, tender 7202436, the one sum it refuses beyond G-single (it refuses 13 in all).
  - A fresh sample of 30 of the 3,467 read right against their notices, all 30.
- What subscribers see: each of the 3,467 Tenders takes its **third value correction since B5**. It went on with
  B5's drain (2026-10-18/19), off with its drain back (2026-10-19), and goes on again with this drain. 2,334 of them
  were also among the Tenders issue 551's first drain moved to `lot-sum-awards` and unit 4a's drain moved back (both
  2026-10-17), so this is their fifth. Each move is announced like any value correction: a `version: null`
  `changed` row (ADR-0017 D3), for the Tender and its lots.
- Tenders already stored are refolded by one drain over an exact id set, never by walking the corpus.
  - The drain is `refold-lot-sum` scoped by `tender_ids` to the 3,502 Tenders B5 served, in stages, and then the
    Tenders that took a change row since.
  - The runs over the 3,502 are expected to correct exactly the sums predicted for them (3,467 as measured;
    re-predicted in the flip window for those of the 3,502 whose facts changed since) and to leave the sums the
    guards refuse unchanged (35 as measured). Any other correction there switches it off. The later runs, over the
    Tenders changed since, may newly derive sums (about 1.4 % of them). Above max(2 × their projection,
    projection + 10) the drain stops for a read, and new sums that fail the sampled read switch it off.
  - Until its refold, a Tender keeps the figure stored for it.
  - A Tender folded after the deploy for another reason, such as a new notice in the daily `project`, has its sum
    derived by that fold, before or without the drain, and announced the same way.
- Accepted residuals:
  - A sum vouched by a total that a later notice superseded (5816379's shape) stays unguarded. It is about 1 in 60
    of the sums read, and the error is about 1.5x.
  - A partial sum stays served. This is a sum that leaves out awards stored without their lot, so it covers part of
    what was awarded. Of the eight read, seven read right and one neutral (tender 7749755).
- A Tender with no convertible amount matches a value bound again when its lot awards are summed (`/docs`, the
  filter table and the Amounts caveat; OpenAPI `min_value`). `/docs`, OpenAPI `value` and `value_derivation`
  (`method`, `vouched_by`) and the `/v1/sql` notes say it is served, with both guards.

## Unreleased (issue 530) — a body one notice names twice under two per-notice platform keys binds by its name

A v4 UUID in `identifier` (`identifier_kind: "national"`) is a submission platform's record key, not a register
number, and some German platforms mint a fresh one per ORG section per notice: one body (a buyer and its information
desk, a review chamber in both review slots) became a row per notice with one or two mentions each. Landkreis
Göttingen read as 52 GUID-keyed organizations.
- From the deploy on, when ONE notice copy (a TED and a DÖE copy are each read alone) names one body — the same
  normalised name in the same register country — under two or more distinct v4 keys, those keys are dropped from its
  mentions (the published value stays in the notice) and the mentions join the body's name-keyed organization (the
  identifier-NULL row of that name and country, or a new one). Exceptions, which keep binding as before: the 7,312 keys
  on a frozen keep-list (every key whose organization already spans two or more Tenders and the stable platform keys
  such as 254 DB InfraGO and 255 Vergabekammer des Bundes, as of the list's cut; the list is only added to by a reviewed
  commit, and a key merged later is not added to it), names that are not safe as a fallback key (fewer than two content
  words, generic body names such as `Vergabekammer` or `Zentrale Vergabestelle`, polity umbrellas such as `Freistaat
  Bayern`), groups whose sections the notice's own role references give a review body beside a buyer-side role, or a
  bidder (`Tenderer`, `Tenderer-MainCont`, `Tenderer-SubCont`) beside a role that is not a bidder's, and mentions with no
  country. A body a notice copy names under one key only (once, or more than once under the same key) is untouched: the
  Südbayern and Bremen per-notice keys, one per notice, stay split.
- The stock moves once, with the wet `dissolve-guid-twins` run after the deploy: about **10,995** single-Tender GUID
  organizations are dissolved — each answers `/v1/organizations/{id}` with 404, no redirect, as in issues 440, 510
  and 522 — and their mentions join the name-keyed rows (about 52 of those are new). The change feed carries about
  10,995 `organization removed`, about 52 `organization added`, `organization changed` for each name-keyed row that
  took mentions (and each row that kept some), and about 10k `tender changed`. E.g. Göttingen's 22759261 and
  22759262 answer 404 and their mentions read on 2294259 (`Landkreis Göttingen`, identifier NULL).
- `?identifier=` with the key of an organization the wet run dissolved answers an empty page (the notice still
  publishes the key). The key is dropped only from the twinned notice copy's mentions: where it still binds elsewhere —
  another notice, or the other (TED/DÖE) copy, naming the body under it once, or an organization that kept some of its
  mentions (a partial row) — `?identifier=` keeps answering that GUID organization, which no longer carries the twin
  copy's mentions. Until the wet run, every pre-deploy GUID organization still answers its key.
- Until the wet run, only twin notices ingested after the deploy bind this way.

## Unreleased (issue 539) — a DÖE notice's `published_at` is raised to its IssueDate's day when that is later

DÖE publishes no journal date. A DÖE notice's `published_at` was the publication date the buyer requested (BT-738 /
`RequestedPublicationDate`) or, where the notice carries one, a date the portal stamped (OPP-012 /
`DE1-Publication-PublicationDate`), the stamped one first. On a share of notices that date is weeks or months older
than the notice's own IssueDate, which lies inside the export that delivered it (586 days on notice 26044079, the
largest gap in the sampled id ranges): an sdk-0.1 change notice repeats its first version's request, and a de-*
stamped date can be months older than the request beside it. So `dispatched_at` (the IssueDate) fell days to months
after `published_at`.
- From the deploy on, a DÖE notice that states a publication date (requested or stamped) gets as its `published_at`
  the latest of those dates and of its IssueDate's day, the day read in the IssueDate's own offset. When it states
  one and the IssueDate's day is later than every one, `published_at` is that day as a date only (`2023-01-31`),
  never the IssueDate's time of day. Otherwise it is the latest stated date, served as before. An sdk-1.x notice
  (3,950 DÖE notices) states none and keeps its IssueDate, timed, as before; the rule does not touch it.
  `dispatched_at` is unchanged, and no other source changes. E.g. tender 65023 (notice 26044864) serves
  `published_at` 2023-01-31 instead of 2022-10-19, beside `dispatched_at` 2023-01-31T14:02:48+01:00; notice 26411995
  serves its requested 2024-07-25 instead of its stamped 2023-12-19.
- Counted over the 135 DÖE exports stored on 2026-10-19, **16,344** DÖE notices had an IssueDate on a later day than
  their `published_at`. The drain is expected to re-date about that many, plus the notices of this kind ingested
  before the deploy (the `repair-notice-instants` dry pass gives the exact count). On a 7-export sample (702 notices
  with a stamped date) the latest-date rule re-dated no notice outside that set. The rule fixes 8,292 of the 8,296
  sdk-0.1 notices dated 2 or more days before their export, and 5,343 of the 5,669 notices 30 or more days early in
  the regular exports. A DÖE `published_at` still usually runs 1–3 days before the day of the portal's daily export
  that carries it; that is how the portal cuts its exports, and it is left alone, as are the 2022-12 backlog export's
  genuinely old notices.
- `published_at` orders a Tender's versions, so a re-dated version can move within its chain: about **3,800** are
  estimated to (23.5% of the re-dated versions in a four-export sample).
- **Stored rows change at the drain.** `repair-notice-instants` re-dates the stored DÖE notice rows. Then a `refold`
  of the DÖE profiles that re-date (sdk-0.1 and de-1.0 to de-2.1) re-queues their notices, stamps their Tenders stale
  and folds them, so each chain is re-sorted. Every Tender with a re-dated version is announced like any correction:
  a `version: null` `changed` row (ADR-0017 D3). That covers the roughly 16,344 re-dated versions, about 3,800 of
  which also change order. `repair-version-instants` is not part of this drain: it rewrites dates in place without a
  change row and re-sorts no chain.
- Until the drain, a stored DÖE notice (`/v1/notices/{id}`) keeps the date it was stamped with, unless it is
  re-parsed. A Tender is different: it dates its DÖE versions again whenever it is refolded after the deploy (a new
  notice joining it in the daily `project`, or a refold run for another issue), each move announced like any
  correction. So until the drain, a Tender's `published_at` and version order can be ahead of its notices'
  `published_at`. `/docs` (the Dates caveats) and OpenAPI `Tender.published_at`, `Tender.dispatched_at` and
  `Notice.published_at` say so: `Tender.dispatched_at` no longer promises "earlier than published_at" where a DÖE
  notice caused the current version (a `ted`-labelled Tender included): there a dispatch falls on or before
  `published_at`'s day, and days after it until the drain.

## Unreleased (issue 571) — an R2.0.9 transport award's operator owners are no longer its result winners

An R2.0.9 public-transport award (MOVE form T02) lists, inside its award block, the operator (`ADDRESS_CONTRACTOR`)
and then the operator's owners (`ADDRESS_PARTY`, 0–100 public bodies). The results reader took every organization in
the block as a winner, so the owners sat in `lot_results[].winners` beside the operator and `?winner=` found the
Tender under an owner, while `parties[]` already refused them the `winner` role (issue 534).
- `lot_results[].winners` (and `?winner=`, `/v1/lots?winner=`, `v_lot_results` on `/v1/sql`) name the operator only.
  The owners stay as parties under the `ADDRESS_PARTY` role. Example: 6386700 serves winner 3302058 (Trenitalia).
- An award block that names only owners writes no winner.
- Measured: 67 notices carry `ADDRESS_PARTY`; their Tenders were refolded on 2026-10-19 (66 corrected), each announced
  as a `version: null` `changed` row (ADR-0017 D3).

## Unreleased (issue 541) — a 2016 public-transport award (T02) serves its operator as a `lot_results` winner

An R2.0.8 public-transport award (MOVE form T02, Regulation 1370/2007) keeps its award in a group the results reader
never opened. Its operator was served as a `winner` party, but `lot_results` was empty, so `?winner=` missed it.
- A T02 notice with no result section now writes one result, key `RES-T02`: winners are the operators it names
  (`ECONOMIC_OPERATOR_NAME_ADDRESS`), decided on its `CONTRACT_AWARD_DATE`, with no value (the form's figures are cost
  parameters, not an award value). Example: 5782055 serves `RES-T02`, winner 1953406 (Barnimer Busgesellschaft mbH),
  decided 2016-09-21.
- The operator's owners (`PARTY_NAME_ADDRESS`) are not winners.
- Measured: 753 notices carry the form's cost parameters (T01 and T02); their Tenders were refolded on 2026-10-19
  (126 corrected), each announced as a `version: null` `changed` row (ADR-0017 D3).


## 2026-10-19 (issue 551 B5's stop rule) — `lot-sum-awards` is switched off again, pending an investigation

The B4/B5 entry below switched `lot-sum-awards` (path W v2) on, with a stop rule set before the drain ran: if the
drains made fewer than 835 or more than 3,338 corrections together (0.5x–2x of the 1,669 projected), switch it off
and investigate. They made 3,504 (cohort drains 74 + 1, corpus drain 3,429): about 2.1x the projection. They fall on
3,503 Tenders (one cohort correction was corrected back), and 3,502 of them serve `lot-sum-awards`. Nothing has been read wrong yet; the switch-off is the
rule, and the investigation (why 2.1x; the pre-registered confirmation read) follows.
- From the deploy on, no fold derives `lot-sum-awards`: a Tender with no procedure-level figure serves the value
  election's one figure again (an award's Tender-level copy, or `null`), as it did after unit 4a. `lot-sum-vouched`
  is unchanged, byte for byte.
- The Tenders the drains moved to `lot-sum-awards` are refolded back after the deploy (`refold-lot-sum` scoped by
  `tender_ids` to those 3,502 Tenders). Each move is announced like any value
  correction: a `version: null` `changed` row (ADR-0017 D3). Until its refold a Tender keeps the sum stored for it.
- A Tender with no convertible amount matches no value bound again (`/docs`, the filter table and the Amounts caveat;
  OpenAPI `min_value`). `value_derivation.vouched_by` stays declared (OpenAPI, `/v1/sql`): it is written only while
  path W is switched on, and the guards R1–R10 below are what it is served under if it is switched back on.
- Superseded by issue 551 B6 (above): served again, with two guards on the vouching total.

## Unreleased (issue 551 B4/B5) — `lot-sum-awards` is served again, only where a total the buyer stated vouches for the sum

Unit 4a (below) withdrew `lot-sum-awards`, the sum of a Tender's latest lot awards served where it publishes no
procedure-level figure, after a read against the notices found it wrong on 18 % of a uniform sample and on 42 % of
the Tenders it served at EUR 100 m or more. Unit 4b redesigned it and measured the redesign before any of it was
served: on a fresh uniform 30 of its sums, 29 read right against their notices (1 wrong), and all 20 at EUR 100 m or
more read right.
- From the deploy on, `lot-sum-awards` is served again under its earlier guards plus ten refusal families (R1–R10):
  the sum is refused, and the Tender keeps the value election's one figure (an award's Tender-level copy, or `null`),
  when a lot was won by two or more separate tenders (a consortium on one tender, or a TED award to a group, is one);
  when the awards' published lot numbers collide, list several lots, disagree across rounds, or are missing on awards
  from two or more notices; when a notice's own total is below the lots it awards (or, with no such total anywhere,
  the sum is over 10x the largest estimate); when an award is 10x its lot's estimate, 10x or a tenth of an earlier
  award on the same lot, or 10x the next largest award; when an award reads as a closed lot, a unit rate, a ceiling
  copied per supplier or the procedure total booked on one lot (fifteen shape checks, among them a sum below
  1,000.00 in its own currency); when a lot is restated under a second key; when an award was resolved through
  another lot's tender; and **R10**, when no total the buyer stated vouches for the sum.
- **R10 applies at every size**: some notice of the Tender must state a total in the sum's currency within 1 % of
  it, and that is not one lot's own award nor equal to any single summed award. Accepted: an award notice's total
  (BT-161, BT-118, BT-1118, TED `VAL_TOTAL`), a procedure estimate (BT-27-Procedure, TED `VAL_ESTIMATED_TOTAL`) or
  either end of a TED `VAL_RANGE_TOTAL`; never a modification's before/after value. BT-118 and BT-1118 are read for
  this only: they are never an amount row and never elected (issue 506 stands).
- `value_derivation` on a `lot-sum-awards` head gains **`vouched_by`** `{field, notice_id, cents}`: the stated total
  that vouches (its field as published, e.g. `BT-118-NoticeResult`, `TED-VAL_TOTAL`, `TED-HIGH`; the notice that
  stated it; its cents in `value`'s currency). `lot-sum-vouched`'s detail is unchanged, byte for byte. OpenAPI
  declares `vouched_by`; `/v1/sql`'s `current_value_derivation_detail` note names it.
- E.g. 585917 serves EUR 250,000.00 again (vouched by its BT-118-NoticeResult), 5995030 PLN 1,549,511.60 (its TED
  `VAL_TOTAL` reads PLN 1,546,390.00, 0.2 % off) and 8426318 GBP 3,000,000,000.00 (its TED `VAL_RANGE_TOTAL` high
  end). The Lyon-Turin Tenders 6450467, 6450469 and 6450473 state no total and keep EUR 1,660,661,682.94.
- Tenders already stored are refolded after the deploy (`refold-lot-sum`: a named cohort first, then the whole
  cohort): about 1,669 values are expected to move (the drain stops outside 835–3,338). Each move is announced like
  any value correction: a `version: null` `changed` row (ADR-0017 D3). Until its refold a Tender serves the value
  election's one figure.
- A Tender with no convertible amount matches a value bound again when its lot awards are summed (`/docs`, the
  filter table and the Amounts caveat; OpenAPI `min_value`).
- Superseded by issue 551 B5's stop rule (above): switched off again on 2026-10-19, pending an investigation; served
  again by issue 551 B6 (above), with two guards on the vouching total.

## Unreleased (issue 568 unit 8) — an R2.0.8 award that names several lots, or another lot, stays on the Tender

Issue 568 serves each R2.0.8 award on the lot its contract notice printed under the award's number. Its read-back
found awards whose own text contradicts that number: one award for several lots that prints only one of them
(8327340: `обособени позиции 5, 6, 15, 16`, its 709,431.63 BGN served on lot 5 alone), awards numbered by a contract
counter rather than by lot (`Pakiet nr 5` printing 4), awards titled exactly as another lot, and one award printing
six lot numbers. Measured 2026-10-18 on 180 uniformly drawn corrected Tenders: 12 (6.7 %, 95 % 3.5–11.4 %) served a
wrong block.
- From the deploy on, an R2.0.8 award whose own title names several lots, names another lot than both its printed
  number and the lot that number finds (by that lot's own title or description: `Pakiet 8 poz. 1` printed 9 on a lot
  titled `pakiet 8` agrees), or carries another lot's title, or that prints several lot numbers, stays on the Tender
  (`lot_results[].lot` null), as it was before issue 568. Where two or more awards of one notice name another lot
  that way, that notice does not number its awards by the contract notice's lots: every award of it stays on the
  Tender too, except one whose own title names exactly the number it prints, as a lot (5051007's `Pakiet nr 1`..`3`
  keep lots 1-3; a position within a lot, `Pakiet 16 zad.2` printing 16, confirms nothing). One such award alone
  keeps only itself off its lot. A title is read language by language, and only
  ever keeps an award off a lot, never puts it on one (an award titled `Zadanie 2` printing 1 is served on no lot,
  not on lot 2). None of this applies on a Tender whose lots print fewer than two numbers.
- What moves, on the corrected Tenders only: `lot_results[].lot` back to null, and with it the lot a winner and an
  awarded value belong to (`parties[]`'s lot-scoped rows, `/v1/lots?winner=`, a lot's award value; on `/v1/sql`,
  `v_lot_results.lot_key` and `v_awards.lot_key`), and a stated total's awarded lots. Rarely a Tender's `value` and
  `value_derivation` move too, where an inflated lot sum shrinks back (issues 492 and 537, `lot-sum-vouched`).
- Expected: about 7 k Tenders touched (3–15 k), about 3 k of them fully back on the Tender; no right award of the
  2,081 measured (1,762 in sample, 319 out of sample) is refused with them (≤ 0.18 %). A residual stays: 5 of 230
  uniformly drawn Tenders (2.2 %, 95 % 0.7–5.0 %) keep a wrong award (a renumbered award paraphrasing its title, a
  single counter-numbered award with no lot word, a weld of another procedure, a lot that exists only in corrigendum
  prose, duplicated placeholder lot sections); 2 of the 5 are issue 568's own.
- Tenders already stored are corrected by the refold that also drains issue 570 (the notices that carry a LOT_NO or a
  LOT_NUMBER). Each move is announced as a correction: a `version: null` `changed` row for the Tender and for its lots
  (ADR-0017 D3). Until its refold a Tender serves its awards as before.

## Unreleased (issue 570) — a legacy TED lot keeps one identity across the notices of its Tender

TED's legacy notices (2016–2024 R2.0.9, and R2.0.8) key each lot by its position in the notice (`LOT-1`, `LOT-2`,
…), and a Tender's lots were merged across its notices by that key. Where two notices list the lots differently
(an award notice listing only some lots, packages split across two award notices, two lots swapped), one Tender
lot held another lot's title and facts and served another lot's awards. Measured 2026-10-18 on 204 multi-lot legacy
award notices (2016–2023): 3.4 % had a result or lot fact on another lot (9 of 1,596 results); e.g. 8375276's eight
2018–2020 award notices swap lots 1 and 2, so 147 of its awards served on the other lot.
- From the deploy on, a lot section of a later legacy notice joins the Tender lot an earlier notice of the same
  Tender printed under the same number (as printed, else its one other spelling, as issues 567 and 568 match), and
  that notice's awards follow it. The first notice that publishes lots keys them: lot keys are never renamed or
  invented, and every lot id stays the one it was. A section keeps its position where it prints no number. A whole
  notice keeps its positions, as before, where two of its sections name one lot, where one prints a number no
  earlier lot prints at a position a lot with another number holds (a range such as `1-4` against `1`, or `2.`
  against `2`), where no earlier lot prints a number at all, or once an eForms notice of the Tender has published
  lots. Never by title, item ordinal or position.
- What moves, on the corrected Tenders only: a lot's title and facts (the lot keeps the identity its first notice
  gave it), `lot_results[].lot` and with it the lot a winner and an awarded value belong to (`parties[]`'s lot-scoped
  rows, `/v1/lots?winner=`, a lot's award value; on `/v1/sql`, `v_lot_results.lot_key` and `v_awards.lot_key`), and a
  stated total's awarded lots. Issue 568's "earlier notice renumbered its lots" exception narrows: an R2.0.8 award
  after a notice whose swap is now joined by number gets its lot; one after a renumbering that cannot be joined
  still stays on the Tender. Rarely a Tender's `value` and `value_derivation` move too, where a moved award changes
  the lot sums the €1 bn checks read (issues 492 and 537, and `lot-sum-vouched`).
- Tenders already stored are corrected by a refold after the deploy (the notices that carry a LOT_NO or a
  LOT_NUMBER; about 22 k Tenders are expected to move, 9–42 k). Each move is announced as a correction: a
  `version: null` `changed` row for the Tender and for its lots (ADR-0017 D3). No lot is added or removed. Until its
  refold a Tender serves its lots as before.

## Unreleased (issue 568) — an R2.0.8 award is served on the lot its contract notice numbered

TED's R2.0.8 award notices print each award's lot number (LOT_NUMBER) but carry no lot section of their own: the
lots exist only in the Tender's contract notice. The fold matched an award only against the lots of its
own notice, so every R2.0.8 award was served on the Tender (`lot_results[].lot` null), even where the contract
notice had published the very lot it names. Measured 2026-10-17 on 470 R2.0.8 award notices (2012–2015): about
16 % sit on a Tender that gains a lot key, 1.4 award blocks per award notice; of the 647 blocks the new rule
places in a re-read sample of 71 multi-lot Tenders, none was placed on a wrong lot. E.g. 4748635's eight awards
now serve LOT-1, LOT-2, LOT-3 and LOT-5 to LOT-9, the lots that print 1, 2, 3 and 6 to 10 (its contract notice
skips lot 5).
- From the deploy on, an award notice with no lot section of its own puts each award on the lot an earlier notice
  of the same Tender printed under the award's own number: the number as printed, else its one other spelling among
  those lots (`01` and `1` match, as issue 567's do). Two lots printing the number, or none, leave the award on the
  Tender. So do: lots lettered A, B, C against awards numbered 1, 2, 3; a number only the award prints; a lot added
  only in a corrigendum's prose; an award notice that prints one number on all of its two or more awards while the
  Tender's lots print several numbers (5476747 prints `1` on all three); a Tender whose earlier notice renumbered
  its lots; an award that prints no number. The lot is never guessed from its key's position, nor from the award's
  position, title or item ordinal. An award notice with lot sections of its own (R2.0.9) keeps issue 567's
  matching, unchanged.
- The lot is set on the version the award notice causes and carried as it was: a later notice that adds or
  renumbers lots never moves an earlier award.
- What moves: `lot_results[].lot`, and with it the lot a winner and an awarded value belong to (`/v1/lots?winner=`,
  a lot's award value; on `/v1/sql`, `v_lot_results.lot_key` and `v_awards.lot_key`). Several awards of one lot
  (framework suppliers, partial awards) all land on it. What does not: the winner's `parties[]` row stays on the
  Tender (`lot` null), because a notice's parties take only the notice's own lots and an R2.0.8 award notice has
  none. Rarely a Tender's `value` and `value_derivation` move too: the rules that check a procedure figure of €1 bn
  or more against its lots' award sums (issues 492 and 537, and `lot-sum-vouched`) now see R2.0.8 award lots for
  the first time, so a kept figure can turn refused or a refused one kept, a move by a power of ten.
- Tenders already stored are corrected by a refold after the deploy (the notices that carry a LOT_NUMBER: R2.0.8
  award notices and the contract notices whose lots they match; an estimated 130–160 k Tenders are expected to move).
  Each move is announced as a correction: a `version: null` `changed` row for the Tender and its affected lots
  (ADR-0017 D3). Until its refold a Tender serves its R2.0.8 awards on no lot.

## Unreleased (issue 567) — a legacy TED award is served on the lot its own lot number names

TED's 2016–2024 award notices (R2.0.9) tie each award to a lot by the lot number the publisher printed (LOT_NO).
The fold read that number mixed with each lot section's position in the notice, the first claim winning, and took
an award's last reference. Where the printed numbers are not the sections' order (lots swapped, a number skipped,
a rotated numbering), an award could land on another lot, which then served two results and the right one none; in
a notice with ten or more lots the store's value order (`LOT-10` before `LOT-9`) decided which lot won. A notice
whose lots print their own positions never put an award on another lot, at any lot count. Measured 2026-10-17 on
304 multi-lot legacy award notices: 10.6 % of 2020–23 and 4 % of 2016–19 (6.1 % of 2020–23 award blocks), e.g.
501322-2018's "Science Direct Backfiles" award (E-Nformation, RON 14,502,000) on the "Wiley Journal Backfiles" lot.
- From the deploy on, an award goes to the lot section that publishes its own LOT_NO: the number as printed, else
  its one other spelling among the lots (`Lot 01`, `Los 1` and `1` match). A number two lots publish, or no lot
  publishes, leaves the award on the Tender (`lot_results[].lot` null), never on a guess. A notice whose lots print
  no number keeps the positional link it had, and so does an award that prints none where every lot's number is its
  position.
- What moves: `lot_results[].lot`, and with it the lot a winner and an awarded value belong to (`parties[]`'s
  winner rows, `/v1/lots?winner=`, a lot's award value; on `/v1/sql`, `v_lot_results.lot_key` and
  `v_awards.lot_key`). The lot that carried two results keeps its own. Rarely a Tender's `value` and
  `value_derivation` move too: the rules that check a procedure figure of €1 bn or more against its lots' award sums
  (issues 492 and 537, and `lot-sum-vouched`) read each award's lot, so moving an award to its own lot can change
  that sum and turn a kept figure refused or a refused one kept, a move by a power of ten.
- Tenders already stored are corrected by a refold after the deploy (the r2.0.9 notices that carry a LOT_NO, about
  1.4 M; tens of thousands of Tenders are expected to move). Each move is announced as a correction: a
  `version: null` `changed` row for the Tender and its affected lots (ADR-0017 D3). Until its refold a Tender serves
  the old lot.

## Unreleased (issue 551 unit 4a) — `lot-sum-awards` is withdrawn: a Tender with no procedure figure no longer serves its lot awards' sum

Issue 551 (below) served a Tender that publishes no procedure-level figure the sum of its lots' latest awards,
marked `value_derivation.method` = `lot-sum-awards`. Read against the notices after its rollout (2026-10-17; the drain
changed 15,352 Tenders), that sum was wrong on 9 of a uniform sample of 50 (18 %, 95 % ~9–31 %) and on 96 of the 229
Tenders it served at EUR 100 m or more (42 %): framework ceilings or per-supplier maxima added up across lots that share
one envelope, one lot under two keys or a re-issued award added to its original, a sum farther from the notice's
stated total than the figure it replaced, a ×10^k award slip. Its guards do not separate those shapes, so it is off.
- From the deploy on, no fold derives `lot-sum-awards`. Such a Tender serves the value election's one figure again
  (often an award's Tender-level copy, i.e. one lot's award) or `null`, with `value_derivation` null. E.g. 585917
  serves `null` again (not EUR 250,000.00), 5995030 PLN 2,721.60 (not PLN 1,549,511.60), 8426318 GBP 2,000,000,000.00
  (not GBP 3,000,000,000.00), and 6450467, 6450469 and 6450473 EUR 1,660,661,682.94 (not EUR 3,574,764,979.28).
- The Tenders that carry it are refolded back after the deploy (`refold-lot-sum`, the cohort issue 551's drain used:
  about 15,352 values move back). Each move is announced like any value correction: a `version: null` `changed` row
  (ADR-0017 D3), so a `min_value`/`max_value` subscriber hears `removed` when the value leaves its bound, and
  `added` or `changed` (apply it as an upsert) when it enters it: under `max_value`, on a Tender with an earlier
  version, the entry usually arrives as `changed` (the earlier version is read as a superset, issue 547 below).
  Until its refold a Tender still serves the sum and its marker.
- `lot-sum-vouched` is unchanged: a refused procedure total that is exactly 10^k times the lot sum still vouches for
  it (all 16 adjudicated heads read right after the drain).
- A Tender with no convertible amount again matches no value bound (`/docs`, the filter table and the Amounts
  caveat).
- OpenAPI keeps `lot-sum-awards` in the `value_derivation.method` enum, described as withdrawn and not currently
  served; a redesign is open (issue 551 unit 4b). `/v1/sql`'s `current_value_derivation` note says the same.
- Superseded by issue 551 B4/B5 (above): the redesign was served from 2026-10-18, every sum vouched by a buyer-stated
  total, switched off again on 2026-10-19 by B5's stop rule, and served again by issue 551 B6 with two guards on
  that total (both above).

## Unreleased (issue 562) — a Tender re-labelled `ted` (or re-kinded) reaches a `source` (or `kind`) subscription

The fold rewrites a Tender's `source` for the whole Tender (a DÖE Tender turns `ted` when its TED twin arrives,
ADR-0003) and, rarely, its `kind` (issue 462). A subscription's diff re-evaluates its filter against the old and new
version, but both sides read the one rewritten label, so such a move reached a `source=doe` subscriber as nothing (it
kept a ghost of the Tender and its lots for good) and a `source=ted` one as `changed`.
- From the deploy on, a fold that moves a Tender's `source` or `kind` also writes a `version: null` `changed` row for
  the Tender and one for each lot of its current version, after the version's own rows (ADR-0017 D3's correction
  shape; `/docs`, event schema). The same holds when a refold corrects a stored label without writing a version.
- SSE: a subscriber to the old label hears `removed` (for the Tender on `/v1/tenders`, each current lot on
  `/v1/lots`). One to the new label hears `changed` for an entity it already held (twice when a version moved the
  label: the version's own event and the correction), and `added` for one the version before did not hold: a lot
  first written by the re-labelling version, or every entity when the fold leaves the Tender at version 1 or
  rewrites it from version 1. Apply either as an upsert, as `/docs` already says. Other filtered streams can get an
  extra `removed` for a Tender they never held (an idempotent delete).
- Polls (`/v1/changes`) and webhooks are unfiltered and get the extra rows: at least ~370 a week, almost all on
  Mondays and Tuesdays, when weekend DÖE publications meet their TED twins (110–150 Tenders a week, measured
  2026-10-05 to 2026-10-16). A Tender minted from its TED notice, or from both notices in one fold, never moves its
  label and gets none.
- Change rows written before the deploy keep the gap: no walk re-announces past flips. A `source`- or `kind`-filtered
  SSE client holding state from before the deploy should re-subscribe once without a cursor (no `Last-Event-ID`); a
  stream resumed across the deploy keeps the ghosts of earlier flips until it re-snapshots.

## Unreleased (issue 560) — a move of a Tender's value alone reaches a `/v1/lots` value-bound subscription

A lot's `min_value`/`max_value` bound reads its Tender's value, but a lots stream re-evaluated a lot only when a change
row was written for the lot itself: its facts, its kind or its Tender's closure moved. A version that moved only the
Tender's value (a corrigendum of the procedure total, another lot's figure, an award value beside unchanged lot facts,
or the exchange rate on a non-EUR Tender) wrote none, so a lots subscriber kept a lot its bound now excluded and never
heard one it now admitted.
- From the deploy on, a version that moves the Tender's EUR value (to or from no value included) writes a `changed`
  row for the Tender and for each lot present on both sides. A lots value-bound stream hears `removed`, and `added` or
  `changed` (apply it as an upsert, as `/docs` already says). For a Tender that already had versions, the last
  version a fold writes is also announced whenever its value differs from the one the Tender served before the fold,
  or from the one before the fold's first written version, so the current version is probed. That does not make a
  fold of several versions exact: a version written in the same fold as a later one is still read as a superset on
  both sides, and a Tender first folded with several versions has nothing to compare its last version against (issue
  561, below).
- Unfiltered lots streams, `/v1/changes` polls and lot webhooks get those extra `changed` rows: about 1,000–3,000 a
  day. They include moves caused only by the rate on non-EUR Tenders. A TED version published today is compared at
  the previous day's fixing, and the next morning's rate re-derivation still re-announces its lots as a `version:
  null` correction, as before.
- What it fixes: before the deploy, TED Tenders with a non-EUR amount were already re-announced to lot streams by that
  next-morning re-derivation, so for them this only makes the event immediate. The error that persisted was on
  EUR-only Tenders and FTS, about 250 lots a day (measured on 2026-10-14), and it accumulated in a value-bound lots
  subscription.
- Change rows written before the deploy keep the gap. A lots value-bound client holding state from before the deploy
  (EUR-only and FTS Tenders above all) should re-subscribe once without a cursor (no `Last-Event-ID`).
- Still open, and named in `/docs`: a lots stream under a filter that reads the lot's Tender (`country`, `cpv`,
  `buyer`, `winner`, `bidder`, `procedure_type`, `currency`, `status`) checks a lot again only when a change row is
  written for the lot itself (issue 565); and under `min_value`/`max_value` a version since superseded is read as a
  superset on both sides, whether a later version was written in the same fold (a Tender first folded with several
  versions, or a value that moved and moved back within one fold) or the stream resumes, or lags, past it. So a
  `changed` or an `added` can name an entity outside the bound, on a live stream as well as a resumed one, until a
  later change probes the current version (issue 561).

## Unreleased (issue 551) — a Tender's `value` can be the sum of its lots, marked `value_derivation`

`value` was always ONE published figure: the largest the value election accepts. When a scale rule refused the
procedure total (a figure 100x–1,000,000x a sum of the Tender's own lots, issue 537, or an exact 10^k or x100
partner, issues 471 and 492), the head fell to one lot's figure or to `null`, and a multi-lot Tender whose only
figures are lot awards served its largest award. Now, where no procedure-level figure is accepted, two cases serve
the SUM of the current version's lots, in the lots' own currency:
- **`lot-sum-vouched`**: a refused procedure figure is exactly 10^k times that sum, to a cent per lot, so the
  buyer's own total (its decimal separator lost) vouches for it. Each lot's latest award, else its largest
  estimated value or framework maximum, at the scale the refusal named, and only when that sum is above the
  figure the election would serve without it. Pinned on unit 2's 15 adjudicated heads,
  e.g. 7427765 now serves EUR 3,000,000.00 (six awards of EUR 500,000.00) instead of EUR 500,000.00, 5943121
  EUR 1,321,579.92 instead of EUR 416,053.05, and 369057 EUR 62,378,736.29 instead of `null`.
- **`lot-sum-awards`** (withdrawn by unit 4a, above; served again at B4/B5, vouched by a buyer-stated total; switched off again 2026-10-19 by B5's stop rule; served again by B6 with two guards on that total, above): the Tender has no procedure-level figure at all (only
  placeholders), its value was an award's copy or `null`, and each lot's latest award is added up, unless one figure repeats across the lots (a total
  printed on every award), one lot equals 1, 10, 100 or 1,000 times the others, an award is in another currency or
  names a lot the notice does not type, two awarded lots carry one lot number under two keys (a Tender re-keyed from
  TED's `LOT-1` to eForms' `LOT-0001` would count that lot twice; the number is read from the key, and TED's
  `LOT-k` is the notice's k-th lot section rather than its published lot number, so a re-keying whose numbers do not
  line up is not caught), an award or its Tender-level copy is itself refused as a scale slip, that copy is over
  EUR 100 bn, the lots' estimates disagree with the awards by more than 10x, or the sum
  is not above both today's figure and EUR 0.00.
- A new field `value_derivation` on every Tender row (`/v1/tenders`, the detail, SSE `include_data`):
  `null` for a published figure; otherwise `{method, summed, lots}` and, for `lot-sum-vouched`, the `rule` (537,
  471 or 492), its `exponent` and the `refused` figure as published. It describes the current version only.
- `min_value`/`max_value` and `sort` compare the sum's EUR value. `amounts` is unchanged: no amount row is invented.
- A multi-lot Tender whose only figures are lot estimates or framework maxima, with no procedure figure, still
  serves ONE lot's figure (summed estimates moved 3 of 20 adjudicated heads the wrong way).
- On `/v1/sql`, `tenders` gains `current_value_derivation`, `current_value_cents`, `current_value_currency` and
  `current_value_derivation_detail` (NULL on every published-figure head), and `current_value_eur_cents` can be a
  figure no amount row carries. A query that finds the elected amount row by `eur_cents = current_value_eur_cents`
  finds none for a derived head: read the derivation columns there.
- A Tender serves its derived value once refolded (`refold-lot-sum`, after the deploy); a change of value is
  announced like any other value correction (a `changed` row, ADR-0017 D3).

## Unreleased (issue 563) — an event stream refuses a date bound spelled as the literal `now`

A `/v1/tenders` stream (`Accept: text/event-stream`) whose `published_after`, `published_before`, `deadline_after`
or `deadline_before` is the literal `now` (any case) now answers `400`, on a fresh connection and on a resume
(`Last-Event-ID` or `?cursor=`) alike. A stream resolved `now` when its connection opened and again on every resume,
so a resumed stream checked a later bound than the one the client's state was built under: under
`deadline_after=now`, a Tender the client held whose deadline fell between the two connects was never sent
`removed`. The message names the bound, says why, and what to send instead: an absolute timestamp (unix seconds,
RFC 3339 or a bare date), and for `deadline_after` alone also `status=open` for tenders still open, with its caveat
(below). A malformed date bound on such a stream gets a `400` that offers the absolute forms only, not `now`.
- Until issue 547 (unreleased beside this one; the two ship in one deploy) a date-bounded stream answered `400`
  unless it passed `sort=id`, so the one request that used to be accepted and is now refused is a `…=now&sort=id`
  stream.
- Pages accept `now` exactly as before. `/v1/lots`, `/v1/organizations` and `/v1/notices` ignore the date bounds,
  so their streams are unchanged.
- `status` reads the clock the same way when a stream connects and has no absolute spelling, so it stays
  documented (`/docs`, Live feed): no event marks a deadline passing, so drop a Tender from a `status=open` state
  once its deadline has passed; after a resume, a change that leaves such a Tender closed is not sent (a version
  that reopens it arrives as `added`).

## Unreleased (issue 547) — an SSE subscription hears a Tender cross a value or date bound

A subscription's diff re-evaluates its filter against the old and new version of the entity (`/docs`, Live feed), but
`min_value`/`max_value`, `deadline_after`/`deadline_before` and `published_after`/`published_before` read the Tender's
CURRENT figures on both sides. So a change that moved a Tender across one of those bounds reached the subscriber as
`changed` where `added` was due, or not at all where `removed` was due: a corrigendum that lowered the value below a
`min_value`, or moved the deadline past a `deadline_before`, left the client holding a Tender its filter excludes.
- Each side now reads its own version's publication and submission deadline, exactly, on `/v1/tenders`
  subscriptions. A date-bounded subscription opens without `sort`: a date bound implies its ordering on a page only,
  and such a stream (or one bounded on both date columns) used to answer `400` for a sort the client never sent.
  One rule now holds on a `/v1/tenders` stream whatever its date bounds: `sort` unset or `id`, and `order` unset or
  `asc`, are accepted and mean id order; `sort=published_at`, `sort=deadline` and `order=desc` answer `400` "sort
  does not apply to event streams", checked before the both-bounds `400` (whose "pass sort=…" advice is refused on a
  stream). `order=desc` on a stream without a date bound used to answer "descending id order is not supported",
  and `order=asc` beside a date bound used to be refused. Pages answer exactly as before.
- A stream whose date bound is the literal `now` is refused (issue 563, above): a resume would resolve `now` again,
  so its bound would move. The date-bounded streams this entry opens take an absolute timestamp.
- The value bounds read the current value for the current version. An earlier version's elected value is not
  stored, so the earlier side matches when any amount that version published lies within the bound. On
  `/v1/tenders` a Tender whose current version leaves a bound always arrives as `removed`. An earlier version that
  published any figure inside the bound without electing it also reads as matching: a refused figure, a
  partner-only award figure (a framework value, BT-709/BT-660, served at its lot), a `Part` figure, or under
  `max_value` ANY figure below its value — which a Tender with several lots usually has, each lot's figure lying
  below the procedure total. That gives a `removed` for a Tender you never received (apply it as an idempotent delete, as
  the snapshot already requires), or `changed` where `added` was due, which under `max_value` is the usual outcome
  on a Tender with several lots: apply `changed` as an upsert by id too (re-read it, or subscribe with `include_data`).
  Exact needs the elected value stored per version (issue 561).
- `source` and `kind` still read the Tender's current label on both sides; a move of either is announced by the
  fold's `version: null` correction since issue 562 (above).
- Pages, polls and webhooks are unchanged.

## Unreleased (issue 537) — a procedure total 10^k times the sum of its own lots is refused

A Tender's `value` (and what `min_value` / `max_value` compare) no longer elects a procedure-level figure of €1 bn or
more that is 100× to 1,000,000× a sum of the notice's own lots, over two or more lots and to within a cent per lot:
the lot total printed with its decimal separator gone. No single figure of the Tender needs to match. The lots'
latest awards and the award notice's framework maximum or re-estimate per lot (issue 506) always count; the lots'
estimates and framework maxima count only when another procedure-level figure of the Tender lies between a tenth of
their sum and ten times it. A framework total that issue 492 keeps is still kept, and a lots group restating the
refused figure is refused with it. Example: 369057's €62,378,736,280.00 over lot awards summing to €62,378,736.29
now has no `value`, and 165437's €19,975,274,626.00 falls to its €20,202,891.70 procedure estimate. The figure stays
in `amounts`; refused is not corrected, so the value falls to the next accepted figure, or to none. The affected
Tenders (22 on the October 2026 band) are re-folded and announced as corrections.

## Unreleased (issue 538) — a title or description its publisher escaped twice is served decoded

Some publishers escape a text two or three times before it reaches the notice. The notice's own XML decoding removes
one level, so `title` and the detail's `texts[]` served `in modalit&#224; multilotto`, `souscription &amp;amp; la
gestion` or `épicerie bio&amp;lt;br/&amp;gt;`: about 1–2 % of TED eForms Tenders, a few per thousand in TED's
r2.0.8 and r2.0.9 formats, about none elsewhere. In every `title` and `description`, Tender and lot, every source:
- numeric character references (`&#224;`, `&#x2019;`) and the five XML names (`&amp;`, `&lt;`, `&gt;`, `&quot;`,
  `&apos;`, in any letter case, so `&Amp;` too) are decoded, again while one remains, up to three levels;
- an escaped `<br>`, `<br/>` or `<br />` (any case) becomes a line feed, and an escaped private-use code point
  (U+E000–U+F8FF, a Symbol-font bullet such as `&#61655;`) becomes `•`;
- nothing else changes. Other markup, other entity names (`&nbsp;`), a reference to a control character other than
  tab, line feed and carriage return (`&#0;`, `&#150;`), to a surrogate or past U+10FFFF, markup the text carries
  literally, and text mangled at the source (`modalità;`) are served as published. A fourth level of escaping
  stays escaped once (`&amp;amp;amp;amp;` serves `&amp;`).

Where: `title` on `/v1/tenders` and `/v1/lots`, the detail's `texts[]` (fields `title` and `description`), and on
`/v1/sql` `tender_version_texts.value` for those two fields and `tenders.current_title`, so a text match there
(`LIKE '%&amp;%'`) answers accordingly. Not changed: `/v1/notices/{id}/content` and `notice_texts` keep the text as
parsed; the other text fields (`funding_program`, `selection_criterion`, `tenderer_requirement`, `appeal_terms`);
and organization names (`name`, `organization_name`, `mention_name`), whose decode changes the name key that binds
organizations without an identifier and waits for its own unit. New notices are served decoded from the deploy; a
stored Tender changes when the refold after the deploy reaches it, and the change arrives on the change feed and SSE
like any other.

## Unreleased (issue 533) — `status=open` leaves out a procedure its newest notice closed

`status` was the submission deadline alone. A result notice usually publishes no deadline, so the Tender carries its
contract notice's (`/docs` caveats, Dates) and a procedure whose award notice cancelled every lot stayed `open` until
that date passed — years, for some. `status` now reads the newest notice too:
- **`closed`** also when the newest notice published results, every result the procedure has published is a no-award
  (all `clos-nw`; a cancellation, an FTS UK12 termination), and together they decide every lot of kind `Lot` (lots
  groups and parts need no result; a Tender-scoped result decides no lot, so it closes only a Tender with no
  `Lot`-kind lot) — unless the Tender declares an open-ended scheme, where such a result reports one round: a dynamic
  purchasing system (eForms BT-766, or the r2.0.9 DPS marker, on any version), a procedure `oth-single`/`oth-mult`, a
  qualification-system (eForms 15) or dynamic-market (UK13–UK16) notice, or a deadline more than a year after the
  first deadline-bearing notice (an explicit r2.0.9 *discontinued* / *unsuccessful* no-award closes even then). A
  winner, or a decision the notice withheld (NULL), never closes a Tender here. A later notice that publishes no
  results of its own (a re-launch contract notice, but also e.g. a corrigendum) reopens it.
- On `/v1/lots`, a lot is `closed` with its Tender. A no-award for some lots of a Tender that stays open (an FTS
  UK12 that cancels one lot of two) does not close those lots yet: a per-lot rule cost too much on the lots stream
  and is still owed (issue 533 unit 3).
- Applies to `/v1/tenders`, `/v1/lots` and their SSE subscriptions: a closing result reaches a `status=open`
  subscription as `removed` and a re-launch as `added`, each a public `tender` change row and one `lot` change row
  per lot (what webhooks deliver).
- The served `submission_deadline`, `sort=deadline` and `deadline_after`/`deadline_before` are unchanged: they still
  read the deadline alone, so a closed Tender keeps serving its carried date.
- DÖE sdk-0.1 award notices (`can-standard`) no longer map their own `TenderSubmissionDeadlinePeriod-EndDate` to
  `submission_deadline`: it is no submission deadline (an ex-post award of a procedure without a participation
  contest). In a contract notice → award chain the contract notice's deadline carries forward; a single award notice
  has none and reads `closed`. The date stays in `/v1/notices/{id}/content`.
- New: `classifications[]` scheme `dps` (BT-766 `dps-list`/`dps-nlist` at its lot; `dps` at Tender scope for the
  r2.0.9 marker), `lot_results[].reason` codes `discontinued`/`unsuccessful` (r2.0.9), and on `/v1/sql`
  `tenders.current_closed_by_notice`. Complete for the open head and for Tenders folded since the deploy; the
  `dps` rows and reasons reach the rest of the corpus through field refolds.

## Unreleased (issue 532) — a place of performance published only as a country is served in `country` and found by `?country=`

A notice may give its place of performance as a country alone, with no NUTS region: eForms' BT-5141 (often with
`anyw-cou`, "anywhere in the given country") and DÖE's realized-location country. Nothing read it, so such a tender
served `country: []` and no `?country=` value found it. It is now served as that country's NUTS level-0 code: `LT`
for Lithuania, `EL` for Greece, `UK` for the United Kingdom.
- `?country=LT` finds such a tender and its list row echoes `["LT"]`; `?country=LT0` does not find it.
- Only on a version that carries no NUTS place code. A version that already serves a NUTS place serves the same
  `country` as before, even when one of its lots names only a country.
- Only for the NUTS countries: the EU-27, the United Kingdom, Iceland, Liechtenstein, Norway, Switzerland, Albania,
  Montenegro, North Macedonia, Serbia and Türkiye. A place in any other country (Egypt, the USA), or given as an
  overseas region's own ISO code (Réunion, Åland), carries none. A later notice that restates a place that way clears
  the level-0 code that the same procedure or lot derived from an earlier notice.
- In `classifications[]` and on `/v1/sql` the row is scheme `nuts`, field `place_country`. Rows of field `place` are
  unchanged and still hold the NUTS codes as published.

Find a Tender notices are not affected. The stored tenders were refolded on 2026-10-15 (jobs 2521 and 2522,
3,288,104 notices; 197,053 tenders corrected).

## Unreleased (issue 524, unit 2b) — an organization's `name` is elected from its mentions, not its first-seen spelling

`name` on `/v1/organizations` (and `parties[].organization_name`, `lot_results[].winners[].organization_name`,
`bids[].parties[].organization_name`; on `/v1/sql` `organizations.name`, `v_organizations.name`,
`v_lot_results.winner_name`, `v_tender_buyers.buyer_name` and `v_awards.winner_name`/`buyer_name`) was the first
spelling any notice published for the organization, kept forever. Merges and refreshed mentions left heads that few or none of
an organization's notices say: the University of Sheffield served as `Sheffield Hallam University` over 16
Sheffield mentions, a German review chamber as `s.o.`.

For an organization WITH an identifier, the head is now elected from the names its mentions published (ADR-0013's
2026-10-15 amendment): the operator's `refresh-org-names` job serves a fit spelling of the winning cluster, its
whitespace collapsed (trimmed, every run one space). The votes are the mentions that published a name (one with no
name, a blank one or a non-name one does not vote), clustered as one name across casing, diacritics, `the` and a
legal form. The head changes only when a cluster other than its own holds at least 55% of the votes, at least 3
votes and at least twice the head's; or when the head's own cluster holds at most 1 vote and a unique top cluster
holds at least 3 votes and at least 25%; or when the head is empty or not a name (a placeholder, a blank, a
void-lot phrase) and some mention's name votes, whatever script that name is written in. Even then it stays when
the change would swap one legal-form family for another (`AG` to `Sàrl`) while the head's own cluster holds at least
2 votes, drop a consortium mark unless the new name keeps a word besides its legal forms and every such word is a
word of the head, fail the NHS ODS name check the old head passed, or, on the 25% arm only, serve an initialism of the head; when the identifier is a
placeholder (`KEINEANGABE`, `UNBEKANNT` or `NICHTVORHANDEN` with any digits, or all zeros); when no spelling of the
winning cluster is fit to serve; when the identifier carries a `wrong` or unapplied `related` review verdict; on
the 55% arm, while the organization's newest procedures still use the current head (a rename: the head names at
least 2 procedures and two thirds of the organization's newest as many), or while a notice printed the current head
within 3 years (1,095 days) before the job's plan; on the 55% and 25% arms, while the current head is the only one
of the organization's names printed on its newest publication day; all three unless the head is unfit to serve
for a reason other than being all lower case, or carries a standalone number the other name lacks while the other
name drops at most one other word of it (a number the other name prints joined to a neighbouring
word, `SMD 3` against `Smd3`, is the name's own); a head unfit only through a line break or a tab counts as unfit
only when the other name's words lie inside one of its lines or are exactly those of a run of its whole lines, and a
head held this way is instead served as its own words on one line when every break falls mid-phrase (the line before
ends in an unspaced hyphen or a joining word such as `und`, `of` or `z`, or the next line starts lower case, and the
line before does not end in a legal form, a comma, a semicolon or a colon) and the name on one line does not itself
carry such a standalone number, unless that change would trip the legal-form, consortium, NHS or placeholder check
above; on the 55% arm, while a name other than the winning one and the
current head's also has votes and no notice printed the winning name (any spelling of it) within a year (365 days)
before the job's plan (an undated one counts as not printed), unless one of those three holds was lifted for the head;
on the 55% and 25% arms, when the change would serve a review body's name (a procurement review chamber or court) for
a head that is not one, a name led by a contracting organ (`Junta de Gobierno …`, `Alcaldía …`) for a head that is
not, the head's own words plus a code (`(976)`), a name that adds a number after a comma or semicolon (an appended
address; not for a Czech or Slovak school), an ALL-CAPS respelling of a mixed-case head's same words, the same words
printed twice (`X - X`) for a head that is not, or the head with its seat dropped (a tail of 2 or 3 words led by `w`,
`we`, `v`, `ve` or `vo`: `Ośrodek Pomocy Społecznej w Kluczborku` to `Ośrodek Pomocy Społecznej`); and, on the 55% and
25% arms only, when the head is in another script than its mentions (TED's English rendering).

Rule v10 (2026-10-16) added the 3-year hold above: a head still printed is usually the current name, so a majority
for another spelling there is more often a respelling, an appended qualifier, an abbreviation or another entity
sharing the identifier than a fix; a head not printed for 3 years is the outdated name the election exists to
replace. The 3 years count back from the start of the job's plan, which the plan records (`as_of`): a rename run
re-checks every head against that same date, so a head printed after the plan is left as it is. The 25% arm's
changes are not applied for now: every rename run excludes them, pending a refit (issue 548).

Rule v11 (2026-10-16) added four holds after a capped run renamed one organization back to a former name and three
to the wrong one. Three of the four had a third name with votes and a winning name not printed, in any spelling,
for over a year: a newer name had superseded the winner, the old name was being printed again in another spelling,
or the winner was another entity buying for the organization. The fourth was a name printed twice. So on the 55%
arm a change now also waits while a name other than the winning one and the current head's has votes and no notice
printed the winning name (any spelling of it) within a year (365 days) before the job's plan, the year counted back
from the plan's start as the 3 years are. A change whose old head the holds above already released as junk still
applies. The other three holds stop a change to a name printed twice in one field, a change that cuts the head to a
generic name by dropping its seat (`… w Kluczborku`), and every change of an organization whose identifier is a
placeholder (`KEINEANGABE`, `UNBEKANNT`, `NICHTVORHANDEN`, all zeros) rather than a register number. The rename run
can also leave out every 55% change not released as junk whose winning name (any spelling of it) is a year or more out
of print, whatever the other names; the operator turns that on only if the dated review of that class fails.

Rule v12 (2026-10-16, issue 552) narrows the junk release for a head whose only flaw is a line break or a tab inside
the name. A dated review of one plan read five of its junk releases wrong, and all five were such heads: the change
served a shorter or another spelling — a review chamber without its Land, a school without its patron and town, a
ministry service as a bare acronym, a name cut mid-word. Such a head is now released only when the winning name's
words lie inside one of its lines, or are exactly those of a run of its whole lines (`Landkreis Saalekreis` from
`Landkreis Saalekreis` / `Domplatz 9` / `06217 Merseburg`); otherwise the hold that protected it stands. A head held
this way whose every break falls mid-phrase is served as its own words on one line instead (`Szkoła Podstawowa nr 13`
/ `z Oddziałami Dwujęzycznymi im. Jana III Sobieskiego w Tychach` becomes one line): the line before the break ends
in a hyphen joined to its word (`Planungs-`) or a joining word (`und`, `of`, `z`, `και`, …), or the next line starts
lower case, and the line before does not end in a legal form (`GmbH`, `Sp. z o.o.`, `S.C.p.A.`), a comma, a
semicolon or a colon; and the name on one line does not carry a standalone number the winning name lacks (that
would let a later run release it to the winning name after all). A head held this way whose break is not mid-phrase
(after a legal form, or before an address, a branch, a person or a member list) keeps its line break for now. The
name on one line has the same matching key as the head it replaces, so the holds that protected the head keep
protecting it on later runs. Rename runs leave both changes out until a dated review of them passes.

Rule v13 (2026-10-19, issue 548) lifts two checks where a review of the changes they held found them blocking
right changes. The legal-form check no longer holds a change whose current head is a stray, printed (in any casing,
accents or legal form) by at most one of the organization's mentions: all 27 such changes a rename run would have
applied read right (often a single old print of a subsidiary or sibling name served on the register entity's own
number, `Southern Gas Network PLC` to `Scotia Gas Networks Ltd`), while every wrong change it held had a head
printed two or more times, and those stay held. The consortium check no
longer holds a change that cuts a group's name down to words the group's name already prints, legal forms aside
(`Groupement Guintoli/EHTP/GTS/…` to `Guintoli`, `X (mandataire)` to `X`); a review of 30 such changes read 29 right,
1 neutral and none wrong. A change that brings in a word the head does not print still waits (`CONSORTIUM DES
PEINTURES ET REVETEMENTS` to `SARL CPR`, where `Consortium` is the firm's own name), and so does a new name made of
legal forms alone. Every other hold above is unchanged.

- A rename arrives as `organization changed` on `/v1/changes`, the SSE streams and webhooks. No `tender
  changed` is published: the parties join the organization when read, so every tender it is on shows the new
  name at once.
- `name_norm` follows the head, so `?name_prefix=` finds an organization by its elected name; its first-seen
  spelling is matched no longer.
- An organization WITHOUT an identifier keeps its first-seen name: there the name is its matching key. So does
  every head the job has not renamed, a head with a single supporting mention included when the bar or a hold
  above keeps it.
- What one notice published is unchanged, on `parties[].mention_name` and `organization_mentions.name`.

Also: the `next_cursor` of a `?name_prefix=` search is now the last row's stored search key (`name_norm`). It
was rebuilt from the lower-cased name, which differs from the key where the key is `org_name_norm` of the name
(every key since issue 432, and every renamed organization's) and the name has a run of whitespace (a doubled
space, a tab) or a trailing `.`, `,` or `;`. An organization keyed before issue 432 while it had an identifier, and
never renamed since (a reverted rename restores that key), keeps its plain lower-cased name as its key, so its cursor
never diverged.
- Where the rebuilt cursor sorted before the key (a doubled space), the next page repeated rows.
- Where it sorted after the key (a trailing `.`, `,` or `;`: `ACME Ltd.` gave the cursor `acme ltd.` for the key
  `acme ltd`), the next page silently SKIPPED every row keyed between the two: any `acme ltd` with a higher id,
  and names continuing `acme ltd `, `acme ltd (` or `acme ltd -`.

A client that paged a `name_prefix` search to its end may have missed organizations and should page it again. A
cursor issued before is still accepted, but one issued on such a name repeats or skips once more.

## Unreleased (issue 529) — a withheld sole trader named `Individual` is no longer a party without an identifier

Find a Tender publishes a withheld sole-trader supplier under the name `Individual`. A bare one, or one carrying the
s.94 placeholder PPONs (issue 522), bound by name, so every withheld UK sole trader pooled into one organization named
`Individual`. Such a party is now dropped, like the other withheld-supplier stand-ins. It is not served in
`parties[]`, `bids[].parties` or `lot_results[].winners`, and the pooled organization is removed (`organization
removed`). On a Find a Tender result the publisher's decision (`selec-w`) and its awarded value stay. An `Individual`
with an identifier the gate admits is still a party, whoever the identifier belongs to. The rule reads the whole name,
not the source: a bare `Individual` (or `Individual (Lot 2)`) from any publisher drops the same way. A name that merely
contains the word (`Individual Centred Care`) is unchanged.

## Unreleased (issue 534) — legacy TED parties serve canonical roles for award winners, mediation and appeal bodies

Legacy TED notices (R2.0.7–R2.0.9) serve a party's role from the element that names it. Several elements were served
verbatim:
- `AWARD_AND_CONTRACT_VALUE` now serves `winner` (the R2.0.7/R2.0.8 utilities award F06, and the 2008 internal-ojs
  `AWARD_AND_CONTRACT_VALUE_SUM`);
- `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` and `BODY_RESPONSIBLE_MEDIATION` serve `mediation-body`;
- `BODY_RESPONSIBLE_APPEAL` serves `review-body`;
- `SERVICE_INFORMATION_LODGING_APPEALS` and R2.0.9's `ADDRESS_REVIEW_INFO` serve `appeal-information` (it was served as
  `review-body`).

Two consequences show in `parties[]`:
- parties carry forward role by role, so a later notice of the chain that publishes a `winner` now replaces the F06
  award winners on `parties[]` (the award decisions accumulate as before: `lot_results` and `?winner=` keep them);
- a folded award winner that is its notice's own buyer now carries `is_buyer: true` (issue 484).

The tenders projected before the change are refolded (about 2.0M notices, 1.1M tenders); a refolded tender whose served
reading moved publishes `tender changed`. The refold ran on 2026-10-15 (jobs 2512 and 2513).

## Unreleased (issue 536) — `publication_id=` matches TED's own spelling, the OJ S form and every stored width

`/v1/notices?publication_id=` and `/v1/tenders?publication_id=` were an exact string match on the stored form, so
TED's own spelling (`83685-2026`), the OJ S form (`2017/S 044-079997`) and the `/docs` example found nothing, and a
6-digit number returned whichever source stored those digits. A TED lookup now tries every stored width of the same
number (at most four spellings: the value sent, unpadded, 6 and 8 digits). A spelling the API adds, rather than the one
sent, matches TED rows only: `83685-2026`, `00083685-2026` and `2026/S 012-083685` find TED's `00083685-2026` and not
Find a Tender's `083685-2026`. The value as sent still matches a row of any source, so a 6-digit number both TED and
Find a Tender hold (`083685-2026`) returns both notices; pair it with `source=` to get one. The OJ S form with issue
`000` (`2026/S 000-083685`) reaches Find a Tender's 6-digit spelling only. When the page is not empty and matched at a
stored spelling other than the one sent, `resolved_filters.publication_id` reports `{"asked": <the raw value>,
"matched": [<stored spellings>]}`. A `publication_id` page that narrows its number's rows after the read can now be
short, or empty, while `more` is true: its `next_cursor` is the last row examined, so follow it to the end.

## Unreleased (issue 522) — the UK's withheld-supplier PPONs no longer name an organization

Find a Tender publishes placeholder PPONs on the suppliers a notice withholds under s.94 of the Procurement Act 2023:
`GB-PPON-PBXY-3232-WNHZ` ("Withheld Section94 Supplier") and its second profile `GB-PPON-PGYY-2165-QXNQ`. They were
read as identifiers, so every party carrying one was filed under one organization, with an insurer and a German
laboratory among them. These PPONs now key nothing. Each party binds by its published name, and the placeholder organizations are removed
(`organization removed`). The affected Tenders publish `tender changed`. Every other PPON is one supplier's own and
keys it as before.

## Unreleased (issue 526, unit 3) — lettered lot labels, `à <Company>` lists and address columns in 1993–2010 winner lists

The re-parse of 1993–2010 winner lists now reads several shapes the first one left as published.

- **Lettered and Roman labels** split the list as numbers do:
  - `Lot A) Somaco. Lot B) Geneton`, `Lot III: Dompe Biotec SpA`;
  - `lots B1) et B2) Buhrmann France Image`, `lot M00.14) Commune de Spincourt`;
  - `Lot BEY 01) Dumas TP`, `lot 6/1) Imprimerie Dessaint`;
  - the Italian `Lotto 1: Team Service`.
  On a sample of 217 such notices, names that keep a label fall from 453 to 0. A word that only starts like a
  label stays whole (`Lot Atlas`, `LOT Polish Airlines`, `Lot IVECO`).
- **`à` and `pour` award the lots** in the 2007–2010 hospital lists (`Lots 1 et 4 à Ansell`, `Lot 22 pour
  Sophysa`), and each lot keeps its own figure. `1 à 4` is still a range.
- **A company that opens on its digit** is a winner (`3M France`, `3M Santé`, `2SDA`, `Trois M Santé`). The first
  re-parse had refused these. A figure, a quantity or a description is not (`12 véhicules`, `3D Scanners`).
- **Entries joined by a comma, dash or slash** before the next lot's reference name both winners (`Seguridad
  Gallega Nosa Tera, SA, Lote II: Centro de Seguridad Marítima Integral "Jovellanos"`). A lot whose sub-references
  share one winner names it once (`lot 1) Tr. 5 et 6, lot 2) Tr. 1 et 2: Planète Aventures`).
- **The addresses some 2004–2010 values repeat lot by lot** after the winners are no organizations (`Lot 1:
  Frantour, … Lot 1: rue des Clercs, …`), nor are a column header (`Lot`) or a street (`Rua do Cinzeiro`, `Avda.
  Cortes Valencianas`).
- **A label that only describes a consortium** gives way to its name (`groupement d'entreprises: Falconnet Denis`
  → `Falconnet Denis`). A head cut inside a département list reads past it (`Île-de-France (75, 78, 92, 95)
  Lamos SA` → `Lamos SA`).

The organizations that carried a reference are removed (`organization removed`), and the affected Tenders publish
`tender changed`. A block that now names two winners keeps its total unattributed, as a joint award does.

## Unreleased (issue 526) — 1993–2003 winner lists keyed by lot are read one winner per lot

The oldest award notices list their winners lot by lot — `Lot 1) Blin. Lot 2) Prodirest.`, `Lot 1: Spie
Trindel; lot 2: Philips`, `lots 7, 8, 9 and 10: Carabelli`, `10) et 11) Maquet`. These lists were read as one
organization per list or per comma (`Lot 1) Blin. Lot 2) Prodirest. …`, `lots 7`, `et 11) Maquet`), so a company
was split across one organization per lot-number spelling. After the re-parse of the 1992–2003 notices:
- each lot's winner is served under the company's own name (`Blin`, `Prodirest`), merging with the same name
  elsewhere;
- a lot reference is never part of a name, and a slot that names only a lot (`Lot 3`), a pointer (`Lot 2) Idem`),
  a status (`en cours`, `were not awarded`), a price or a lot's lower-case description (`Lot 3) papier, Sopaco`)
  is no organization;
- a keyed award sentence names its awardee (`lot 3) la société Storz a été retenue` → `Storz`);
- an entry that names its lot's subject before the company names the company (`Lot 1) VRD: Colas IDF Normandie`,
  `Lot 1) Terrassement - VRD, Entreprise Malet` → `Colas IDF Normandie`, `Entreprise Malet`), and a company's own
  lot list (`Société Laser: lot 1) Alpes Maritime; lot 3) …`) names the company once.

On a 912-notice sample, names carrying a lot reference fell from 4,423 to 208. Lettered and Roman lot labels
(`Lot A)`, `Lot III:`) were kept as published until unit 3 (above). The prefixed organizations are removed
(`organization removed`), and the affected Tenders publish `tender changed`.

## Unreleased (issue 528) — Dutch award prose in a winner slot is read as what it says

About 130 organizations were named after Dutch award prose sitting in a winner slot rather than after a company.
Each of these reviewed rows is now read as what it says:
- one awardee named in prose is served under the awardee's name (`De opdracht is gegund aan: Tebodin BV` →
  `Tebodin BV`), and a company name carrying an award note serves without it (`De Beijer Bladel BV (6 percelen
  gegund)`). Most of these old notices carry no country, so the awardee is a new organization rather than an
  existing one of the same name;
- a pointer, a count, or an award still to come is no organization (`Zie bij nadere inlichtingen, 3 aannemers zijn
  gegund`, `Dit perceel wordt later gegund.`), and the result keeps its decision;
- a statement that nothing was awarded is no organization, and the result reads `decision: clos-nw`.

Lists of several awardees are kept as published. The prose organizations are removed (`organization removed`), and
the affected Tenders publish `tender changed`.

## Unreleased (issue 527) — a tenderer the notice marks "niet gegunde partij" is not served as a winner

When TenderNed re-publishes an award with each tenderer marked after its name — `(gegunde partij)` for the one
awarded, `(niet gegunde partij)` for the others — the marked losers are no longer served as that notice's
result's winners, even where its contract lists their tenders. They keep their bids (`?bidder=`), and both marks
are dropped from the names served for those mentions. One tender carried the marks when this shipped (Gemeente
Maastricht, 00798764-2024). Its earlier, unmarked award (00792573-2024) still lists every tenderer as a winner,
and an organization first seen under a marked name keeps that name as its served name, until later work.

## Unreleased (issue 520) — an award summary that names one winner binds to that winner

Issue 510 kept, as published, a winner "name" that states a void lot but also seemed to name an award. Three
kinds of them have changed. Void prose that names no award after all (`Le lot 3 : … a été déclaré infructueux`)
is now treated like any other "not awarded": its organization is removed and its result reads `decision: clos-nw`
(about 170 organizations). A summary that names exactly one awardee now binds its winner mention to that awardee's
organization, which takes the winner role. The awardee is named either in its award clause (`Lot déclaré
infructueux … puis attribué à la Société Arrow Génériques suite à procédure négociée` → `Arrow Génériques`) or
before a void tail (`BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les
parts non couvertes des besoins` → `BGE Guyane`; `GRPT GCC (mandataire) et Andre BTP, suite à procédure négociée
après AO infructueux` → the consortium). That is an existing organization of the same name and country where one
exists, otherwise a new one; the result stays `selec-w`, and the summary organization is removed. Summaries that
list several winners, and any summary whose reading is not certain, are still kept as published.

## Unreleased (issues 513, 519) — an NHS body is one organization under its ODS code, its PPON and its sub-codes

An NHS trust, ICB or other body that Find a Tender publishes under its letter-only ODS code (`GB-NHS-QWU`) in some
notices and under its PPON in others, or under several spellings of its name, stood as several organizations, and the
code itself found none of them. Each allowlisted body on the frozen ODS list is consolidated once a reviewed verdict
for it is applied: its rows fold into one survivor (its PPON organization where it has one), every folded row
publishes `organization removed` and answers `308` to the survivor, `?identifier=` with the code answers the survivor,
and later mentions under any of the reviewed spellings bind to it — by name, never by the code: a coded mention under
a spelling no review read stands as its own organization until the next review (issue 513; 171 bodies so far). Since
issue 519, a sub-ICB location or trust site that publishes its own child code while naming itself as its parent body
(`NHS Coventry and Warwickshire ICB - B2M3M`) folds into the parent the same way: its child code also answers the
parent, and a later mention under that code binds to the parent only when its name passes the same strict check the
review used — any other name stays on its own organization, which `?identifier=` then returns beside the parent.
Nothing folds until each body is reviewed, so the change reaches `/v1` as the review runs.

## Unreleased (issue 469) — a UK college, charity or society published under its PPON is merged into its register entry

A UK body that Find a Tender publishes under its Central Digital Platform PPON in some notices and under its register
number in others — a college's UKPRN, a charity's Charity Commission, OSCR or CCNI number, a housing association's
FCA Mutuals number — stood as two organizations. Pairs that a notice states together for one party, that the register
confirms or a reviewer reads, are now merged into the register-numbered organization: the PPON organization publishes
`organization removed` and answers `308` to the survivor, and later PPON-only mentions bind to the survivor. NHS codes
are not part of this yet. Nothing merges until each pair is reviewed, so the change reaches `/v1` as the review runs.

## Unreleased (issue 491) — text-era award notices serve each contract's own figure

Award notices of the 2004–2010 text era now serve each contract's own figure (its "total final value of the
contract") as that award's `awarded` value, where the notice heads its awards and the figure belongs to exactly one
named winner. Before, that figure was read only as the notice's total, and only when the notice awarded a single
contract and stated no overall total. The contract figures are now partners of the existing scale rules, so a notice
total of €1 bn or more that is exactly 10ᵏ× one of its contracts' figures (×100 included) can be refused: a €35 bn
total whose one contract states €35 m is no longer the Tender's `value`, which falls to its next accepted figure or to
none (refused is not corrected). A total that is close to the sum of its contracts is kept, as a framework total over
lots is. No other Tender `value` moves, and `amounts` gains no rows; the contract figures appear only on the results.
The affected Tenders are re-folded and announced as `tender changed`.

## Unreleased (issue 517) — `/v1/changes` refuses a `cursor` parameter

The change feed resumes from `since`. A request that sent `cursor` instead (the list routes' parameter) was answered
from the very start of the log, with `ignored_filters: []`, as if the client had asked for everything. It is now a
`400` naming `since`. `since` itself is unchanged, including its deliberate lenience: a value that does not parse
still reads from the start.

## Unreleased (issue 516) — a supplier that spells out "Public Limited Company" is no longer split from its plc

The Companies House ↔ PPON merge arm (issue 448) read `PUBLIC LIMITED COMPANY` and `LIMITED LIABILITY PARTNERSHIP`
as "Limited", so a supplier published as `X PUBLIC LIMITED COMPANY` under one identifier and `X plc` under the other
was refused as a form conflict and stayed two organizations. The spelled-out forms now read as `plc` / `LLP`; such
pairs merge once reviewed, and the merged organization publishes `organization removed` with a 308 to the survivor.

## Unreleased (issue 512) — a text-era award body that says every tender was rejected reads "closed, no award"

Award notices of the 1993–2010 text era that state an award date and say every tender was rejected now serve
`decision: clos-nw` on their result, as the same statement already did in the XML eras. They served no decision
(NULL) before: the rejection was recorded but never read.

## Unreleased (issue 510) — "the lot was not awarded" is no longer served as a winner

A winner slot that says the lot was not awarded (`Infructueux`, `Sans suite`, `Desierto`, `Lotto deserto`,
`Niet gegund`, `Not awarded`, …) is no longer an organization. Such a party is not served as a winner, buyer or
any other party, and an older-format award whose only "winner" it was now reads `decision: clos-nw` instead of
`selec-w`. About 17,000 organizations made of these phrases (one alone, `infructueux`, held over 3,000 winner
mentions), in every language including TED's own multilingual "not awarded", have been removed; each published
`organization removed`, and every Tender that served one published `tender changed` (with `lot changed` for its
affected lots). eForms results keep the publisher's own decision code. A "name" that states a void lot but also
names an award — a per-lot summary such as `Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré
infructueux`, or `infructueux, relancé et attribué à …` — is kept as it was.

## Unreleased (issue 506) — an award notice's framework values per lot are kept, and refuse scale slips

eForms award notices state a framework maximum and a re-estimated value per lot result (BT-709, BT-660). They are
now served in `amounts` at their lot under two new `field` values, `result_framework_maximum` and
`result_framework_reestimate`; they are never elected as a Tender's or a lot's `value`. A figure of €1 bn or more
that is exactly 100× (or 10ᵏ×) one of them can now be refused as a scale slip under the existing rules (their
corroboration and framework-total exemptions are unchanged). Measured before the release, three Tenders move: two
whose value was a framework maximum typed 100× too large now serve their estimate, and one whose €1.6 bn estimate is
exactly 1000× its own award notice's framework maximum now serves no value (refused is not corrected). `?currency=` matches a Tender whose only amount
in that currency is one of these rows. eForms-DE 1.x notices that state a notice-level framework maximum at the
root of the notice (`DE1-FrameworkMaximumAmount`, about 100 notices) now serve it as the Tender's
`framework_maximum`, as EU notices with the same element already did; their `value` can rise to it.

## Unreleased (issue 508) — text-era award notices of 2009-12 to 2010 serve their winners and values

Award notices published from 2009-12-02 to 2010-12-31, the last thirteen months of the tagged-text era, now serve
the winners they name. Those published from March 2010 on also serve their total value. Before, about 155,000 of these
Tenders served no winner, and about 96,000 of those from March 2010 on served no value, although the notice text
states both. One value is no longer served: where a 2004–2010 notice awards several lots under `LOT NO` headings and
states no total, one lot's figure is no longer served as the Tender's `value`.

## Unreleased (issue 507) — a carried prior-information part no longer outranks the contract notice's figures

When a prior information notice's parts carry into a later contract notice that has lots, a part's figure is no longer
elected as the Tender's `value` while the notice states any other figure the election accepts. It is still served on
the part's own row, and it is still the value when nothing else is accepted.

## Unreleased (issue 505) — a lot figure that is a scaled residual of its procedure total is refused

A lot's figure of €1 bn or more is no longer served, nor elected as the Tender's `value`, when it is exactly 100× to
1,000,000× what the notice's accepted procedure total leaves after the notice's other lots that state a figure in the
same field and currency, and that remainder is at least a hundredth of the total. Where the slip had also kept that
procedure total from being accepted as a framework total (issue 492), the total is now the Tender's `value`. Example: a £60 bn lot beside lots of £150 m and £250 m under a £1 bn procedure total. The figure stays in
`amounts`; the affected Tenders are re-folded and announced as corrections.

## Unreleased (issue 492) — a x100 scale slip is refused unless it is a framework total

A Tender's `value` (and what `min_value` / `max_value` compare) no longer elects a figure of €1 bn or more that is
exactly 100× another figure of the same Tender in the same currency, unless it is a framework total: a
procedure-level figure over two or more lots, whose smaller partner is a lot figure, and whose lots add up to
between a tenth of it and all of it. Each lot's served value follows the same rule, except that a lot figure is
kept only when its smaller partner only ever appears as a figure of other lots of the Tender (never procedure-level,
never this lot's own, never a part's or a lots group's) and the notice states an accepted procedure total at least as
large. A part's own figure is never kept this way.
×1,000 and up was already refused (issue 471). The refused figure stays in `amounts`. Affected Tenders are
re-folded and announced on the change feed as corrections.

## Unreleased (issue 495 unit 5) — a rate correction is announced

A corrected exchange rate re-derives stored EUR values in place (`eur_cents` on amounts, lot results, bids and
contracts). That used to be silent on the change feed. Now each Tender whose EUR values moved gets one
`tender` `changed` with `version: null`. Each lot whose own values moved also gets a `lot` `changed`, and when
the Tender's current version moved, so does every lot of that version. The values elected from them (the
Tender's and the lots' headline EUR value) follow at the next fold, which announces them the same way.

## Unreleased (issue 495 unit 4) — a re-derivation announces corrections, not a history replay

When a fold re-derives a Tender, for example after a fix to how notices are read, the change feed (`/v1/changes`,
SSE, webhooks) used to re-emit every version of that Tender under new cursors. A large refold wrote tens of
millions of change rows that changed nothing.

The fold now compares the re-derived Tender with what is stored and writes only what differs. A Tender whose
served reading moved gets two kinds of row, both with `version: null`:
- one `tender` `changed`;
- one `lot` `changed` for each lot whose reading may have moved. When the Tender's current version moved, that is
  every lot of the new version; when versions were inserted or removed mid-chain, every lot of the Tender.
A Tender that did not move gets no row at all.

Treat a `version: null` `changed` the way the docs already say: re-read the entity by id and upsert it. A client
that ignored such rows also missed every correction from now on, and no longer receives the replay that used to
carry them. Notices arriving as usual are announced exactly as before.

## Unreleased (issue 457 unit 3) — `/v1/sql` refuses recursion and EXCLUDE frames

`/v1/sql` now answers `400` to three constructs, naming the reason:

- `WITH RECURSIVE`;
- any CTE whose body reads its own name or the name of a CTE declared after it;
- an `EXCLUDE` window frame.

The engine already refused `WITH RECURSIVE` and `EXCLUDE`. The change a client can see is the
self-reference without the keyword: `WITH tenders AS (SELECT … FROM tenders) …` used to read the
base table and now answers `400`. Name the CTE something else. The next turso release executes such
a CTE recursively, so its meaning was about to change silently. These constructs are refused
because their work grows with the data's fan-out, which the time limit bounds but memory does not.

## Unreleased (issue 484 unit 3) — `?winner=` no longer counts a buyer as its own winner

`?winner=<org>` on `/v1/tenders`, `/v1/lots`, the SSE subscriptions and webhook
filters means "tenders this organization won" — and now leaves out the awards
where the winner **is the tender's own buyer**: the award notice names one
organization as buyer and as contractor (the same party, or the same name up to
case and accents). ~2,650 notices in the corpus have that shape (census 1981 at
stride 10); in the 63 read by hand it was a publisher repeating its own block in
the contractor slot (57) or a genuine in-house award to the authority's own
service (4). Neither is a supplier win to count.

**The award itself is unchanged and still served**, as published:
`lot_results[].winners[]` keeps the winner and adds `"is_buyer": true`, and the
detail's `parties[]` `winner` / `Tenderer` entry carries the same key (absent
everywhere else — additive). A tender the organization ALSO won on another,
unflagged award still matches. On `/v1/sql`, `v_lot_results` and `v_awards` gain
`winner_is_buyer` (1 or NULL): supplier statistics filter `WHERE winner_is_buyer IS
NULL`, as the filter does.

The flag is written when a tender is folded: a tender not re-folded since the
change reads unflagged (served and counted exactly as before) until the next
re-fold reaches it.

## 2026-09-11 (last) — the derived EUR column never reports zero

Completing the two entries below: the column also declines a conversion that
**rounds** to zero. Six Tenders reached €0.00 that way — published CZK 0.10,
CZK 0.12, HUF 0.79, HUF 1.48 and LIT 2.89, all real figures smaller than half a
euro cent, which the documented half-away-from-zero rounding puts on 0.

This is not a placeholder rule and the published figures are fine. It is about
the column's own vocabulary: 0 already means "no value was elected", so a derived
0 would have the column asserting €0.00 for a HUF 1.48 procurement. Declining
says "no value this column can express", which is the true statement.

So `min_value`/`max_value` never match on a zero, `?max_value=0` returns nothing,
and a zero in the derived column means one thing rather than three. Published
amounts are unchanged, as in both entries below.

## 2026-09-11 (later) — the value filters also skip a published 0.01 or 1.00

A fifth class joins the four below: **exactly one minor unit or one major unit**.
112,244 Tenders served one as their headline value, which makes this the largest
placeholder class in the corpus — larger than the zeros and the negatives put
together.

The evidence is a distribution with two spikes and nothing after them: 59,030
Tenders at €0.01, 53,214 at €1.00, and the third-placed value 36× smaller. Ten
currencies each spike at exactly one major unit against their own two (EUR 125×,
CZK 730×, DKK 412×). Most of it sits on `result_value`, on ordinary award notices
— subtypes 29/16/33/30, no concession marker — for a gymnasium renovation, site
security, painting works, school meals. You cannot award a contract for a cent.

**The rule is two exact values, not a floor.** `0.10` (1,463 Tenders) and `2.00`
(658) are a tail rather than a convention and are still elected.

**Published amounts are unchanged**, as ever: `amounts` carries every figure as it
arrived. What changes is that a Tender whose only figure is a one-unit token now
has **no known value** and is returned by neither value bound.

## 2026-09-11 — the value filters skip placeholder amounts, zero among them

`min_value`/`max_value` compare a derived EUR column, and that column now
declines to elect an amount that is a placeholder rather than a figure. Four
classes, each measured on the corpus rather than assumed (issue 366):

- **Negative** (~15,650 Tenders, 15,529 of them exactly −1.00, the eForms SDK's
  marker for a withheld figure).
- **Exactly zero** (24,647 Tenders). A 0 is an absence, not a price: 11,793 of
  the zero rows sit on `result_value` and 1,408 on `framework_maximum`, and you
  cannot award a contract for nothing or cap a framework at nothing.
- **A run of nine or more identical digits**, with or without the decimal point
  (€999,999,999.99 on street cleaning in a town of 47,000; PLN 22,222,222,222) —
  a form-width maximum typed by holding a key down.
- **Above €100 bn** EUR-equivalent.

Two observable consequences. A Tender whose only published amount falls in one
of these classes has **no known value** and is returned by NEITHER bound —
notably `?max_value=0`, which used to hand back thousands of contracts that are
not free. And the `value` in a payload can be a figure the value filters ignore.

**Published amounts are unchanged.** The `amounts` array still carries every
figure exactly as it arrived, negatives and zeros included; this entry is about
the derived column the filters compare, which layers beside the published
values rather than over them.

## 2026-08-27 — `min_value`/`max_value` compare derived EUR, not raw cents

The value bounds on `/v1/tenders` and `/v1/lots` now compare against the
tender's highest amount **converted to EUR at its publication date**
(ADR-0014), instead of comparing published cents numerically across mixed
currencies. Pass the bounds in EUR cents. Rows with no convertible amount no
longer match a value bound (previously a 20,000,000-SEK tender matched
`min_value=10000000` as if it were EUR). The old comparison was documented as
a caveat, not a contract; this entry retires it. Published values themselves
are unchanged and still served exactly as published — the conversion layers
beside them, never over them.

Also new on the same surface: the `currency` filter (published ISO-4217 code),
the `lang` selector (preferred language for picked titles), and
`eur_cents`/`awarded_eur_cents` columns on the `/v1/sql` tables and the
`v_tender_amounts` view.

## 2026-08-27 — /v1/sql surface promise stated; `currency_rates` queryable

ADR-0015 states what the SQL endpoint promises: allow-listed names, existing
columns and the response envelope are contract (renames/removals only with an
entry here); the dialect is described, not promised, and a canary test suite
pins representative query shapes against every build. `currency_rates` — the
full EUR-pivot rate series behind `eur_cents` (ECB daily 1999→, daily ECU
1993–1998 via Eurostat CC BY 4.0, irrevocable conversions) — joins the
allow-list as reference data.
