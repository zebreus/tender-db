# 484 — the buyer is served as its own contractor (the winner slot repeats the authority)

Status: ready-for-agent — NEXT: fix unit (unit 2, parse): text `awarded_names` reads a numbered value to its own
next item, refuses `Contrato n°` segments and drops the authority-contact entry (shared e-mail/URL, another entry
survives); FTS parties keyed on (id, name) with the shared identifier kept on the supplier only. Land with the
phone-as-lot-reference by-catch (file it). Dry re-parse diff over the text era + FTS (names gained/lost per notice),
then re-project. Unit 3 (projection): flag `is_buyer` on buyer-equal winners, exclude from supplier statistics.
Kind: data correctness (awards, parties)
Relates to: 483 (found by its census: the `contractor-*` classes), 456 (mention binding), the served `awards[]`

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

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/2959772 | jq -c '[.parties[] | [.role, .organization_name]]'

(Tender 2959772 is notice 2002406, TED 190812-2002.)
- **open** (2026-10-03): `[["buyer","Gobierno Vasco"],["winner","GOBIERNO VASCO"]]` — the authority as its own winner.
- **done:** the winner is the supplier the text names after the authority's contact, or none.
  Expected after unit 2: `[["buyer","Gobierno Vasco"],["winner","Montte"]]`.
