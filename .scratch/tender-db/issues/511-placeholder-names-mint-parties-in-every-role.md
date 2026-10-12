# 511 — placeholder names (pointers, withheld, `Various`, award summaries) mint parties in every role

Status: ready-for-agent — UNITS 4+5 DRAINED 2026-10-12. Wet 2306 (expect 46,499; 46,487 re-queued) → fold 2307 retired **55,007** mentions. That clears the ~54,200 floor and stays under the 55,923 read trigger. Dropped parties: 216 void and 55,337 placeholder. 2,607 tenders corrected, 6,453 correction rows. The fold's sample of 25 identified placeholders was read. Echo identifiers aside, it is 'siehe Ziffer VI.4.1' in DE eForms review slot ORG-7005 carrying BWI GmbH's own ids (Leitweg 992-80159-61 = org 2216; VAT DE814789837). The publisher copied the buyer's id into a review slot that names nothing, so dropping it is correct: BWI no longer appears as its own review body. The auto sweep 2308 was over the cap (20,898), so it recorded a plan (10 protected by review tables). Wet sweep 2309 removed **20,898 orphan orgs** (28,158 name variants). NEXT: unit 6 precision follow-ups (below). Watch the daily fold's identified sample.
Was: ready-for-agent — UNITS 4+5 DRAINING 2026-10-12. Unit 5 was deployed in 4e79e46 (81f784d, gate green; build wf_b03f1aa2-7c5 with 3 reviews and 6 fixes).
- Combined dry 2291 (`../511-placeholders/u45/plan-dry.json`): placeholder 21,334 orgs / 55,708 mentions, blank 52 / 215, name 1,165 / 2,534; 114 buyer rows; 46,499 notices.
- Mandatory reads (wf_63890637-aa3, 46 agents, `u45/reads-wf_63890637.json`) covered all 2,136 non-exact folds and 1,115 list entries. Exactly one real party drops: 'Please see section VI.4.3 Translink' (1 mention, review body).
- OWNER RULING: proceed rather than stop. The same no-separator shape already dropped under unit 2's anchor ('See section VI.4.3 Translink'), and this is 1 mention against ~55.9k. The fix is a follow-up keep rule (see Unit 6).
- Buyer canary 2303/2304: 0 of 110 changed (`u45/canary/result.md`).
- Wet 2306 (expect 46,499) → fold 2307 running. Floor: about 54,200 retired (drop-class 55,923, less guard 3, the canary's 13, and a ~3% multi-name allowance); above 55,923 is a read trigger.
Was: ready-for-agent — UNIT 4 DEPLOYED 2026-10-12 (6593d9c, gate green; build wf_c7215857-cd9 with 3 reviews and fixes; deployed c786f0e). The unit 4 WET IS HELD. Dry 2283 (`../511-placeholders/u4/plan-dry.json`): the widened net (`echo_shaped`) catches 25,333 orgs. The fold would drop placeholder 1,253 / 1,920 and blank 52 / 215, but `name` is 24,027 orgs / 65,668 mentions and buyer_rows is 1,277, over the 1,000 stop line. Reading the `name` class exposed the real target. About 20.7k orgs / 56.7k mentions are bare SECTION-REFERENCE POINTERS in every language, which the lexicon does not classify:
- 'siehe VI.4.1' (2,550 orgs / 10,629 mentions);
- 'V.1 Award and contract value' (2,629 winner orgs);
- 'siehe I.1', 'see VI.4.2 (below)', 'wie (oben Ziffer) VI.3.1', 'jak w pkt VI.4.1', 'cf rubrique VI.4.1', 'vedasi punto I.1', 'As in I.1 above', 'Same as VI.4.1'.
Also, the legal-form check reads 'As' as the AS form ('As VI.4.1' is listed with a legal form). NEXT: unit 5, the section-reference pointer class plus a net narrowed to pointer shapes. Then ONE combined dry → reads → canary → wet for units 4 and 5. Daily folds already apply unit 4's rules to notices they touch.
Was: ready-for-agent — UNIT 3 DRAINED 2026-10-12.
- Buyer canary passed: 10 of 389 buyer notices changed Tender, all German award notices joining their contract notice once the placeholder buyer left; one refused split healed. See `../511-placeholders/u3/canary/result.md`.
- Wet job 2270 → fold 2271 retired 38,700 mentions (dropped parties: 211 void, 9,179 placeholder, 29,525 blank; 3,358 tenders corrected). Auto sweep 2272 removed 5,631 orphan orgs.
- Retired fell 205 short of the restated floor of 38,905. Residual dry 2273 (`../511-placeholders/u3/plan-2273-residual.json`) explains the gap: 851 drop-class mentions are KEPT by the fold, and the planned-but-unseen parties add 590. 604 of the 851 are TED's multilingual pointer: one party carries 'See Section VI.2/VI.3 Additional information' in 24 languages, and the all-names rule keeps it because the EL/LT/HU/… translations read as names. 144 of those 604 are also read as AwardSummary ('Additional information' taken as a name after the pointer).
- Identified-placeholder sample: no real supplier was dropped. 's.o.' [0251411691] keys only junk org 22713056; 'Keine Angabe' [DE198511192] keys no other org.
NEXT: unit 4 (precision), see the 'Unit 4' section below.
Was: ready-for-agent — KEEP RULES DEPLOYED 2026-10-12 (6faf241, with 513 U4). Re-dry job 2266 (`../511-placeholders/u3/plan-2266.json`): award_summary 240 orgs / 510 mentions (+38 orgs: the perfil genitive owners, e.g. 'perfil de contratante del gobierno de aragon', and the High Court 'but' clause), blank 3,566 / 29,386, placeholder 2,534 / 9,877, name 1,670 / 3,225; 42,488 mentions on 34,534 notices, 390 buyer rows on 389 notices. Dropped orgs 6,100 clear the restated floor of 5,928. The 10 perfil+genitive names still placeholder are intended junk ('ver perfil del contratante', '… del organo de contratacion', URL forms) plus one accepted cost ('ver perfil de contratante del hospital clinic de barcelona', stem not at position 0 of the owner). NEXT: buyer canary (refold-notices jobs 2267/2268 on the 389 buyer notices; pre-image `../511-placeholders/u3/canary/pre.json`), then wet with expect 34534, then the sweep.
Was: ready-for-agent — UNIT 3 READS DONE 2026-10-12 (wf_a3c63d17-e99: 700 non-exact folds and every list read; gap analysis). GAP EXPLAINED, so no regression. The floor counted the 522 scoped ingen/inget/nd orgs as Blank, while the plan's by_class shows them as name and the fold decides them per notice language. Like for like, the plan catches 185 more drop-class orgs than expected; the restated floor is 5,928, and the plan has 6,138. STOP for real-party losses, two patterns:
- `Perfil del/de contratante de(l) <body>`: about 43 orgs, mostly specifications-provider / further-information slots, no winners or buyers;
- `Not applicable, but proceedings may be started in the High Court…` (a review body).
KEEP RULES BEING ADDED (partyname.rs: the genitive owner after a perfil stem, and a blank 'but' clause); they gate and deploy together with 513 U4, then a re-dry, the buyer canary, and the wet run. No identified placeholder is a real supplier. Follow-up (precision, not loss): junk the anchor keeps as name, e.g. 'Please see section VI.4.2' (759 orgs), 'For details of the awarded suppliers please see section VI.2', 'Information withheld for security reasons', and '… erfolgt hier keine Angabe'.
Was: ready-for-agent — UNIT 2 DEPLOYED 2026-10-12 (10cb829, gate green; built by agent, correctness and test-strength reviews, 13 owner-ruled fixes). Unit 3 drain: dry job 2265 caught 7,808 orgs, of which the fold drops blank 3,572 / 29,394 and placeholder 2,566 / 9,974, and keeps name 1,670 / 3,225 and award_summary 202 / 405. That is 42,593 mentions on 34,635 notices, 390 buyer rows. The ORG FLOOR TRIPS: 6,138 dropped orgs against a stop line of ~6,450 (mentions 39,368 clear their ~37,000 floor). The mandatory reads (700 non-exact folds, the identified lists, legal-form and digit-only names) and a gap analysis are running (wf_a3c63d17-e99). The wet run waits on them and on the buyer canary (runbook step 2). Daily folds since the deploy drop parties as they touch notices.
Was: ready-for-agent — UNIT 1 DECIDED 2026-10-11 (`../511-placeholders/unit1-decision.md`; census wf_9b2cb650-aa4, 36 bounded indexed calls, 0 × 408; design plus code and data reviews, both 'sound with fixes'; all 22 owner rulings applied, and the 7 extensions confirmed). Classes: VoidLot, Placeholder (narrowed), Blank (new) and AwardSummary (new, never dropped). About 5.1k orgs / 36k mentions are admitted. NEXT: unit 2 build: U2a predicate, U2b fold + NON_NAME_FOLDS, then U2c plan readers and U2d `refold-placeholder-names` once 513 U2 is committed; one gate, one deploy. Then the unit 3 drain (§8).
Was: needs-triage — filed 2026-10-10 from issue 510's unit-1 decision (§9, `../510-void-names/unit1-decision.md`),
which split non-names into two classes and took only `VoidLot` ("the lot was not awarded"). This is the other class.
Kind: data quality (organizations; all eras)
Relates to: 510 (the `partyname::not_a_name` predicate and its `Placeholder` class), 484 (`NON_NAME_FOLDS` in
`project/role_census.rs`, a fold-level placeholder list for the buyer-equal test), 483 (buyer-slot fixes)

## What is wrong

A party slot that names no organization — a pointer (`See section VI.3`, `Voir autres informations`, `Véase perfil del
contratante`, `See Contracts Finder Notice for full supplier list`), a withheld name (`Not applicable`, `Withheld
Section94 Supplier`, `Keine Angabe`), a summary (`Various`, `Various suppliers`, `Multiple Suppliers`, `Diverse
Firmen`) or a relaunch summary (`Lot déclaré infructueux … puis attribué à la Société X`) — is minted as an
organization in every role.

510's census (2026-10-10): 2,952 orgs, 5,672 mentions. Unlike the void class, these are mostly NOT winners:
review-body 1,782, mediation-body 1,252, winner 1,214, appeal-information 403, Lot-ReviewOrg 293,
RESPONSIBLE_FOR_MEDIATION_PROCEDURES 194, purchasing-body 172, specifications-provider 160, further-information 46,
Tenderer 46, tender-receipt 35, **buyer 20**.

## Award summaries kept by 510 (2026-10-11)

510's drain keeps, as `Placeholder`, every void-lot name that also names an award
(`partyname::names_an_award`): ~500 orgs, listed in the `void-names-refold` report's `exempted` (job 2182). Each
is a summary standing in for real winners — `Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré
infructueux`, `lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07 - …`, `Infructueux … attribué à l'entreprise
Promocash`. They belong to this issue: the right outcome is per-lot winners parsed out of the summary (or the
summary dropped and the result left without a winner), not one organization named after the whole list.

## The semantics differ from 510

A winner was chosen; its name is elsewhere or withheld. So a result keeps `selec-w`, but nothing should be bound as
the winner. A buyer-slot placeholder changes buyer tokens and grouping, so dropping one needs its own decision.

## Units

1. **Decide** per role: drop the party (winner, review/mediation slots), keep it, or map it (a buyer placeholder).
   Unify `NON_NAME_FOLDS` with `partyname::Placeholder`. Carry 510's deferred list (`neant`, `ingen`, `n a`, `none`,
   `nie dotyczy`, `entfällt`, `keine Angabe`, `brak`, `sans objet`, `aucun`, `nessuno`, `varios`, `diverse`,
   `various suppliers`, `multiple …`, `withheld …`), each with its role split and its real-name traps (`InGen`,
   `NIL d.o.o.`, `N/A s.r.o.`, `Diverse Care Services`).
