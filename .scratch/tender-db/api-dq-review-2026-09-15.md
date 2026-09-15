# API and data-quality review, 2026-09-14/15 — campaign record

32 independent lenses (12 API, 20 data quality) probed the public API at https://tenders.zebreus.click
and the serving database through bounded reads at rev 9e082fd. They returned 141 raw findings (56 from
the API lenses, 85 from the data-quality lenses), deduped to 104. Each survivor got two adversaries: one
re-running the evidence independently (the **reproducer**), one judging it against CONTEXT.md,
docs/domain.md and the 384-issue board (the **judge**). Outcome: **41 confirmed** (independently
reproduced *and* judged a real defect), **7 plausible** (one adversary held, the other dissented),
**56 refuted**. The fan-out itself was read-only throughout: GETs, bounded SELECTs, no writes, no
/admin calls, no load testing.

This file exists so nothing measured is lost and nobody re-discovers a refuted claim. Read the
refuted table before opening a new issue in this area.

## Confirmed and filed

41 findings: 7 high, 20 medium, 14 low. 29 were filed as new issues 385–398; 12 were appended as
comments to issues that already own the mechanism.

| Finding | Severity | Filed as |
| --- | --- | --- |
| F14 corrigendum date changes folded section-blind into `submission_deadline` (IV.2.6 validity, IV.2.7 opening); the max-election serves them as the tender's deadline, so long-closed tenders head "closes soon" | high | 385 |
| FTS publisher-reused ocids (`ocds-h6vhtk-02874b/c`) weld different buyers' procurements into one Tender, attributing other buyers' awards, winners and contracts to the head buyer | high | 386 |
| FTS contract values never parsed (award dates and contractPeriod dropped): every FTS `contracts[].value` is null and `amounts` empty although the release publishes them | medium | 386 |
| `name_prefix` search drops its upper bound when the prefix's last UTF-8 byte is 0xBF (`successor()` yields invalid UTF-8) and returns non-matching organizations | high | 387 |
| /docs lookup example `identifier=RO42283735&kind=VAT` returns an empty page: `kind` is a case-sensitive match on a lowercase scheme | medium | 387 |
| Reverse lookups by a prolific bidder/winner org cost 24–28 s cold and 3–5 s warm per page on /v1/tenders and /v1/lots — within 2 s of the 30 s bound | high | 388 |
| Lot headline `value` elects an exact-zero amount the tender-level election refuses: one response says the tender has no value and the lot is worth 0 | medium | 389 |
| /v1/lots `status=open` is decided by the tender's deadline while the lot row's `submission_deadline` is lot-scoped, so open lots are served with a null deadline | medium | 389 |
| `limit` outside 1..1000 is silently clamped by `clamp(1, MAX_PAGE)` instead of answering 400, against the spec's own minimum/maximum | low | 390 |
| `cpv`/`country` bound verbatim into LIKE: `%` and `_` act as wildcards and an empty string matches every row, all 200 with `ignored_filters []` | medium | 390 |
| /v1/notices/{id}/content is token-free but not CORS-open: no Access-Control-Allow-Origin, and its OPTIONS preflight is a bare 405 | medium | 390 |
| 405 responses on /v1 carry an empty body instead of the documented JSON error envelope | low | 390 |
| OpenAPI documents `deadline_after=now` as the "closes soon" query but `parse_instant()` 400s the literal (unix seconds / RFC 3339 only) | medium | 391 |
| 12 epoch-seconds columns (incl. `tenders.current_published_at`) carry no timestamp note in /v1/sql/schema although the docs promise every time column is flagged | low | 391 |
| `Accept: text/event-stream` on `/v1/notices?tender=<id>` answers a plain JSON page, not the documented stream | low | 391 |
| /v1/sql/schema prescribes `strftime(col,'unixepoch')` — the argument order its own column notes say returns NULL for every row (plus copy-paste column-name drift) | medium | 391 |
| /docs notice-content example contradicts the served shape and the OpenAPI (integer `section_id` / kind `root` / lang `deu`) | low | 391 |
| OpenAPI component schemas declare 2–6 fields for rows that serve 7–15; /docs relies on a field (`provisional`) the spec never declares | low | 391 |
| /v1/changes never validates `since`: garbage or negative restarts the feed from cursor 1 and a beyond-head cursor is echoed back as `last_cursor`, all 200 | medium | 392 |
| r209 TRANSLITERATED_ADDR block served as a party with role `TRANSLITERATED_ADDR`, minting a second provisional Organization for the same buyer | medium | 393 |
| Text-era Greek organization names served as Windows-1252 mojibake of ISO-8859-7 bytes and minted as provisional organizations under the mangled name | medium | 393 |
| Party roles served as raw per-era source element names (ECONOMIC_OPERATOR_NAME_ADDRESS, APPEAL_PROCEDURE_BODY_RESPONSIBLE, "Tenderer") instead of the documented 5-value vocabulary; r208 winners never get `role=winner` | medium | 393 |
| DÖE sdk-0.1 CPV codes served in mixed shapes under one scheme (2-digit division, bare 8-digit, 8-digit with check digit, glued multi-code strings) | medium | 394 |
| 7,158 DÖE eForms-DE notices carry TED's placeholder `publication_id` `00000000-1900` instead of their notice UUID (still occurring in current ingests) | medium | 394 |
| TED June 2025 (~72k notices) was never fetched — no 2025-06 package in the fetch registry — while the pipeline reports fetch complete and coverage never flags it | high | 395 |
| 2026 coverage reads 117.38 % because the vendored denominator is a 2026-07-17 snapshot while the corpus holds dailies through 2026-09-11; the footnote explains the opposite direction | low | 396 |
| Quarantine table glosses the benign "unreadable zip bundle" reason with the unmapped-content fallback text, contradicting its own class column | low | 396 |
| Text-era titles keep the source's 75-column hard line break (and, in 1993–94, the OJ boilerplate footnote) inside the title string | medium | 397 |
| /v1/notices/{id} serves the historical (reclaimed) quarantine object on parsed, projected notices — the read ignores `reprocessed_at` | low | 398 |
| FTS `bids.statistics[]` served as phantom `STAT-<id>` lot_results (decision/lot null, currency and decimals dropped) while the real awards carry empty statistics | medium | comment on 342 |
| Legacy CAN award-block contract titles (AWARD_CONTRACT / RES-n CONTRACT_TITLE) filed as tender-scope titles and win the title election over the procedure's own title | medium | comment on 343 |
| Date-only publication dates served as shifted instants (local midnight → T22:00/23:00Z of the previous day), so `published_at` precedes `dispatched_at` | medium | comment on 367 |
| `/v1/lots?winner=<org>` stalls to the 30 s service bound and 503s for ordinary winners while /v1/tenders?winner= answers in under 1 s | high | comment on 223 |
| /docs still describes provisional organizations as "single-mention" while served provisional rows carry many mentions | low | comment on 370 |
| OpenAPI 408 text says the query's server-side work "was abandoned" while the 503 text and the code say a timed-out query cannot be interrupted and pins a worker | low | comment on 370 |
| /v1/sql/schema says `tender_version_classifications.scheme` is "One of: cpv, nuts" but 9.7 % of sampled rows carry scheme `cc` | low | comment on 370 |
| /v1/changes `limit` is a raw change-log row window, not an event count: hidden lot_result rows make pages far shorter than `limit` (even empty with `more:true`) | low | comment on 211 |
| Allow-listed view `notice_withheld_fields` cannot be read even with LIMIT 1 — 408 at the cap, and the abandoned scan pins one of the two SQL workers | high | comment on 239 |
| Every column of every view (105 columns, 13 views) declared TEXT in /v1/sql/schema while the served values are integers | low | comment on 50 |
| A cpv/country prefix the reachability guard declines (non-ASCII, LIKE metachar, 5+ letters) that matches nothing walks the corpus to the 30 s bound and answers 503 "safe to retry" | medium | comment on 117 |
| Coverage era summary line divides one profile's held count by the whole-year denominators, contradicting every one of its own 100 % year rows | medium | comment on 229 |

