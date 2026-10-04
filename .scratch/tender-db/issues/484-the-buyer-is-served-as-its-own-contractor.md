# 484 — the buyer is served as its own contractor (the winner slot repeats the authority)

Status: ready-for-agent — NEXT: unit 2 LANDED, NOT DEPLOYED (uncommitted; see "Unit 2 — landed"): gate
(`ops/check.sh`), commit with 485, deploy, then the re-parse runbook in that section — probe one text package
holding 2002406 and re-read `/v1/notices/2002406/content` (expect `Montte`), wet `text` + `fts:ocds-1.1` re-parse with
`reclaim_only`, ONE `project`, re-run `buyer-role-census` at stride 10 against job 1942, then the Verify. No dry
re-parse diff exists (measured around the wet run instead). Unit 3 (projection): flag `is_buyer` on buyer-equal
winners, exclude from supplier statistics.
Was status (2026-10-04 sample read): fix unit (unit 2, parse) + the phone-as-lot-reference by-catch (now 485).
Kind: data correctness (awards, parties)
Relates to: 485 (the by-catch, landed in the same change), 483 (found by its census: the `contractor-*` classes), 456 (mention binding), the served `awards[]`

## What is wrong

The 483 census (job 1942, stride 10, 2026-10-03; `.scratch/tender-db/483-roles/census-1942.json`) flagged 263
notices where the buyer is ALSO the contractor (`contractor-org-same-name` 135, `contractor-name` 125,
`contractor-same-section` 3) — ~2,600 across the corpus. In every one of the 63 samples the buyer is the right one
and the winner is the doubtful mention:
- **2002406** (legacy TED text, 2002): `TXT-CO` reads "Supplier(s): Contrato n° S-036/02-DJ.\n1: Gobierno Vasco, A
  la atención de Mesa de…" — the legacy text parser takes the first name after "Supplier(s):", which here is the
  authority's contact line. Spanish legacy notices dominate `contractor-name` (Ministerio de Justicia, Consejería de
  Educación y Ciencia, Autoridad Portuaria de Málaga, Universitat de Barcelona). Likely a PARSE defect.
