# 526 unit 1: the parser measured on real bodies before the reparse (2026-10-13)

`reparse` has no dry run (operations.md, issue 484), so the change was measured locally on stored bodies.

## Method

1. **Lot sample.**
   - 912 notices: a stride over the first-mention notice of every fifth organization whose `name_norm` starts
     with `lot ` (`id % 5 = 0`, 4,172 organizations, 2,735 distinct text-era notices).
   - Text-era bands only: ids 17k–500k and 870k–2.24M.
2. **Control sample.**
   - 596 text notices: every 2,000th id in 1.2M–2.24M and every 4,000th in 20k–500k.
   - 37 of the ids were not found, and 7 had no `TXT-TX`.
3. **Pull.** `pull.py` ran on the box. It fetched each `GET /v1/notices/{id}/content`, throttled to 0.2 s and
   backed off on 429. It kept the `TXT-TX` body and the stored winner names: `TED-OFFICIALNAME` under `RES-*`,
   which is the old parser's output.
4. **Parse.** A temporary `#[ignore]` test (not committed) ran `awarded_names_and_values` on each body. This is
   the same call the `TX` prose rule makes. Its output, `{id, old, new}` per notice, is in
   `dry-sample-lot.jsonl` and `dry-sample-ctrl.jsonl`. `analyze.py` summarizes it.

## Result (final build)

| | lot sample (912) | control (596) |
| --- | ---: | ---: |
| notices unchanged | 37 | 581 |
| winner names, old → new | 4,644 → 5,258 | 310 → 354 |
| names carrying a lot reference, old → new | 4,423 → 208 | 9 → 4 |
| names shared by ≥ 3 notices | old: `Lots 1)` 48, `lots 2)` 46, `lots 1)` 27 … (all references); new: Vygon 17, Becton Dickinson 16, Medtronic 16 … (all companies) | none either side |

- **No real winner is lost.** Every old name absent from the new output is itself a list fragment: `Pomona
  Episaveurs. Lot 9: 1: Prodirest`, `rabais 0 %. Lot 4) Wanadoo Interactive`, `Marché numéro 03001. Lot 1) Lyreco`.
- **Control diffs** are the same class, or the same names in a different order.
- **The review's costly cases**, each now refused and tested:
  - `Lot 2) Idem`, `en cours`, `sera attribué ultérieurement`, `même titulaire`;
  - a price after `Lot 1:`;
  - a cue after the comma stealing the head;
  - a cut window read past its comma;
  - the dropped `S.A.` dot;
  - a reference left in the name.
- **Found by the sample and fixed before commit.** The first build still minted the hub `lots 1)` (lists longer
  than 24 bytes), `lots 7` (`lots 7, 8, 9 and 10: Carabelli`), `et 11) Maquet` (about 6,000 organizations on prod
  start `et <n>)`), `Lot 3`, `1: Prodirest` and the lower-case lot descriptions `papier` and `imprimés` (1644391).
- **Residue kept as published (unchanged or better than before):**
  - lettered and Roman labels (`Lot A)`, `Lot III:`; about 250 organizations on prod);
  - heading prose before the company (`contrôle technique: Alpes Contrôle`, `n° de marchés 030180: Edwards
    Lifesciences`);
  - a price tail (`ditta … di Palermo per 346 337 600 LIT`);
  - a description in the provider slot with the company in another item (`local bus service`, 150979);
  - `no action`. These cases were junk before too.

## Prod baseline (bounded `name_norm` range counts, before the reparse)

| prefix | orgs |
| --- | ---: |
| `lot 1)` | 5,239 |
| `lot 2)` | 1,487 |
| `lot 3)` | 999 |
| `lots ` | 8,929 |
| `lote 1)` | 109 |
| `lot n` | 192 |
| `et 1` … `et 9` | 987, 1,596, 1,033, 803, 517, 351, 284, 232, 184 |
| `-1` | 147 |

Verify exhibit: `Lot 1) Blin` → 1 (31735007).

## Second review (wf_78c49010-3c4, 13 agents, on ca096af)

Six findings were confirmed, in four classes. Each is now fixed and tested
(`a_lot_list_names_its_winners_not_its_headings_or_subjects`):