Two rows sit slightly outside their issue's title: the DÖE CPV-shape finding rides on 394 (the DÖE
island issue) rather than a classifications issue of its own, and the CORS/preflight finding rides on
390 because its second half is the same bare-405 envelope defect.

## Plausible — one adversary dissented (NOT filed)

Seven findings where the reproducer and the judge disagreed. **None of these is filed**, and none
should be filed without the measurement named under it. They are the cheapest follow-ups on the
board: each is one measurement or one owner decision away from being filed or dropped for good —
no new probing campaign, no new lens.

**1. Unparseable `cursor` on id-ordered collections silently restarts at page 1** (low, api-lots,
issue 216 territory). `Params::after()` maps any cursor that fails to parse to 0, so `cursor=abc`,
`3x`, an i64 overflow, an empty string or a foreign sorted cursor all serve the no-cursor first page
with `ignored_filters []`, while the sorted-tenders and `name_prefix` paths 400 on the same input.
*Dissent: the judge.* It is a recorded, reasoned, test-pinned decision — the doc comment at
mod.rs:668-673 ("a truncated cursor never strands a client"), issue 216's 2026-08-16 hardening note,
and `a_sorted_cursor_never_crosses_into_another_ordering` at api.rs:1182 — and no published contract
promises a 400 (docs.rs:171 is about parameter *names*). The reproducer held, widened it to all four
id-ordered collections and to `Params::since()` on /v1/changes, and showed 216's justification
("restarts visibly") is false: the 200 body carries no marker.
*What would settle it:* nothing left to measure — both sides reproduced it and the decisive
observation is already in hand (no envelope field distinguishes a restart from a genuine first page).
It is a one-line owner decision on 216: 400, a `restarted` marker, or keep the lenience and document
it. If it is kept, `since()` on /v1/changes should be named in the same note.