2. **Build** on 510's module and retire path.
3. **Drain** with 510's `refold-void-names` job generalised, then the 443 sweep.

## Unit 1 decision (2026-10-11): owner adoption

The decision is `../511-placeholders/unit1-decision.md`. It was revised the same day after two adversarial reviews
(code and data lenses; findings in `../511-placeholders/workflow-wf_9b2cb650.json`). All 22 owner rulings are in, and
the doc's "Review" section lists them.

The revision agent made seven extensions beyond the rulings' letter. The owner CONFIRMED all seven:
1. `would prejudice` stays unanchored.
2. The keep rule also strips a lot designator, accepts en and em dashes, and lets a bare `see section` keep without a
   separator (Royal Courts).
3. Guard 3's echo test compares space-free folds, and only when the name has a letter.
4. Twelve more "name missing" words move to Placeholder (66 winner/bidder rows in total, none with an identifier).
5. FI counts as in scope for `ingen`.
6. 57 specimen assertions flip, not about 45.
7. Anchoring also narrows the text parser's pointer refusals on a reparse. This is accepted.

Figures:
- admitted: 5,089 orgs / 36,236 mentions / 45,910 party rows (Blank 3,917 / 29,605; Placeholder 1,172 / 6,631);
- buyer rows: 286;
- dry floor: about 6,475 orgs, stop below 6,450.