| class | defect | fix |
| --- | --- | --- |
| A | The award reader was opened on any prefixed entry, so on bare `1)` / `1:` / `A:` entries a cue after the comma stole the head (`1) Dupont SA, Paris. Le lot 2 a été attribué à la société Martin` gave Martin, where the old parser gave Dupont SA). | Only the head's own cue opens the reader. |
| B | A heading before the lot list became a name (`Rabais. Lot 1) Exbrayat`). | A comma-less entry with no lot word, right before the value's first lot entry, names nobody. |
| C | Capitalised lot subjects became hub names (`Hérault`, `Var`, `Bière`, `Terrassement - VRD`). | `<subject>: <company>` takes the company: the last colon part outside parentheses, never after a contact or role label. A subject head takes the company-word field after it (`Entreprise Malet`). `Société Laser: lot 1) …` names the company once, and its lots are subjects. A heading label (`Architectes attributaires:`) opens the lots as winners. A void field after the head (`Lot Sans Suite`) names nobody. `sous-lot 1A)` is a reference. |
| D | In a 2004–2010 block, a keyed list that collapsed to one name took the block's total. | The keyed entries the bar refuses still count in the pairing. |

The first build of these rules regressed six cases on the sample, and each was fixed with a test:
- `Telis; Bull; lot 2)` lost Bull: a heading only opens a list.
- `Multi attributaire: Cébéa Activ` was read as an award cue.
- `Att: Richard Perry` was taken as the company.
- `zone géographique Nord` won over `Gozzi Sport 2000`: the last colon part wins.
- `(mandataire: Ancelin)` was split inside its parentheses.
- `Entreprises attributaires:` was taken as a company label: the plural is a heading.

**Final sample (out8):**
- The 596-notice control is unchanged from the first build.
- In the lot sample, names keep a lot reference in 208 cases. 5,483 names are read against the old parser's
  4,644, and every name in three or more notices is a company.
- Against the ca096af build, 93 notices change. Every name removed is a heading, a subject or a fragment.
- The residue still includes `Champagne` (2053729) and `Canton Ouest`: a subject followed by a company with no
  company sign.

## Third review (wf_6e92c784-dae, 12 agents, on 5337b57)

The review confirmed seven findings. All are fixed and tested, mostly by narrowing the second round's rules:

| finding | fix |
| --- | --- |
| A company-word heading label (`Entreprise retenue:`, `Société attributaire:`, `Groupement solidaire:`, `Ste-Geneviève-des-Bois:`) became a hub, and its lots were dropped. | A company label needs a legal form or `société`, `entreprise` or `groupement` before a name. It must carry no award word and not be a generic phrase. |
| The company-word field rule replaced a company with its town (`Ste-Foy`), its branch (`Ets Nord Picardie`), its kind (`société anonyme`) or a relation (`entreprise du groupe Vinci`). | The rule takes only `société`, `entreprise` or `groupement` followed by a name. |
| The field-colon scan took a contact, a subcontractor, a member or the head office (`Contact person: John Smith`, `sous-traitant: X`, `Siège social: Lyon`). | Any role or contact word before the colon refuses it. |
| A company's own lot list (`Siemens SA: lots 1) et 3); lot 2) Philips SA, Suresnes`) swallowed Philips SA. | An addressed or signed keyed entry ends the list. |
| The heading rule dropped a winner (`Spie Trindel. Lot 2: infructueux`). | A heading only opens a list at lot 1, and never carries a company sign. |
| **Real (2111622, 2120723):** an empty 2004–2005 provider slot (`Lot 4) V.1) Award and contract value V.1.1) Name and address of successful supplier, …provider: Smiths Médical`) duplicated the next award's winner. | `keyed_name` never reads past a form item (`n)`), and such an entry is not counted. |

**Re-measured (out9):**
- The control is unchanged.
- Against out8, 4 lot-sample notices move:
  - 2111622 goes from 12 to 10 names and 2120723 from 3 to 2 (the duplicates are gone);
  - 1293291 drops one junk fragment;
  - 1610125 regains `I. Siemens SAS`, the old parser's reading.