- **23011756** Stadt Hilden (F20 modification): the contractor block names the city (the contract moved to the
  city's own company, per the modification text). Possibly the source's own entry.
- **SPMS ×3, Comunidade Intermunicipal ×3, Comune, Município** (legacy): central purchasing bodies and in-house
  awards, where buyer = contractor may be correct as published.
- **UK award updates** (Kirklees, Clackmannanshire, Cabinet Office): the same Organization referenced as buyer and
  supplier.

Served effect: `awards[].supplier` names the buyer; supplier statistics count authorities as winners.

## Census (unit 1, dry)

Reuse the `buyer-role-census` walk (its `contractor-*` classes already find these): add per-cell samples of the RAW
winner field (legacy `TXT-CO` text, `ADDRESS_CONTRACTOR`, eForms `Tenderer` / `LotTender`) so the read can tell
"the parser picked the wrong line" from "the notice says so". Fix by source: parser defects at parse (re-project),
genuine in-house awards left as served (an in-house award to oneself is correct data — mark it, don't drop it).

## Sample read — 2026-10-04

All 63 census samples (job 1942's `contractor-name`, `contractor-org-same-name` and `contractor-same-section`)
were read against the notice. Three readers did the first pass, giving 41 source-says-so, 2 parser-defect,
1 in-house and 19 unclear. Their production GETs were refused, which is why 19 stayed `unclear`. The public,
read-only `GET https://tenders.zebreus.click/v1/notices/{id}/content` needs no box access, and it was used to
spot-check 23 samples one at a time: both parser defects (both confirmed), 2 other verdicts (24211216 and
46811758, both confirmed) and all 19 unclear ones. That endpoint serves the parsed sections, not the source
file. For the text era its `TXT-TX`/`TXT-CO` is the published prose word for word. For XML and OCDS sources it
shows the section graph, one step away from the raw file.

**Verdicts after the spot-check:**

| verdict | n | what it means |
| --- | --- | --- |
| source-says-so | 57 | The notice's own contractor block (V.3, `ADDRESS_CONTRACTOR`, eForms `Tenderer`, OCDS `suppliers[]`) names the buyer, and the parse is faithful. Most are publisher errors: competitive awards with 2–34 offers, and central purchasing bodies (SPMS, CIM Viseu/Médio Tejo, Comunidade) that do not supply goods themselves. |
| in-house-or-self-supply | 4 | The buyer is really one of the winners: 1200610 Staffanstorp (its own Städservice won lot 1 at 2,750,000 SEK), 2808875 Cardiff (the Asylum Support Team is 1 of 19 providers), 24210321 Morsø (its own road service won 5 of 12 winter-service lots, same CVR), and 46811758 Clackmannanshire (1 of 22 partnership providers). |
| parser-defect | 2 | 2002406 (legacy text) and 46804384 (FTS). |
| unclear | 0 | |

Changes from the readers' verdicts. Re-reading resolved all 19 unclear samples:
- **Unclear → source-says-so (16):** 2208215, 2209731, 3002722, 3009398, 3016761, 3205106, 2602206,
  2605797, 2013295, 23012255, 23408177, 24608868, 46802818, 47004844, 47006179, 47014844. In the Spanish
  sectioned forms, V.3 / V.1.1 itself repeats the authority's contact block, often with `A la atención de` or
  `Att:`. In 2208215 the 5th award block is the file reference `C1-VIG02/2002` and names the ministry.
- **Unclear → in-house (3):** 1200610, 2808875 and 24210321. Issue 484 listed 1200610 beside 2002406 as a
  suspected parse defect, but it is in-house. The lot-keyed prices confirm it.
- **2013295 (source-says-so, real supplier in the text):** the source names the hospital and then, in the same value,
  `Lieferauftrag: Fa. Salesianer Miettex`. No general rule can reach that supplier.
- **Stadt Hilden (23012255 and 23408177 read; 23011756 has the same buyer and form):** these are F20 notices extending a cleaning contract by
  6 months (`Vertragsverlängerung von sechs Monaten`). The city's new subsidiary SHB is a future
  facility-management partner, not the contractor. "The contract moved to the city's own company" (above) is
  wrong. The V.2.3 and VII.1 contractor blocks both name the city, with SME=1, so the publisher made an error.

**By Source × profile:**

| Source / profile | n | source-says-so | in-house | parser-defect |
| --- | --- | --- | --- | --- |
| TED legacy (text era + XML R2.0.x) | 50 | 47 | 2 | 1 (2002406) |
| TED eForms (subtype 29) | 5 | 4 | 1 | 0 |
| FTS award+contract | 3 | 2 | 0 | 1 (46804384) |
| FTS awardUpdate+contractUpdate | 4 | 3 | 1 | 0 |
| FTS UK7 | 1 | 1 | 0 | 0 |

By census class: `contractor-name` 27 S / 2 I / 1 P; `contractor-org-same-name` 29 S / 1 I; and
`contractor-same-section` 1 S / 1 I / 1 P. **Parse defects are about 3% of the class (2/63).** Scaled from
stride 10, that is on the order of 60 notices in the corpus. The rest is what the notices say.

### Parser defect 1 — text era: the numbered value is cut at 256 bytes, and the authority's contact entry wins (2002406)

The source text (TED 190812-2002, item 6) reads `Supplier(s): Contrato n° S-036/02-DJ. 1: Gobierno Vasco, A la
atención de Mesa de Contratación … mv-ruiz@ej-gv.es, … www.ej-gv.net. 2: Montte, Polígono Industrial 10,
E-20200 Beasain …`. The real supplier is **Montte**, as entry `2:`. Entry `1:` repeats the item-1 awarding
authority, with the same e-mail and fax.

Code path (`crates/ingest/src/text/parse.rs`): `awarded_names` (597) bounds the value with
`window = rest[..NAME_WINDOW]`. `NAME_WINDOW` (103) is 256 bytes. The authority's contact entry alone is
longer than that, so `2: Montte` falls past the window and is never seen. `winner_segments` (526) splits at
`. 1:`. The first segment, `Contrato n° S-036/02-DJ`, has no comma and no item stop, so it is refused. That
leaves `Gobierno Vasco` as the only name. `Emit::award` (1594) files it as `TED-OFFICIALNAME` under
`TED-ADDRESS_CONTRACTOR`, and `legacy_role` (project.rs 6775) turns that into `winner`. Served result:
`GOBIERNO VASCO` as winner, and Montte missing.

Fix design. All of it goes in `awarded_names`, so the function stays pure over the body:
1. **Read the numbered value to its own next item, not to 256 bytes.** When the label follows a numbered item
   `N.` (the pre-2004 form: `6.  Supplier(s):`, `7.  Service provider(s):`), the value ends at ` {N+1}. ` or
   at an existing `ITEM_STOPS` entry, searched within a larger bound (2 KiB, which still caps the quadratic
   case in the window comment). Sectioned forms keep the 256 window and the V.x stops. A blanket larger window
   would pull ` 8.  Price(s): 1: 2 750 000 SEK` into the value (1200610), so the bound must be the item's own
   successor.
2. **Refuse a contract-reference segment.** A segment that opens with `Contrato n°`, `Contract No`,
   `Contrat n°`, `Vertrag Nr` or `Contratto n.` is a reference, not a name. `CONTRACT NO` already gets the
   same treatment as a prefix hop.
3. **Drop the authority's contact entry, and only that.** Inside one multi-segment value, drop a segment when
   all three of these hold:
   - its name folds equal to the body's awarding-authority name (item `1.  Awarding authority:` / `I.1)`, read
     from the same body);
   - it repeats that item's e-mail address or URL host;
   - at least one other named segment survives.

   The e-mail/URL condition keeps 1200610's `1: Staffanstorps kommun, Städservice` (different phone, no shared
   address), which is a genuine in-house lot. The condition that another segment survives keeps every
   single-entry source-says-so (3002722, 3009398, …) as published.

   An attention marker (`A la atención de`, `Att:`) is **not** a usable signal. 2808875 prints `Att: <person>`
   on all 19 genuine Cardiff providers.

Tests to add next to the existing `awarded_names` fixtures:
- 2002406's item 6 should give `["Montte"]`.
- 1200610's item 7 should give `["Staffanstorps kommun", "Clean Service System AB"]`.
- 3002722's V.3 should give `["Consejería de Educación y Ciencia"]`, unchanged.

### Parser defect 2 — FTS: one party id under two names becomes one section (46804384)

The OCDS release has two parties with the same `id` `GB-COH-01624297`, which is Microsoft Limited's Companies
House number: the buyer `The Council of the Borough of Kirklees` and the supplier `Microsoft Limited`.
`crates/ingest/src/fts/parse.rs` (200–245) opens `ORG-{pid}` per party. The second party appends to the same
section, which ends up holding `BT-500 ['…Kirklees', 'Microsoft Limited']` and the identifier twice. The buyer
role (`OPT-300-Procedure-Buyer`) and the award's `OPT-300-Tenderer` (318–338, `ORG-{sup_id}`) both point at
it. The first name is served for both roles.

Fix design:
- In the parties loop, key the section on (id, folded name). The first party keeps `ORG-{pid}`, and a
  differently named party with the same id gets `ORG-{pid}#2`. Role references push the section of the party
  that carries the role.
- `PartyRef` (1107) gains `name`. An award supplier resolves to the section with the same id and name, and
  falls back to the first section for that id when the name is missing or matches nothing.
- An identifier that two differently named parties claim in one release goes only on the party whose role is
  supplier/tenderer. If no such party exists, or more than one does, it goes on neither. Without this rule the
  identifier fold would merge the two sections again downstream.
- Test: 46804384's release should give the buyer Kirklees and the winner Microsoft Limited as two sections,
  with `GB-COH-01624297` only on the supplier.

### By-catch (not this issue): phone numbers read as lot references mint junk winners

The same text path that serves 3009398, 3016761, 3205106 and 2808875 mints organizations named
`URL: www.puertomalaga.com. Fax 952 12 50 02` and `Fax 0044 2920 644615`. In 2808875 there are 19 of them, all
served as winners. The cause is in `winner_segments`/`lot_prefix_len`: `Tel. 952 12 50 00.` passes as a
digits-only lot reference, so the `.` after `Tel` splits the value. The next segment then has no comma, is
bounded by `V.4)`, and becomes a name. Fix: `lot_prefix_len` refuses two digit groups separated only by
whitespace (phone notation; real lot lists use `,` `/` `-` `and`), and `plausible_name` refuses a candidate
that opens with `Fax`, `Tel`, `URL:` or `E-mail`. **File it as its own issue and land it in the same change as
defect 1.** It is the same function and the same 3.8M-notice re-parse, so landing them together costs one
re-projection instead of two.

### What to do with source-says-so and in-house (61 of 63)

**Serve them as published, and flag them. Do not drop them.** The winner slot holds what the notice says, and
an in-house award to oneself is correct data. A notice alone does not reliably separate in-house awards
(Morsø, Staffanstorp) from publisher error (Pombal, SPMS): both are "the buyer's own org in the contractor
slot".

So the projection marks the served winner instead of judging it:
- `awards[].supplier` / `parties[]` gains `is_buyer: true` when the winner mention resolves to the same
  organization as a buyer of that notice, or folds to the same name.
- Supplier statistics (winner counts and amounts per organization) exclude `is_buyer` wins by default.

That is a projection unit, separate from the parse fix, and it covers every source at once.

Recoverable subsets. The notice itself names a different real supplier in these, and each is a projection
rule of its own. They are listed for later, not for this fix:
- **eForms:** `Tenderer` points at a copy of the buyer org, while the `TenderingParty`'s `OPT-211` (and often
  an unreferenced Organization) names the real winner: 24211216, 24609234, 24614290. Rebind to the
  Organization whose BT-500 equals `OPT-211`, or to the `OPT-211` name alone.