**2. The OpenAPI request-body example for POST /v1/sql is the v_tenders aggregate the schema's
examples dropped** (low, api-sql). openapi.json:389 serves `SELECT source, count(*) FROM v_tenders
GROUP BY source` while /v1/sql/schema teaches three prod-timed base-table queries and /docs says
"do not filter a view".
*Dissent: the reproducer.* The literal evidence reproduces, but the claim built on it ("a client
copying this gets a 408 and pins a worker") is an inference: sql.rs:817-822 names no query, and three
of the system's own surfaces say the opposite — served schema note #4 lists this exact aggregate as
accepted, README.md:113-116 ships it annotated "fine (3 s)", and sql.rs:1811 pins it in a test. The
judge held: the drift is real and system-authored, issue 239 fixed sql.rs and /docs but never
openapi.json, and its acceptance line 252-253 for this exact query is unticked.
*What would settle it:* execute that one query against the box and time it. Both adversaries
declined (unbounded full-view scan; the prod-read rule). 408 → a two-surface defect (openapi.json:389
*and* README.md:116, whose "3 s" would be wrong). ~3 s → a one-string consistency nit plus an
assertion that the OpenAPI example is a member of the schema's example list.

**3. Text-era detail serves `submission_deadline` twice (date-only and date-time)** (medium,
api-cross-endpoint, issue 366 territory). Tender 8216604 serves two `submission_deadline` rows,
"2008-04-10" and "2008-04-10T10:00:00+00:00", because project.rs:278-279 maps both TXT-DD and TXT-DT
to that field.
*Dissent: the reproducer* — on the premise. TXT-DD is "Deadline for receipt of documents" and TXT-DT
"Deadline for receipt of tenders" (text-inventory.json; the 1993 fixture record ND 54692-1992 has
DD 19930215 / DT 19930601), so they are two published facts, not one at two precisions, and the
proposed fold would discard one. Measured over notices 3,340,000–3,360,000: 10,953 notices carry DD
or DT, 6,130 carry both, 4,851 (79 %) have DD strictly before DT, 1,217 are byte-identical, and only
142 — 2.3 % of pairs — have the claimed date-only + date-time same-day shape. The judge agreed the
premise is false and found a different, untracked defect underneath: the XML eras deliberately do
*not* map DD_DATE_REQUEST_DOCUMENT to `submission_deadline` (project.rs:267-268), so the text era
serves the document-request deadline under the submission label. In tender ids 8,210,000–8,220,000,
9,150 of 14,502 deadline-bearing version groups carry a second `submission_deadline` row and 8,818
span two or more distinct days.
*What would settle it:* count text-era versions carrying TXT-DD with **no** TXT-DT — the class where
the document-request deadline is elected as the tender's own deadline (DT fill is 49–61 % per
docs/research/ted-legacy-mapping.md:491). The judge was denied that `notice_dates` read. If the class
is non-trivial, file the corrected defect (drop TXT-DD from DATES or give it its own field, refold
the text era), not the finding as written.