Still open, as the decision lists:
- 1188404 `Ingen` FR (language unread);
- whether to promote the "nobody" words to VoidLot (needs 510's per-word value/date table);
- the deferred words `nil`, `ei`, `sin`, `vari`;
- a follow-up issue for per-lot parsing of the ~500 award summaries, to be filed with unit 2.

## Unit 4 — precision follow-up (filed 2026-10-12 from the unit 3 residual)

What the fold keeps that the decision meant to drop, measured on residual dry 2273 and the gap analysis
(wf_a3c63d17-e99):
1. **TED translation echo.** TED r208/r209 publish a pointer winner in every EU language. Proposed rule in
   `drop_decision`: when one of a party's names is `Placeholder` and carries a form section number (`VI.2`, `VI.3`,
   `VI.4.2`, `II.2.3`), its other names that carry the same number are translations of it. 460 + 144 mentions.
2. **A form section title after a pointer is not a name.** `pointer_names` reads 'Additional information' /
   'Complementary information' / 'Place of performance' as a name, so 'See Section VI.3 Additional information'
   becomes an AwardSummary.
3. **Politeness before the stem.** 'Please see section VI.4.2' (378 orgs / 411 mentions, review and appeal slots),
   'Please see section VI.4.3' (73 + 24), and 'please see Section VI.3.2' (38) read as names, because the stem does not
   open the name.
