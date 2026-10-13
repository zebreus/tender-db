# 528 — Dutch award prose in a winner slot (`… is gegund aan …`, `Zie bij nadere inlichtingen, 3 aannemers zijn gegund`) stands as a winner organization

Status: ready-for-agent — UNIT 1 DECIDED 2026-10-13 (`../528-dutch-award-prose/unit1-decision.md`; classifiers + challengers wf_d8bd6f3b-45c, `unit1-classified.json`). Today all 166 class `None` and none is in the void net, so no walk reaches them. Classes: P 58, A 56, B 34, N 8, V 8 (5 void + 3 later), X 2. The owner reverted 6 challenger moves after bounded seeks showed each result holds exactly one string. Treatment: P + V-later → Placeholder; V-void → VoidLot; A + N → an exact reviewed rename table (`NL_AWARD_REVIEWED`, 64 rows), as 520 U6 did, because no general shape survives review; B + X are kept. NEXT: U2, the P/V Dutch lexicon (simulate exactly on the 166, 0 moves elsewhere; check the placeholder walk's net, else drain via `refold-notices`). Then U3, the reviewed table, its fold consultation and a `refold-notices` drain.
Was: ready-for-agent — filed 2026-10-13 from issue 527's census (job 2385, `../527-niet-gegunde/name-phrase-census-2385.json`, the 166 rows whose phrase set is `gegund` alone).
Kind: data quality (results; organizations)
Relates to:
- 510 (VoidLot);
- 511 (Placeholder / pointer);
- 520 (award summaries: `award_winner`, `A2_REVIEWED`);
- 521 (nobody words).

## What is wrong

A bare whole-word `gegund` reaches 166 organizations:
- by country: NL 48, BE 4, none 114;
- every one is reached by its served name;
- almost every one is served as a winner of one tender.

They are the Dutch counterpart of the French prose that 510, 511 and 520 drained, and their lexicons miss it:

- **pointers** (the 511 Placeholder class):
  - `Zie bij nadere inlichtingen, 3 aannemers zijn gegund` (7813581, 8314178/79/86);
  - `De opdracht is gegund aan de organisaties als vermeld in VI.2 Nadere inlichtingen` (5819913);
  - `Zie bijgevoegd overzicht voor de namen …` (3354990);
  - `Zie voor de n.a.w.-gegevens … de bijgevoegde bijlage` (10722366);
  - `Gegund aan 2 partijen, zie verder bij VI.3) Nadere Inlichtingen` (10774733);
  - `De opdracht is aan 316 aanbieders gegund. U kunt de lijst vinden op de website …` (12408872);
- **void or future** (the 510 VoidLot class):
  - `Dit perceel wordt later gegund.` (5752780);
  - `Perceel 1 zal later gegund worden` (10703666);
  - `Voor dit perceel (5) wordt er niets gegund.` (11425570);
  - `Geen inschrijving(en) ontvangen voor perceel 2, 3 en 4 en geen opdracht gegund …` (12924415);
  - `Fictief gegund — geen inschrijvers` (9050747);
  - `Dit perceel wordt in een andere TenderNed omgeving gegund.` (11425573);
- **one winner in prose** (520's A1, in Dutch):
  - `Perceel L is gegund aan: Koeman en Bijkerk B.V.` (3543691);
  - `Perceel M is gegund aan: Grontmij` (3543692);
  - `XL Insurance Company SE — gegund layer 1` (6888341);
- **lists and co-winners** (520's B, kept per 511 §5):
  - `Perceel N is gegund aan: Grontmij, Waardenburg, …` (3543693);
  - `Gegund aan: ADC ArcheoProjecten; Archeologisch Onderzoek Leiden; …` (6530135);
  - `DVG Personenvervoer (perceel 1 en 2 gegund; perceel 3 gegund aan Klomp Groepsvervoer)` (5275836);
- **a company with an award note** (the name stands; strip the note?): `De Beijer Bladel BV (6 percelen gegund)` (5890355).

## First unit

1. Class all 166 names (the report's `rows`) the way 520 unit 1 did: classifiers plus challengers.
2. For each class, decide the lexicon home: 511's pointer heads, 510's Dutch void stems, 520's `award_winner` cues for `is gegund aan:`, or kept.
3. Simulate on the 166 before building, then drain with the same refold machinery (`refold-void-names` / `refold-placeholder-names`).

Re-take the population with `name-phrase-census` (`"phrases":["gegund"]`), together with the other Dutch award verbs: `toegewezen`, `gunning`, `opdrachtnemer`.

## Verify

`{"kind":"name-phrase-census","phrases":["gegund"]}`: the rows whose phrase set is `gegund` alone.
- Open: 166 (job 2385).
- Done: only kept names (B lists and company names with a note) remain, each listed in the issue with its reason.