**4. "Data quality" linkage and "Award linkage" panels show the same metric at different values**
(low, api-dashboard). Four legacy eras differ by 0.35–3.07 points (r209 82.9 vs 79.8 ≈ 51,000 award
tenders; r208 68.0 vs 65.2), plus eforms-de-2.1 at one display digit (98.0 vs 97.9).
*Dissent: the judge.* They are two deliberately different things, both labelled: the DQ panel is the
weekly snapshot stamped "measured N h ago" (issue 265, whose non-goal is explicit), the Award panel
re-measures on every concluded job (issue 191), and the 18 eForms/fts eras agree to the decimal
because the intervening fold touched only legacy eras. Issue 203 is direct precedent — a labelled
historical value beside a live gauge was resolved as not-a-bug. The reproducer held, added the fifth
era, and found /metrics sides with the weekly copy (2 of 3 surfaces carry the pre-fold number) and
/api/quality's last two runs hold byte-identical pairs, so the panel's ▼ delta cannot show a
three-point move.
*What would settle it:* read both panels once after the next weekly DQ run with no fold in between.
Convergence — with the ▲/▼ delta showing the move — confirms the judge and closes this for good;
persistence means the two surfaces really do diverge and it is worth a cross-reference line.

**5. Coverage "Held" and Pipeline "Processed" count the 306 quarantined-whole notices as held**
(low, api-dashboard). Processed 14,373,700 − tender_versions 14,373,394 = 306, exactly the actionable
quarantine class; `notice_counts_by_profile_year` groups by (fetch_id, profile) with no `parse_state`
predicate.
*Dissent: the judge.* It is the documented design: the query's own doc comment says coverage asks
"how much of what TED published that year do we hold — a question about packages", ADR-0004 calls a
quarantined notice "held whole", issue 189 is built on the coverage/quarantine separation, and issue
33 defines the funnel stage as "processed (notices)". 0.002 % of rows; no displayed ratio changes.
The reproducer held the arithmetic exactly and traced the mechanism through coverage.rs:415/434.
*What would settle it:* one bounded count of notices by `parse_state` per (fetch, profile) — denied
to both passes. Today's inflation is only the 306 quarantined, but the same unfiltered count also
reports identity-only `pending` rows as held, and issue 342's dry rung once recorded 7,243 fts
notices pending. If `pending` can be non-trivial mid-backfill, the label needs fixing; if it cannot,
this is closed as designed.

**6. Text-era abstract (AB) stored and served tagged ENG while carrying original-language text**
(low, dq-titles). 2005-vintage non-English originals serve a `description` row tagged ENG whose body
is Spanish/French/Polish prose (6 of 6 sampled: 3287506 SPA, 3060707 FRA, 3057663 FRA, 3084526 POL,
3116500 ITA, 3120481 FIN); text/rules.rs:62 is literally `("AB", Prose(Some("EN")))`.
*Dissent: the judge.* It is a publisher shape: in the 2005 EN daily `AB:` = CPV codes + the II.4
title the contracting authority wrote + the II.5 English CPV label, and TED left II.4 untranslated in
its English edition — the same sentence sits inside the ENG-tagged TX row of the same record
(verified on 3287506, 3060707, 3145158, 3120200). `lang` is documented as a *title* selector and
`texts[]` carries every stored variant, so no promise breaks; and the proposed fixes would mislabel
the era's pure-English AB rows (2001, 2007, 2008 samples). The reproducer held but narrowed the
scope: 1996, 2001, 2003, 2007 and 2009 samples are clean, so the mislabel is vintage-bound, not
era-wide, and "Abstract (English)" is the project's own inference (text/mod.rs: "no official spec of
the tagged format exists").
*What would settle it:* one bounded census of AB rows on non-English originals across 2004–2006 —
the unmeasured edges; both passes were denied SQL and worked from ~30 API reads. If the
original-language carrier is confined to the CPV-prefixed 2005 shape, a shape-bound rule exists; if
it scatters across vintages, the judge's "no single tag fits this mixed-language row" stands.

