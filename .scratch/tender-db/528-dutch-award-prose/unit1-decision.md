# 528 unit 1: measure and decide (2026-10-13)

## Population

`name-phrase-census` job 2385 (`../527-niet-gegunde/name-phrase-census-2385.json`) found 166 organizations whose
served name holds the whole word `gegund`, and no other asked phrase. Their rows are in `rows-2385.json`.
- **Today:**
  - every one classes as a plain name (`not_a_name` → `None`, with and without `NLD`);
  - none is in the void net (`mentions_void` false);
  - no `award_winner` cue reads one.

  So none of the existing walks (510's `refold-void-names`, 511's `refold-placeholder-names`, 520's extraction)
  reaches them. Each stands as a junk winner organization.
- **By country:** NL 48, BE 4, none 114.
- **Nearly every one** is served as the winner of one tender.

## Classing

Classifiers and challengers ran as wf_d8bd6f3b-45c, in four batches of about 42 rows, each batch challenged in the
costly direction. The result is `unit1-classified.json`, with each row's class, the winner for A and N, the
confidence and the reasons.

**Owner rulings.** The challenger moved 3543691/92/94/95 (`Perceel L/M/P/Q is gegund aan: X`) to B and
5275836/37 (DVG and Klomp with their lot notes) to B. Its reason: a lot-scoped string with 7 or 3 mentions might
sit in every lot's result, so a rename would add the winner to lots it did not win. Bounded seeks
(`tender_version_result_winners` → `lot_results`) refute that:
- Tender 4816260's notice 17363505 (127063-2013) has 49 RES sections and no lot mapping. Each one carries exactly
  one of the seven strings, cycling J…Q.
- Tender 5137318's notice 17909932 alternates DVG and Klomp over RES-1..6.
- No result holds two strings, so a rename changes only the name on slots the string already holds.

The challenger's own revert condition therefore holds, and the six go back to A and N. Its other changes stand:
- 11425573 `… in een andere TenderNed omgeving gegund` → P: the lot is awarded elsewhere, so it is not void.
- 8105109 Falck (`perceel 2 nog niet definitief gegund: verificatiefase`) → X: a provisional award stays as
  published.
- 27113662 (`combinatie van de taxibedrijven van v.o.f. Van de Krieke`, cut at a comma by the text parser) → B.
- 31683724 (`Combinatie Randstad Polytechniek en Advin B.V.`) → B.
- 30837740 (`… de volgende twee dienstverleners: A`, a bare enumeration label) → P.
- 26825531 keeps its capital `De` in the winner, as published.

| class | n | what it is | treatment |
| --- | --- | --- | --- |
| P | 58 | a pointer, a count or a plural non-name (`Zie bij nadere inlichtingen, 3 aannemers zijn gegund`, `aan 316 aanbieders gegund … website`, `Aan alle inschrijvers is de opdracht gegund`, `Open House – 7 inschrijvers gegund`, `Vertrouwelijk …`) | **Placeholder** (511): the party is dropped and the result keeps its decision |
| V-later | 3 | not yet awarded (`Dit perceel wordt later gegund.`, `Perceel 2 wordt later gegund`, `Perceel 1 zal later gegund worden`: 5752780, 8273820, 10703666) | **Placeholder**: drop the junk party and keep the published decision. "Later" is no verdict, so the result is not marked `clos-nw` |
| V-void | 5 | nothing awarded (`Fictief gegund — geen inschrijvers` 9050747, `Voor dit perceel (5) wordt er niets gegund.` 11425570, `Geen inschrijving(en) ontvangen … geen opdracht gegund` 12924415, `niet (combinatieperceel C is gegund)` 29194063, `de opdracht zal niet worden gegund` 29208719) | **VoidLot** (510): the party is dropped and the result reads `clos-nw` |
| A | 56 | one awardee named in prose (`Perceel L is gegund aan: Koeman en Bijkerk B.V.`, `De opdracht is gegund aan: De Bouwcombinatie Korteweg-Boot v.o.f. i.o.`) | **rename to the reviewed winner**, as published |
| N | 8 | a company name carrying an award note (`De Beijer Bladel BV (6 percelen gegund)`, `XL Insurance Company SE — gegund layer 1`, `DVG Personenvervoer (perceel 1 en 2 gegund; …)`) | **rename to the reviewed clean name** |
| B | 34 | several awardees, a consortium list, or a per-lot list (`Perceel N is gegund aan: Grontmij, Waardenburg, …`, `Gegund aan: ADC ArcheoProjecten; …`) | **kept as published** (511 §5, 520's B) |
| X | 2 | a provisional award (Falck 8105109) and a mixed lot note (31680216) | **kept as published** |

## Why A and N take a reviewed table, not a rule

520 U6 found that no general shape survives review: two panels broke every rule in the costly direction. These
rows are closed history:
- the text era and r208/r209 2012–2016;
- eForms notices carry structured winners, so new award prose here is unlikely.

So A and N rename by an exact reviewed table (`(published name, winner)`, read whole on the fold, as
`A2_REVIEWED`). The A rule `… is gegund aan: X` is not built until a new census shows such names still arriving.

A and N names class `None` and sit outside the void net, so 520's `extracted_winner` (gated on `AwardSummary` or an
award cue, reading void-net copies only) never consults them. The fold therefore needs a reviewed-rename
consultation for any kept party whose copy is fold-equal to a table row. A walk does not reach them either, so the
drain is `refold-notices` over their notices, followed by `sweep-orphan-orgs`. The notices come from bounded
`organization_mentions` seeks by `organization_id`.

## Units

1. **U2 (S–M): P and V lexicon.** Add Dutch forms to 511's Placeholder lexicon (pointer heads `zie (bij) nadere
   inlichtingen`, `zie bijgevoegd/bijlage`, counts `aan <n> <partijen|aannemers|leveranciers|aanbieders|
   ondernemingen|dienstverleners|inschrijvers> gegund`, `alle inschrijvers`, `Open House`, `vertrouwelijk`). Treat
   V-later as Placeholder. Add V-void to 510's Dutch void forms.
   - **Gate on the 166:** P + V-later read Placeholder and V-void reads VoidLot, exactly as classed. A, N, B and X
     move nothing.
   - **Also verify:** the 156 kept 520 exemptions and the 402 void-walk names do not move.
   - **Drain:** the walks reach a newly classed name only if their nets catch it, so U2 checks
     `refold-placeholder-names`' net first. If the net misses these names, U2 drains with `refold-notices` over the
     66 orgs' notices instead.
2. **U3 (S): A and N reviewed table.**
   - Build `NL_AWARD_REVIEWED` (64 rows) and consult it for kept parties at the fold.
   - Drain: dry-read every rename, then `refold-notices` over the notices, then `sweep-orphan-orgs`.
   - Verify: no row of the table remains as an organization name, and each winner is served on the same results.
3. **Verify (both):** `name-phrase-census` `["gegund"]`. Only the B and X rows (36) remain, plus any row that was
   withdrawn, with its reason.