- **F20:** the V.2.3 contractor equals the buyer while the VII.1 `DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR`
  names another org (21605540, Real Dolmen). Prefer the VII.1 contractor.
- **FTS framework updates:** the `See Contracts Finder Notice for full supplier list` placeholder plus the
  buyer's own party (46814545, 46802818). Drop the placeholder name as a non-organization.
- **Free text only:** NHS England's INFO_ADD supplier list (20019562), the F13 prize winners (20612475), and
  the F15 `D_JUSTIFICATION` contractor (22614978, Hilton Nursing Partners). These are not extractable by rule.
  The `is_buyer` flag is the whole treatment.

## Unit 2 — landed (not deployed) — 2026-10-04

Uncommitted in the worktree with 485. Files: `crates/ingest/src/text/parse.rs`, `crates/ingest/src/fts/parse.rs`,
`docs/operations.md` (no dry re-parse: how to measure around one), `.scratch/tender-db/issues/485-…`.

**Defect 1 (text), in `awarded_names`, pure over the body as designed:**
1. A label that heads a NUMBERED item (`numbered_item_before`: ` N. ` within 96 bytes before the label, nothing
   with `:` or `.` between) ends its value at the next item marker (`successor_marker`: ` {N+1}. ` first, else
   N+2 … N+4, since the forms skip empty items), searched within `NUMBERED_VALUE_MAX` = 2 KiB; only a SECTIONED stop
   (`V.x)`, `CONTRACT NO`) may cut it earlier — the numeric ` 7.`/` 9.`/` 10.` stops are ignored there. No marker
   found: the old 256-byte window with the old `item_end` semantics, unchanged, as for the sectioned forms.
   1200610's ` 8.  Price(s): 1: 2 750 000 SEK` stays out.