**7. r207/r208 title-less tenders serve `title` null while the OJ heading is parsed and unused**
(low, dq-titles, issue 368 territory). 17/1000 nulls in the 2011-06-01 window, 8/915 in 2014-06-03,
2/920 in 2016-06-01, 0/1000 in r209 and 0/1000 in the 1996 text window (where 992/1000 titles are the
heading shape); 15,604 r208 tenders corpus-wide per 368 unit 2.
*Dissent: the judge* — and the reproducer dissented from the defect framing too, holding only a
narrowed claim. The judge: issue 368 unit 1 rejected TED-TI_TEXT as a title on 2026-09-08 (it is the
CPV category label in 23 languages, already served as a classification) and unit 3 closed exactly
this residue on 2026-09-12 — "title null, description filled … Unit 3 closes as an answer". What the
reproducer held is narrower: the codebase carries two policies on the same heading shape —
project.rs:3150 ("a notice that carries no title element still has a title: the heading the OJ
published it under", served for internal-ojs and, via TXT-TI, ~99 % of the text era) against 368's
rejection — and 368's statement that TI_DOC is "excluded from TEXTS for exactly this reason"
misdescribes a fallback that does read it.
*What would settle it:* not a measurement — the counts already exist. It is 368's still-open unit-3
write-up: one paragraph reconciling the heading policy across eras, plus a correction to the issue-233
code comment (r2.0.x title-less notices are untouched because they emit no TI_DOC, not because they
have a title).

## Refuted — do not re-file these

56 findings did not survive. **Do not re-file these without new evidence** — every one of them was
reproduced or judged once already and cost a lens its budget.

One caveat that belongs in this record: **the result JSON preserved the reproducer and judge text for
only 1 of the 56.** For the other 55 both fields are null — the run recorded the claim and its lens
and nothing else, so this file cannot state per row whether it failed on reproduction, was ruled
source-published, was already fixed and verified, or fell out of scope. That is a gap in the record,
not a gap in the work, and a second round must not read these 55 as unrefuted. Re-running the judge
pass over these 55 titles alone (no probing, board and code only) would restore the reasoning
cheaply; see the last section.

All 55 unexplained rows come from the 20 data-quality lenses; exactly one API-lens finding was
refuted, and it is the one whose reasoning survived.

### Ruled already-decided by design — reasoning recorded (1)

| Claim | Why it did not survive |
| --- | --- |
| r209 TD=2 corrigendum with REF_NOTICE (099796-2018) stands as its own buyer-less "procedure" Tender instead of chaining to its original (068852-2018, tender 6128493) — api-tender-detail | Not actionable: the split is the deliberate, deployed outcome of issue 364 unit 6. All three curls reproduce, but the cited "original" 068852-2018 (notice 19680085) is a **prior information notice** — `/v1/notices/19680085/content` shows TED-TD_DOCUMENT_TYPE `0` and TED-FORM `1` (F01) — and 364's owner decision classifies a kind-less citation by its target's document type. Working as designed. |

### Reason not recorded in the result JSON (55)

Grouped by lens. Claims are shortened; the full titles are in the campaign JSON.

| Claim | Why it did not survive |
| --- | --- |
| **dq-amounts** — the −1.00 withheld placeholder served as money on `lot_results[].awarded` and `contracts[].value`, summed into a derived −2.00 | not recorded |
| **dq-amounts** — a revealed value (BT-198 publish-after date reached) still marked withheld and served as null, dropping the tender from head value and filters | not recorded |
| **dq-amounts** — about a quarter of −1.00 bids unmarked and served as offers of −0.01 even under the LotTender section the marker rule keys on | not recorded |
| **dq-amounts** — retired euro-predecessor codes (SKK/MTL/EEK/CYP) published after adoption converted at the irrevocable rate; ADR-0014 D4 says NULL | not recorded |
| **dq-amounts** — `?currency=GPB` (a single-row code present in the corpus) walks to the 30 s bound and 503s on /v1/tenders | not recorded |
| **dq-amounts** — eForms-era amounts carry `tax_basis` NULL on 100 % of rows although eForms value terms are net of VAT | not recorded |
| **dq-dates** — five impossible submission deadlines (years 0007/0016/0025/0206 and epoch 0) head `sort=deadline&order=asc`; the head election has no past floor | not recorded |
| **dq-buyers** — buyer served under the eForms technical id `ORG-0001` as the organization name although later notices publish the real name | not recorded |
| **dq-buyers** — r208-era tenders served with no party at all (1.07 % of a 10k sample; 0.11 % in r209; 0 elsewhere) | not recorded |
| **dq-buyers** — Liechtenstein state buyer re-countried to CH because its VAT id carries the CHE prefix | not recorded |
| **dq-buyers** — same buyer listed twice on one tender version when two ORG sections resolve to one org; one published name disappears | not recorded |
| **dq-country-stats** — nameless organization rows (`name = ''`) minted and served as buyers/canonical Organizations, 1.4M rows corpus-wide | not recorded |
| **dq-country-stats** — alpha-3 country codes minted again on organizations after issue 319's fold (13 rows from sdk-1.10 BT-514); Kosovo under three codes | not recorded |
| **dq-country-stats** — DÖE `buyer_country` NULL on 90–93 % of tenders every year 2023–2026 (TED 0.0 %), so per-country statistics are blind to the German portal | not recorded |
| **dq-results-winners** — r208-era award results never linked to their lots: every result serves `lot=null` although the notice publishes LOT_NUMBER | not recorded |
| **dq-results-winners** — `notice_subtype` null on every legacy-TED and DÖE sdk-0.1 version, so award notices cannot be identified through the API | not recorded |
| **dq-classifications** — TED-XML era: every additional CPV code served as `field=main`; the "additional" label never fires | not recorded |
| **dq-classifications** — FTS UK6/UK7/UK12 rows serve no NUTS place although the release publishes region under `awards[].items[].deliveryAddresses` | not recorded |
| **dq-classifications** — DÖE eForms-DE (E3) "Untranslated value (…)" free text served as a NUTS place code, then split on the comma by the list echo | not recorded |
| **dq-classifications** — `country=` documented as ISO-3166 alpha-2 but stored NUTS uses UK and EL, so `GB` returns an empty page with no `ignored_filters` signal | not recorded |
| **dq-tender-identity** — residual legacy weld: tender 8350926 fuses a 2015 Polish CN with a 2018 German procurement via a mistyped OJS citation | not recorded |
| **dq-tender-identity** — split on the DÖE numeric channel: a re-versioned notice (`<id>-2`, `-3`) minted as a second island Tender beside its `-1` Tender | not recorded |
| **dq-tender-identity** — publisher-constant BT-04 weld still served: tender 430681 is one 789-version, 75-lot Tender spanning 16 months | not recorded |
| **dq-tender-identity** — `publication_id` exact match needs the era-specific zero-padding: official TED/OJS spellings (615938-2024, 001662-2010) miss | not recorded |
| **dq-lots** — r209 per-lot estimated values (TED-VAL_OBJECT) parsed into notice_amounts and dropped by the fold; every r209 lot serves `value: null` | not recorded |
| **dq-lots** — r209 per-lot award values filed as tender-scope `result_value` rows beside the procedure total, so summing doubles the award | not recorded |
| **dq-lots** — r208 undivided contracts project zero Lots while r209 and eForms project one; /v1/lots empty for ~70–78 % of 2011–2015 tenders | not recorded |
| **dq-lots** — untitled legacy lots: 19 % of the 6.0M (r208) and 32 % of the 12.0M (r209) lot band serve `title: null` | not recorded |
| **dq-organizations** — text-era winner extraction mints organizations out of form headings, price labels and contact lines (21.5 % of the newest provisional rows) | not recorded |
| **dq-organizations** — one eForms notice mints one organization row per ORG section for the same (country, kind, identifier) | not recorded |
| **dq-organizations** — same-identity, same-name organization rows still stand after issue 329's E0 fold reported residual zero | not recorded |
| **dq-organizations** — organization served as `provisional=false` with no identifier, contradicting the documented flag semantics | not recorded |
| **dq-organizations** — placeholder strings `N/A` and `-` stand as high-mention Organizations, one per country | not recorded |
| **dq-text-era** — 2010 text-era award notices name the winner under a label variant the extractor does not know; 89 % of a 2,000-notice window has no lot_results | not recorded |
| **dq-text-era** — PIN-only text-era tenders (TD 0/M/P) served as kind `procedure` with `notice_subtype` null, indistinguishable from a call for competition | not recorded |
| **dq-multilingual** — `original_lang` serves raw non-ISO-639-2/T compound codes ("FR;NL", "FR NL", "DE IT", "GR"), spelled differently per era | not recorded |
| **dq-r208-r209** — R2.0.9 award notices publish DATE_CONCLUSION_CONTRACT but the projection never reads it; award-date coverage collapses 93 % → 8 % → 0.07 % | not recorded |
| **dq-r208-r209** — R2.0.7 (2011) notices: every organisation mention nameless because the name is published as TED-ORGANISATION direct text | not recorded |
| **dq-r208-r209** — F20 modification notices mint an empty-name provisional Organization with the raw role `DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR`; before/after values dropped | not recorded |
| **dq-eforms** — a DÖE award notice and its TED twin folded as two award rounds: lot_results, bids and contracts served twice on merged Tenders | not recorded |
| **dq-eforms** — about 7 in 10 eForms business terms a notice publishes (BT-105, BT-23, BT-01, BT-36, BT-765/766, BT-60 …) have no place on the Tender | not recorded |
| **dq-islands-sdk01-fts** — FTS parser skips party identifiers that carry an id but no scheme (e.g. Companies House SC213461) | not recorded |
| **dq-islands-sdk01-fts** — DÖE sdk-0.1: the buyer CompanyID field exists in the inventory but the projection never reads it; every mention is provisional | not recorded |
| **dq-freshness** — TED OJ S issues 2026/124–135 (1–16 July 2026, ~40k notices) never fetched: the monthly series stops at 2026-06 | not recorded |
| **dq-freshness** — DÖE completed-day packages 2026-07-20, 07-21 and 07-24 never fetched (lost in the Jul 21–22 restarts) | not recorded |
| **dq-changes-semantics** — OpenAPI/docs say `ChangeEvent.version` is "null for notices", but every null is a seq-less removal or in-place change | not recorded |
| **dq-document-types** — eForms X02 business-registration notices minted as `kind=procedure`, titleless, invisible to `?kind=registration` (X01 maps correctly) | not recorded |
| **dq-document-types** — r208 design contests (F12) and concession notices (F10) publish their deadline as TED-TIME_LIMIT_CHP, never projected | not recorded |
| **dq-document-types** — TED-era "General information" notices (TD=G, EEIG registrations) served as titleless procurement procedures | not recorded |
| **dq-document-types** — text-era PIN (TD=0) and qualification-system (TD=O/Q) notices: TED's dispatch+12-month placeholder served verbatim as `submission_deadline` | not recorded |
| **dq-amount-plausibility** — per-lot eForms award notices that relabel their lot as LOT-0001 all attach to the procedure's LOT-0001 (11 awards on one BGN 9,000 lot) | not recorded |
| **dq-amount-plausibility** — headline `value` reads the latest award notice's BT-161 while lot_results accumulate across rounds; served value 1.6x–6.1x below the sum in the same payload | not recorded |
| **dq-amount-plausibility** — /docs Amounts caveats describe `tax_basis` but REST amounts carry no `tax_basis` field at all | not recorded |
| **dq-filters-semantics** — /v1/lots `status=open` ignores the ten-year deadline horizon /v1/tenders applies (a lot echoing a 2999-12-31 deadline) | not recorded |
| **dq-filters-semantics** — `country=` cannot reach tenders whose current version carries no NUTS place even when the buyer's country is known (2.1 % of a 20k sample) | not recorded |

## What the fan-out could not cover

**The completeness critic did not run.** The session hit its limit; `critic` is null in the result
JSON. Nothing checked whether the 32 lenses collectively cover the API surface and the data model,
and no cross-lens omission pass exists beyond the mechanical dedupe from 141 to 104. Treat the lens
list as a convenience sample, not a partition.

**Eleven lenses lost their bounded-SQL leg entirely.** The session's permission classifier refused
the box helper as "Production Reads" for api-tenders-list, api-tender-detail, api-organizations,
api-cross-endpoint, api-performance, dq-titles, dq-results-winners, dq-classifications,
dq-tender-identity, dq-multilingual and dq-amount-plausibility — several of them on every attempt,
including plain bounded literal SELECTs. Five more lost part of it (dq-buyers, dq-organizations,
api-dashboard after its seventh read, dq-text-era, dq-r208-r209). Consequence: **every scope figure
those lenses report is a per-sample rate, never a corpus rate.** dq-multilingual states it plainly —
"no SQL evidence exists in this review".

**Lenses that ran shallow, and why:**

- **api-performance** (2 findings) — zero SQL, one measurement per figure, and it skipped the
  heaviest reverse-lookup cases on purpose: org 355 (FARMEXIM, 171k mentions) and any `winner=` on
  /v1/lots. The confirmed 24–28 s figure is therefore a floor, not a ceiling.
- **dq-tender-identity** (4 findings) — no census ran at all. It wrote out the five SELECTs a
  follow-up should execute: top-10-by-version-count per ~300k id window; `publication_id` spanning
  two tenders; `caused_by_notice_id` spanning two tenders; legacy citations whose S-issue disagrees
  with the target's TED-NO_DOC_OJS; DÖE numeric-channel `publication_id`s with VersionID>1 whose
  `-1` twin still stands.
- **dq-results-winners** (4 findings) — REST-only, eight details; corpus-scale rates unmeasurable for
  every check, and sdk-0.1, internal-ojs and text-era award details went unexamined.
- **dq-amount-plausibility** (3 findings) — every SQL-only check (currency_rates sanity, published
  currencies with no rate row, tax_basis across lots/versions, eur_cents on the four money loci, the
  200-tender lot-sum census) was replaced by a REST proxy or dropped.
- **api-organizations** (3 findings) — three named lens checks were never run: API `mentions` against
  a bounded COUNT over organization_mentions, the duplicate (name, country, identifier) census, and
  served identifier against `organization_mentions.raw_identifier/scheme`.
- **dq-multilingual** (2 findings) and **dq-freshness** (2 findings) — the first REST-only; the
  second lost six reads to a saturated runtime (the mid-range no-version scan over ids 30.37M–30.42M
  and the monthly-fetch listing since 2026-04 never ran).
- **dq-classifications** (5 findings) — rates are per 1000-row window only; internal-ojs was seen only
  inside a mixed 2008 window and eforms-de beyond subtype E3 not at all.

**Checks killed by the 10 s cap or a pinned worker** (these are known-unknowns, not clean results):
lot-level date ordering in dq-dates (duration_start vs duration_end, opening_date vs
submission_deadline, lot dates outside the tender range — both self-joins 408'd); corpus-scale field
reach, BT-501 → `organizations.identifier`, results density and lot-kind drops in dq-eforms;
`COUNT(*) FROM tender_versions`, the quarantine split by reason, the `parse_state` of the 306 held
notices and the per-(fetch, profile) grid in api-dashboard; the LIMIT-1 peeks of `v_tenders`,
`v_lots`, `v_organizations` and `v_tender_current` in api-sql, deliberately skipped because a worker
was probably still pinned by the abandoned `notice_withheld_fields` scan — which is confirmed finding
239 biting the review itself.

**Surfaces no lens probed:** SSE and streaming end to end (named unchecked by api-changes, api-lots,
api-tenders-list, api-robustness, api-docs-vs-reality and dq-changes-semantics), `include_data`,
Last-Event-ID resume, webhooks, POST /v1/sql body validation, /v1/me, /health/deep, /_source, the
Swagger/Redoc links, the 429 envelope and anything requiring load (no load testing was permitted),
and the `lang`/`currency` selectors' effect on picked titles.

**Eras and sources sampled thinly or not at all:** internal-ojs (unsampled by dq-buyers, not located
by dq-lots, absent from dq-document-types' probe); fts:ocds-1.1 (amounts, lots, buyers and document
types all unsampled — and only one month, 2025-06, is ingested, so FTS cross-month chaining cannot be
tested yet); eforms-de-1.x beyond subtype E3 (the de-2.1 window shows no head rows because TED copies
win under ADR-0003, so any served-kind conclusion there would be about the TED head); text-era and
r208 lot sums; r208/r209 award-date and winner coverage, measured on three 10k windows (2014-08,
2018-09, 2021-10) with 2016, 2017, 2019, 2022 and 2023 unsampled.

**What a second round should target, in order:**

1. Restore the refutation reasoning for the 55 rows above — board-and-code only, no probing. It is
   the cheapest work on this list and it is what stops the next campaign re-finding them.
2. Run the campaign with the production-read path actually granted, and re-run the eleven SQL-denied
   lenses as pure census passes: their findings exist, only the scopes are missing.
3. Execute dq-tender-identity's five SELECTs verbatim.
4. Take the seven plausible measurements listed above; five are single queries or a single re-read.
5. Cover the untouched surfaces as one lens each: SSE/streaming, webhooks + token-gated endpoints,
   and rate limiting under load (which needs a window where holding the SQL workers is acceptable).
6. Two loose ends worth one read each: the FTS results with `lot=null` on 18 of 37 real results
   (one notice content settles whether the lot is absent per release), and tender 4490098's
   three-magnitude figure, which needs a gated archive read — ted.europa.eu answered 202/empty, and
   the lens marked it "worth a gated archive look under 366 unit 5".

One incidental correction the campaign produced: the board's example ids have drifted. Issue 116's
verification id 7161565 now serves `lots=1`; the 2,604-lot tender is 6978910.
