# 520 U6 build notes: A2 winner before a void tail (2026-10-13)

Built against `unit1-decision.md` §4.2 "A2 (10 orgs)" and §5 U6. U5's read was clean, so the decision said to build
A2's reliable subset.

## What changed

- **Superseded by panel 2 (below): `void_tail_winner` is now an exact table of the five reviewed rows.** The
  shape description that follows is the version panel 2 broke, kept as the record.
- `crates/ingest/src/partyname.rs`: `pub fn void_tail_winner(name) -> Option<String>`. It never answers for a name that
  `may_name_a_winner` flags (A1's `award_winner` owns those), so the two never both answer. Two exact shapes, each with
  its own evidence that the head is a party:
  - **address** (`address_party`). The text is `<name> <digits> <street word> … <postcode> <city>`, then `/` or a
    spaced dash, then BOAMP's partial-void sentence in full (`partial_void_sentence`: `[la|le]
    consultation|procédure|marché a été déclaré(e) sans suite pour la part [du marché] | les parts non couverte(s)
    [des besoins]`). `one_address` requires one postcode, one street opening, at most three city words after the
    postcode, and no separator, bracket or `et`/`ou`.
  - **consortium** (`consortium_party`). The text is `GRPT …` or `Groupement …` with exactly one `(mandataire)` and no
    other bracket, then `, suite à` + `NEGOTIATED_WORDS` only + an `EARLIER_VOID` phrase closing on `infructueux`.
  - `void_tail_name` refuses a name that fails any of these:
    - at least two words, with a capital first;
    - no digit;
    - no separator or bracket (the consortium's comma and `(mandataire)` aside);
    - no lot designator;
    - no street, place or number-mark word;
    - no public-body first word;
    - no offer, rejection, undo or status stem;
    - no `et`/`ou` joining two bare names;
    - `plausible_winner`.
- `crates/ingest/src/project.rs`, `extracted_winner`:
  - the gate also passes a party with an `AwardSummary` copy;
  - a void-net copy's winner is `award_winner`, else `void_tail_winner`;
  - a winner from a void tail (`from_tail`) is refused when a copy outside the net lists anything (`;`, `/`, `+`, `|`,
    a dash, a line break).

## Reviews

- **Panel 1** (wf_8883fc55-316, 4 lenses, 97 probes). The first draft was generic: any `;` void-only segment, any
  partial-void tail, any `, suite à … infructueux` tail, and a comma allowed wherever a consortium word stood. Every
  lens found costly leaks:
  - lot titles, procedures, statuses and failed or rejected bidders before `; infructueux`;
  - B2 co-winners joined by `,` or ` et ` around BGE's address;
  - two consortia, or the buyer's `mandataire du maître d'ouvrage`;
  - foreign void stems (`niet gegund`).

  The rewrite above came from it. **The decision's third shape — a void-only `;` segment (`Bp innovis ; infructueux`,
  4836476) — is withdrawn.** Its head cannot be told from a lot title (`Plomberie sanitaire ; infructueux`) or a
  failed bidder (`Dupont SARL ; offre infructueuse`), and it is B1's list grammar without the lot labels, which 511 §5
  keeps. 4836476 stays a summary (1 mention).

  All 97 probes were then run mechanically against the code, together with the 402 population names. After the rewrite
  and a follow-up guard (two words; place, public-body and number-mark refusals for `Garage`, `SCI`, `Mairie`,
  `Immeuble Simarouba`, `BGE Guyane N°` and `… sise`), there are **0 costly mismatches**. 14 cheap refusals remain:
  probes where a correct winner is left unextracted, for example `Dupont SARL (3 avenue Foch …)`, `16-18 rue` and
  `GRPT Dupont SARL (mandataire) et Martin SAS` (`plausible_winner`'s `second_party`). The representative probes are in
  `void_tail_winner_cuts_the_reliable_a2_tails`.
- **Panel 2** (wf_421af829-0da, 3 lenses, 37 probes) broke the two-shape rule again in the costly direction.
  It found no panic, no A1 regression and no leak in the real population. The leaks:
  - an address proves a place, not one contractor: a second party after the city with no joiner (`… 97300
    Cayenne Point Com`), two names juxtaposed before the address, a lot title with its site (`Restauration
    scolaire 16 rue …`), a buyer behind an article (`La Mairie de Cayenne`), generic two-word heads (`Le
    Groupement`, `Entreprise Individuelle`);
  - a consortium head admits a buyer (`Groupement de commandes Ville de Cayenne (mandataire) et CACL`), a
    failed bidder or another lot's winner after a comma (`…, Durand SA non conforme`), and `sans offre` in
    the negotiated words;
  - `listed` at the fold missed `,`, `&` and `et` in a copy outside the net.

  **Decision: no general shape.** Each round of tightening opened another hole, and the benefit is five orgs.
  `void_tail_winner` is now the five reviewed rows (`A2_REVIEWED`), read whole on the fold, so `/` for ` - `
  reads the same. Every shape helper is gone. `listed` now refuses a tail-derived winner when a copy outside
  the net adds a separator the winner does not hold (`;`, `/`, `,`, `&`, `+`, `|`, a dash, a line break) or a
  conjunction past the winner's words (`et`, `and`, `und`, `y`, `e`, `en`, `ou`, `or`, `avec`, `puis`,
  `ainsi`). `BGE Guyane Cayenne` still binds. The fold test covers `/`, `,` and `et` (ORG-4, ORG-6, ORG-7).
  A new A2 summary stays as published until its row is reviewed and added.

## Population

The void walk's dry reports 2351, 2355, 2360, 2375 and 525's after-dry list together hold 402 distinct `[org, name]`
pairs. On them `void_tail_winner` answers exactly:

- 1844226, 1844229 and 1844232 → `BGE Guyane`. The existing FR org 1844216 (`BGE Guyane`, 7 mentions) absorbs them.
- 2734624 → `GRPT GCC (mandataire) et Andre BTP`.
- 2734625 → `GRPT vallée sas (mandataire), La Comec et Arbat System`.

No org of either GRPT name exists, so each mints one.

## Drain (2026-10-13)

- Gate green at 5a43e1b, then deployed at 5a43e1b (queue idle).
- Dry 2380 extracted 6 mentions of 5 orgs on 2 notices (r208), exactly the five rows (`void-names-refold-2380-dry.json`). None of them is identified, and A2's other 5 rows stay exempted.
- Wet 2381 (expect 2) → fold 2382: 6 mentions refreshed and re-bound, 7 correction rows, 2 tenders written.
- Sweep 2383 swept 5 of 5, each published as `organization removed`.
- Verify:
  - 1844226, 1844229, 1844232, 2734624 and 2734625 answer 404.
  - BGE Guyane 1844216 holds 11 mentions (from 7).
  - The GRPT consortia are minted as 31922245 and 31922246.
  - Dry 2384 converges: 0 void, 0 extracted, 151 kept exemptions.