2. `contract_reference_len` HOPS a segment-opening contract reference (`Contrato n°/nº/no/n.`, `Contract No`,
   `Contrat n°/nº/no`, `Marché/Marche n°/nº/no`, `Vertrag Nr`, `Contratto n./n°/nº/no`; not when a letter follows:
   `Contract Northern Ltd` is a name) through its `:` or sentence `.`, and reads what follows; a segment that is
   only a reference is skipped. Needed now, not just defensive: with the successor bound, `Contrato n° S-036/02-DJ`
   is a BOUNDED comma-less segment and would otherwise be minted.
3. `awarding_authority` reads the body's authority (`Awarding authority:` / `Awarding entity:` / `Contracting
   entity:` to its successor item, else `I.1)` to `I.2)`/`SECTION II`): the folded name before the first comma,
   and its e-mail addresses and `www.` hosts. Inside one value, a segment is dropped when its name folds equal,
   it contains one of those contacts, AND another named segment survives.

**Defect 2 (FTS):** `party_sections` keys sections on (id, folded name): first name → `ORG-{id}`, each other name
under the same id → `ORG-{id}#2`, `#3`…; the same (id, name) twice still shares one section. Role refs use the
party's own section. `PartyRef` gained `name`; `supplier_section` resolves (id, name), falling back to the same-id
section whose party is a supplier/tenderer, then to the id's first section (= the old `ORG-{id}`).
`withheld_identifiers`: only among sections split out of ONE party id, an identifier they share is withheld from
the non-supplier claimants when exactly one claimant is a supplier/tenderer; with none or several, all keep it.
Identifiers shared across DIFFERENT party ids are never touched. Releases with unique party ids produce exactly
the old output.