4. **Withheld phrasing outside the lexicon**:
   - 'Information withheld for security reasons' (88);
   - 'Due to the sensitive nature of this contract the supplier name will be withheld' (59 + 20);
   - '… erfolgt hier keine Angabe' (19);
   - 'This is not applicable in/to the UK' (15 + 14).
5. **Identified withheld suppliers stay**: 'Withheld Section94 Supplier' [GBPPONPBXY3232WNHZ] is a real PPON with its
   name withheld. Keep it, deliberately, with a test.

Each item needs a keep test for its near misses before it is admitted, and the same dry → reads → canary → wet as
unit 3.

### Unit 4 build review (2026-10-12): fixes and open points

Fixed in the build (uncommitted at the time of writing; `partyname.rs`, `project.rs`):
- **Item 1 seeds only from a bare pointer.** A placeholder whose text after its stem still names something
  (`partyname::names_after_stem`: references stripped, an opening separator stripped, section titles blanked,
  but no separator required) no longer seeds the echo. Before this fix, EN `See Section VI.3 Kier Construction`
  (a `Placeholder`, because the keep rule wants a separator) swept its DE copy `Siehe Abschnitt VI.3 Kier
  Construction` and dropped a real party. Before unit 4, that DE name kept the party. The residual's `Please
  see section VI.4.3 Translink` (1 mention, review body) was the live case. The 460 + 144 still seed. A
  capitalised common noun in the tail also withholds the seed (`Please see section VI.3 for list of awarded
  Suppliers`). This errs toward keeping a name.
- **The legal-form guard** also reads `plc`, `LLP`, `Inc`, `LLC`, `KG`, `AS`, `A/S`, `OÜ`, `d.o.o.` and `a.s.`
  (`LEGAL_FORM_TOKENS`).
- **The walk's net reads every copy the echo sweeps.** A party's head name is its first-seen copy, which is
  the ORIGINAL language's (`r209::parse` walks the primary original first). So a FR- or DE-original pointer
  winner heads as `Voir point VI.2) Autres informations.`, and before this fix `mentions_placeholder` missed
  it. The echo is the first drop rule where the party drops while its head name stays unclassified.
  `mentions_placeholder` now adds `echo_shaped`, a form section number in a translated pointer's shape. The
  next dry's `by_class.name` grows by such names (`Referat II.3`-style buyers). They cost a refold each, and
  the fold decides. How many non-English-head echo parties exist is unmeasured; the next dry counts them.
- **Keep tests added:**
  - an `AwardSummary` copy with no legal form (`siehe oben: Vergabekammer Südbayern (VI.3)`). ORG-8's
    `mbH` used to hide the class guard;
  - `Translink` and `Kier Construction` with their DE copies;
  - `Acme plc`;
  - a FR-first party that still drops.

Open, before the wet run:
- **Item 1 canary (mandatory).** In `TED_VI2_POINTER`, the PL, SK, SL, GA, BG, RO and HR strings are
  reconstructions, and so is every VI.3 translation. The read showed PL only as `Zob. sekcja VI. …`. A
  party drops only when EVERY copy is classed. If one real copy writes the number another way (`VI. 2`, a
  homoglyph numeral), the party is kept and the test stays green. So the unit-4 canary refolds 12516947
  and 19530163 and asserts that ORG-2 and ORG-10 are gone. The check is a bounded PK read, `SELECT
  organization_id, name FROM organization_mentions WHERE notice_id = ? AND section_id = ?`, which must
  return no row after the fold. **Stop** if either survives. The fix is to read that party's 24
  `TED-OFFICIALNAME` values (bounded, by notice) and replace the reconstructed strings with them.
- **Item 5 contradicts the decision, and it needs a ruling.** Item 5 says to keep `Withheld Section94
  Supplier` [GBPPONPBXY3232WNHZ] deliberately, with a test. Three things already pin the opposite:
  - decision §4 guard 3: "`Placeholder` gets no identifier guard";
  - `unit1-decision.md` line 188: one PPON on all 123 mentions, covering different withheld suppliers;
  - the test `a_placeholder_party_with_an_identifier_is_dropped` (project.rs), which decides this exact name
    with a gated identifier as `Placeholder`, dropped, and counted in `placeholder_identified`.

  No keep test was added. A "keep" ruling would need an identifier guard on `Placeholder`, a flipped pin,
  and an amendment to §4. A "drop" ruling amends item 5 and cites that test as the pin. The recommendation
  is drop, because the PPON names the placeholder, not a supplier. The build agent explained the 9
  surviving mentions of org 30914328 this way: an FTS `parties[]` entry with no name mints an empty-named
  mention that binds by the PPON, and the drop rule never sees a name. That is unconfirmed. Confirm it with
  a bounded indexed read of those 9 rows (`organization_mentions_org`: `SELECT notice_id, section_id, name,
  raw_identifier FROM organization_mentions WHERE organization_id = 30914328`).

## Unit 6 — precision follow-ups from the units 4+5 reads (filed 2026-10-12)

From `../511-placeholders/u45/reads-wf_63890637.json` (the readers' notes):
1. **Pointer + body with no separator** drops the body. Examples: 'Please see section VI.4.3 Translink', and 'See
   section VI.4.3 Translink' since unit 2. Proposed keep: when `names_after_stem` finds a name after the reference,
   once titles and positions are stripped, read the name as an AwardSummary. Keep tests: Translink, plus 'S. VI.4.1)
   Weimar Deutschland', which is kept today.
2. **Junk still kept as name**:
   - 'As shown in section I.1 above', 'As per the body in I.1', 'As defined in (section) VI.3 of this notice' and their
     kin (the `As` pointer family whose grammar words are not yet listed);
   - 'See links detailed in II.1.5';
   - 'Se référer à l'article VI.3 du présent appel à candidatures';
   - 'See VI.4.1) Review body field of this Contract Award Notice.';
   - 'Siehe die unter Ziffer VI.4.1) genannte Stelle.';
   - the long UK sentences ('This is an award notice and this opportunity is now closed…', 'The name(s) of the winning
     tenderer(s) are being withheld…', 'A full list of successful suppliers will be made available on Contracts
     Finder…').
3. **Mixed folds**: 'siehe vi 4 1', 'wie vi 4 1', 'voir section vi 4 2' and 'siehe i 1 oder vi 4 1' each have a
   spelling classed name. Find the variant (spacing or separator) and close it.
4. **The dry plan's `legal_form` list** (role_census / supervisor) still lists the `As …` pointers. Stop counting a
   pointer as a legal form there.
5. **'V.1) AWARD AND CONTRACT VALUE:'** (2,629 orgs, one mention each) and 'siehe VI.4.1' (2,550 orgs) never collapse
   to one org. Each slot keys on something else, perhaps an address or identifier. Check what mints them separately.

6. **TED template wordings and multilingual residue** (residual dry 2310, `../511-placeholders/u45/plan-residual.json`:
   434 placeholder orgs / 734 mentions and 49 blank / 185 still kept).
   - Notice 19530163 ORG-10 (org 10601576, 140 mentions) survives the wet. TED's earlier template words its copies
     differently from 12516947's:
     - PT 'Consultar o ponto';
     - SL 'Glejte oddelek';
     - HU 'Lásd a VI.3. szakaszt';
     - MT 'Ara t-Taqsima';
     - ET 'Vt punkti';
     - DA 'Se del';
     - GA 'Féach Alt';
     - LV 'Skatīt VI.3. iedaļu'.
     An unlisted opener keeps the party.
   - Proposed: a TED-template signal. When a party has ten or more language-labelled copies carrying the bare
     English seed's section token, and none of them has a legal form, sweep them whatever their opener; plus the
     openers and titles above.
   - The 'same as (in point) VI.x', 'please refer to VI.x', 'as in (point) VI.x' and 'voir point VI.x' parties are
     kept by a sibling copy. Read which copy.
   - The unit-4 canary (refold 12516947 and 19530163, assert no ORG-2 / ORG-10 mention) was not run before the units
     4+5 wet. It is mandatory before the unit 6 wet.
7. **Junk head names on real orgs.** 22713056 is headed 's.o.' but its 28 remaining mentions are 'Vergabekammer bei
   der Bezirksregierung', bound by the chamber's phone number 0251411691. 30914328 is headed 'Withheld Section94
   Supplier' (see 522). A head-name refresh after drops would fix the served name.
