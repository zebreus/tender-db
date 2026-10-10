# 510 — void-lot phrases (`Infructueux`, `Sans suite`, `Desierto`) mint winner organizations in every era, not only the text era

Status: ready-for-agent — UNIT 2 BUILT, DEPLOYED (76c61c3, 2026-10-10); UNIT 3 DRAIN STOPPED AT THE DRY RUN by its own stop rule. The build passed two adversarial review rounds (wf_09dccb0e-a0c: 5 confirmed, all fixed — retire announces ADR-0017 D5 corrections, the dry plan lists legal-form orgs and the profile split, the predicate allocates nothing per stem, every fold branch pinned and mutation-checked; 1 pre-existing filed as 512 — and wf_e206c799-029: 1 test gap, fixed). The dry `refold-void-names` (job 2164) found 16,264 orgs / 31,219 mentions / 12,759 notices (r208 11,219, text 1,161, internal-ojs 175, r209 155, eForms 48, FTS 1) — above the census band — and 58 legal-form orgs where 3 were expected: most are COMPOSITE award summaries (`Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré Infructueux …`, `GRPT vallée sas … suite à procédure négociée après AO infructueux`) that name real winners beside a void lot. Dropping them would turn real awards into `clos-nw`. NEXT: a full-name walk (every matched name, `/v1/sql` PK windows) to size the composite class, then an exemption in `partyname` (a void stem inside a multi-lot or award summary is a Placeholder), re-gate, deploy, re-run the dry plan, wet only when every legal-form / identified org reads void.
Was: ready-for-agent — UNIT 1 DECIDED 2026-10-10 (workflow wf_c01cf7a5-c88: a fold mint-path map with file:line cites, a prod false-positive read over ~280 name prefixes and 81 windows of 285,757 stored names, and a synthesis; full decision `../510-void-names/unit1-decision.md`). A new pure predicate `ingest::partyname::not_a_name` → `VoidLot` | `Placeholder`; the fold applies `VoidLot` only, in every era and role, through one alias-aware `void_party_sections` that both `NoticeState::mentions` and `NoticeState::read` call: a void party mints no org, carries no role/winner/bid party, and a legacy result left with no real winner reads `clos-nw` (today the void name MANUFACTURES `selec-w`: all 3,570 result-winner rows of 1169469). A refold retires recorded void mentions inside the resolver's transaction (`Db::retire_mentions`), feeding the 443 sweep. The text parser keeps its early drop but takes its lists from the module. Scale: 11,166 void orgs, 24,565 mentions (r208 94 %), ~9.5k notices; the census undercounts ~15–25 %, so the drain enumerates by a full org scan with the same predicate (a new `refold-void-names` job). False positives: whole-value-only new entries; the real names next to the phrases (Desierto OÜ, DESERTOT, VÁRIOS MUNDOS, NIL d.o.o., N/A s.r.o., Void arhitektura, …) stay names; 1 accepted residual in 285,757. NEXT: unit 2 build (predicate module, fold application, store retire, cohort job, tests per §6), then unit 3 drain (§7).
Was: ready-for-agent — filed 2026-10-10 from issue 509's Verify, which could not pass. The text-era parser's rejects
(issue 508) cleaned the 2004–2010 text notices, but the same phrases arrive as winner names from the XML eras
(r208 2011–2021, r209), `internal-ojs` and eForms, where no name check exists. The largest hubs are XML-era. Unit 1 is
the decision below: one fold-level name rule for every era, applied at mint time and through one refold, instead of a
reject list per parser.
Kind: data quality (organizations; all eras)
Relates to: 509 (the text-era drain that exposed this), 508 (`NAME_REJECTS` / `NAME_WHOLE_REJECTS` in
`crates/ingest/src/text/parse.rs`), 484 (`NON_NAME_FOLDS` in `project/role_census.rs`, the fold-level precedent for
"a placeholder is not a name"), 434 (a fold re-resolves recorded mentions), 443 (the orphan sweep that removes what a
refold leaves)

## What is wrong

A winner slot that says the lot was void (`Infructueux`, `Lot infructueux`, `Sans suite`, `Non attribué`,
`Desierto`, `Declarado desierto`, `Nessuna aggiudicazione`) or points elsewhere (`Véase perfil del contratante`,
`See Section VI.2`) is minted as an identifier-less organization and served as the winner. Read 2026-10-10 after
509's drain (mention counts per profile, bounded reads through `/v1/sql`):

| org | name | mentions |
|---|---|---|
| 1169469 | infructueux (FR) | r208 3,219, internal-ojs 126, r209 22 |
| 1170715 | Sans suite | r208 2,122, internal-ojs 26, r209 5, eforms 1 |
| 1177920 | Lot infructueux | r208 1,025, internal-ojs 47, r209 4 |
| 1185001 | Desierto | r208 1,004, text 99, internal-ojs 6 |
| 1247907 | Desierto | r208 99, eforms 12, r209 3 |

plus hundreds of singleton orgs of the same spellings (`name_prefix=Infructueux` returns a full page of 50).

Three gaps:
1. **No check outside the text parser.** `plausible_name` and its two lists live in `text/parse.rs` only. XML and eForms
   names reach the fold's mention resolver unchecked.
2. **Variants the text rejects miss.** `Desierto.` with a trailing period (2005, fetches 253 and 255, inside 509's drained
   range) and `Desierto (lotes VIII y IX)` / `desierto (lote 3)` survive: `NAME_WHOLE_REJECTS` compares the raw value
   whole.
3. **Pre-2004 text** (fetches 270+) was never drained (509 measured ~10–20 junk names per 50k notices there).

## Proposed fix

**Unit 1: decide where the rule lives.** The candidate is the fold: a mention whose every published name is a void-lot
or pointer phrase mints no organization and records no winner role. That is one rule for every era, it applies through
`project` (a refold, no reparse), and the 443 sweep removes the orgs it orphans. The rule is the text lists generalised:
compare a folded form (case and accents folded, trailing punctuation and a trailing parenthetical lot qualifier
`(lote 3)` / `(lots 2 et 4)` stripped) whole against the whole-value list, and the raw form against the substring list.
Decide in this unit:
- whether the text parser keeps its own lists (it drops the slot before a mention exists) or defers to the fold;
- what happens to the RESULT: the lot result keeps its decision (`not awarded` is often stated separately) but names no
  winner;
- the false-positive guard: the 300k-name scan from 508/509 found none for the text era; repeat it over an XML-era
  window before building.

**Unit 2: build and drain.** Refold the affected Tenders: `refold-notices` over the notices carrying a mention of the
listed orgs, or a profile-scoped refold. Then `sweep-orphan-orgs` (dry, read, wet).

## Verify

    ssh root@zebreus.click 'curl -s "localhost:8080/v1/organizations?name_prefix=Infructueux&limit=50" | jq ".items | length"'

- **open** (2026-10-10): `50`.
- **done:** `0` after the refold and the sweep.