4. (review) `winner_segments` also splits at a `.` that ends a contact block (`entry_after_contact`: the token
   before it is an e-mail or `www.`/`http` host) when what follows is an upper-case `Name,` with no `:` before the
   comma and a `Tel`/`Fax`/`@` in its text — 2002408's unkeyed `… URL: www.ej-gv.net. Profinsa, …`.
5. (review) `lot_prefix_len` accepts `N)` (one run of digits) as a lot key: `1) CGC … 2) Furic …` (900123).

**By-catch (485):** `lot_prefix_len` refuses whitespace-separated digit groups; `plausible_name` refuses contact
lines (`Fax`/`Tel`/`Telefax` + number, `URL:`, `E-mail:`).

**Tests** (fixtures are the exhibits' stored values from `/v1/notices/{id}/content`, verbatim with the wrap):
- `text::parse::tests::the_authoritys_contact_entry_is_not_its_own_contractor` — 2002406 → `["Montte"]` (and
  through `parse`, the only `TED-OFFICIALNAME`); 1200610 → `["Staffanstorps kommun", "Clean Service System AB"]`;
  3002722 → `["Consejería de Educación y Ciencia"]` unchanged; 2002406 without entry 2 → `["Gobierno Vasco"]`
  (single entry kept as published).
- `text::parse::tests::a_phone_number_is_not_a_lot_reference` — 485's exhibits.
- `fts::parse::tests::one_party_id_under_two_names_is_two_sections` — 46804384's release (029468-2026) reduced to
  its parties and award, REBUILT from the stored section graph (the raw member is not reachable off the box): buyer
  `ORG-GB-COH-01624297` = Kirklees with no identifier, tenderer `ORG-GB-COH-01624297#2` = Microsoft Limited with
  `GB-COH-01624297` once.
- `fts::parse::tests::a_shared_party_id_falls_back_to_its_supplier_section` — unnamed / misspelt supplier ref, no
  supplier (or two) among the claimants, one party published twice.
- `fts::parse::tests::an_identifier_shared_across_distinct_party_ids_is_untouched` — the old output for distinct ids.
- `text::parse::tests::the_sibling_shapes_of_the_contact_entry` — 2002408, 2002409, 900123, 1200611.
- `text::parse::tests::a_skipped_successor_does_not_run_the_value_into_the_prices`.

**Focused runs — CORRECTED by the review.** The first pass's claim "every integration binary green, no committed
text/FTS expectation moved" was FALSE: its filters (`fts`, `text`, `parse`) match no integration-test name, so
`tests/text.rs` ran 1 of 13 and `tests/fts.rs` 0 of 7, and the two exact 1993-daily gates were red (117 → 150
winners, 79 → 87 dates). See "Review fixes" for the re-pin and the runs that actually cover them.

### Review fixes (2026-10-04)

1. **FTS identifiers (2 × major):** `contested_identifiers` keyed on the identifier VALUE across all parties, so
   two DIFFERENT party ids sharing a Companies House number lost it (both suppliers: dropped from both; one
   supplier: stripped from the other) — the same company listed twice no longer merged. Replaced by
   `withheld_identifiers` (above): contested only inside one split party id, withheld only from non-suppliers when
   exactly one claimant is a supplier. Test `an_identifier_shared_across_distinct_party_ids_is_untouched`
   (tenderer/buyer/payer beside a supplier under distinct ids: both keep it, no `#` section); the fallback test now
   asserts both claimants keep it with no supplier or two.
2. **Text: contract reference threw away the whole segment (major):** 2002408 (`Contrato n° S-037/02-DJ. Gobierno
   Vasco, … URL: www.ej-gv.net. Profinsa, …`, unkeyed) went from the junk winner to NO winner. Now the reference
   is hopped (item 2 above) and the contact-block split (item 4) separates Profinsa; the authority-contact drop
   then leaves `["Profinsa"]`.
3. **Text: numbered bound (minor):** successor of any higher number (N+1 … N+4), numeric stops ignored when found,
   old window/`item_end` otherwise (item 1). Test `a_skipped_successor_does_not_run_the_value_into_the_prices`.
4. **FTS supplier fallback (minor):** a misspelt or nameless supplier reference fell back to the id's FIRST section
   — the buyer in 46804384's order. Now the supplier-role section first. Test
   `a_shared_party_id_falls_back_to_its_supplier_section` (unnamed and `Microsoft Ltd` → `ORG-GB-COH-01624297#2`).
5. **`Marché n°` and `N)` (minor, cheap, done):** 2002409 → its 7 suppliers (was `Marché n° 03/010002: 1)
   Biotronik France`), 900123 → `["CGC", "Furic (Jules et Alain) et Fils"]` (was `1) CGC`). Test
   `the_sibling_shapes_of_the_contact_entry` (with 2002408 and 1200611).
6. **1993-daily gates (blocker):** diffed HEAD vs the change per record (temporary harness, removed): 5 of 199
   records move, all additions real lot winners the 256-byte window cut off — 55170 0 → 16 (`1: Valdis Ouest SA.
   2: Estrier. …`), 55403 3 → 12, 54833 3 → 8, 55361 4 → 6, 55346 4 → 5; no name lost, none an address/price/lot
   reference. Dates 79 → 87 = the new winners of the three DATED records (the date lands on every result, one result
   per winner: 54833 +5, 55361 +2, 55346 +1). Re-pinned in `tests/text.rs` with that explanation. Known remainder in
   the same fixture: 55361 keys its lots `7.` … `11.`, which collide with the successor marker ` 7. `, so entries
   7-11 stay unread (they were unread before too).
7. **Contact-match safeguard untested (minor):** the Vasco value with entry 1's e-mail/URL replaced keeps
   `["Gobierno Vasco", "Montte"]` (in `the_authoritys_contact_entry_is_not_its_own_contractor`).
8. **Doc comment (minor):** `with_delivery` has its doc back; `parse_ok` has its own.
9. **Re-parse expectation (2 × minor):** step 5 below rewritten — the wider bound RECOVERS winners across the whole
   numbered era, so totals go UP.

Also in the change: `next_item_marker` already existed (the price reader's); the new helper is `successor_marker`.

Runs after the review fixes (gate flags + gate package set, to a file, exit read): `… --lib --test text --test fts
--test project --test data_quality --test fetch` → GATE-EXIT=0 (lib 375/3/150/181, text 13/13, fts 7/7, project 88,
data_quality 11, fetch 44); after removing the temporary diff harness, `-- the_sibling a_skipped the_authoritys
a_phone_number shared_party an_identifier_shared one_party_id 1993` → GATE-EXIT=0. `ops/check.sh` still NOT run —
gate before commit/deploy.

**Prod re-parse (after gate + deploy).** There is no dry re-parse diff job: `reparse` writes in place and reports
counts only (see docs/operations.md, "There is no dry `reparse`"). Not built here, per scope. Steps:
1. Baseline: job 1942 (`buyer-role-census` stride 10) is the before; note the Verify's open output and
   `/v1/notices/{2002406,2808875,3009398}/content` winners (above and in 485).
2. Probe: find the text package that holds 2002406 (bounded `/v1/sql` metadata read of its `fetch_id`), then
   `{"kind":"reparse","profiles":["text"],"packages":1,"after":<that fetch_id − 1>,"reclaim_only":true}`; re-read
   `/v1/notices/2002406/content` — expect `ORG-2 TED-OFFICIALNAME Montte` and one `RES-`. Require 0 unmatched,
   0 re-keyed, 0 now failing.
3. Wet: `{"kind":"reparse","profiles":["text"],"reclaim_only":true}` (~215 packages, ~2 h on job 1595's rate) and
   `{"kind":"reparse","profiles":["fts:ocds-1.1"],"reclaim_only":true}` (~314k notices, job 1813's shape). Read
   unmatched / re-keyed / now-failing on both (expect 0: no identity derivation changed). Don't overlap the daily
   (operations.md, issue 404) — neither change moves `publication_id`, so the race is cost only.
4. ONE `{"kind":"project"}` — the text era alone is far over the 500,000 cap, so the FULL fallback (~5.2 h).
5. After: `{"kind":"buyer-role-census","stride":10}`, read PER DIRECTION, not as one delta:
   - **down:** the parser-defect `contractor-*` rows (the authority-contact entry beside a real supplier, ~3% of
     the class) and junk winners — price lists (`Price(s): 1: 208 332 GBP p.a.`, 1200611), contact lines (485),
     `Contrato n° …`/`Marché n° …` strings.
   - **UP, expected:** total text-era winner rows, organizations and awards. The successor bound reads multi-entry
     numbered values to their end: on the committed 1993 daily +28% winners (117 → 150; 55170 0 → 16, 55403
     3 → 12); and 2002409 0 → 7 real suppliers. Some newly read entries are in-house (`1: Staffanstorps kommun,
     Städservice`), so the `contractor-*`/source-says-so classes can RISE too — not a regression.
   - **measure it:** before step 2 and after step 4, count text-era winner rows by a bounded read (or the census
     totals) so the gain is a number, not a surprise.
   Then the Verify below and 485's; extra exhibits: 1200611 → `[Serco Limited, Serviceteam Limited]` (was 4 with
   two price strings), 2002408 → `[Profinsa]` (was `Contrato n° S-037/02-DJ. Gobierno Vasco`).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/2959772 | jq -c '[.parties[] | [.role, .organization_name]]'

(Tender 2959772 is notice 2002406, TED 190812-2002.)
- **open** (2026-10-03): `[["buyer","Gobierno Vasco"],["winner","GOBIERNO VASCO"]]` — the authority as its own winner.
- **done:** the winner is the supplier the text names after the authority's contact, or none.
  Expected after unit 2: `[["buyer","Gobierno Vasco"],["winner","Montte"]]`.
