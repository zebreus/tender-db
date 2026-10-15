# 529 — FTS `Individual` suppliers bind by a shared name or by a buyer's PPON

Status: ready-for-agent — filed 2026-10-14 from issue 522 unit 1's reads. Unit 1 measured 2026-10-15 (job 2493).
Units 2 and 3 DECIDED 2026-10-15 by a design panel: three designs and a judge, read-only, code-cited (below).
Unit 2 built (gate, then main, deploy and drain). Unit 3 is accepted, with no code. Next: deploy unit 2, run the
drain, then the Verify.
Kind: data quality (organization identity)
Relates to:
- 522 (the s.94 placeholder PPONs stop keying, so their `Individual`s bind by name);
- 511 (placeholder and blank names; guard 3 keeps a `Blank` party with an admitted identifier);
- 452 (identifier verdicts cannot move an exact-triple bind).

## Verify

    echo "SELECT id, country, identifier, provisional FROM organizations WHERE name_norm = 'individual' LIMIT 50" | ssh root@zebreus.click /root/sq.sh

- **done**: only identified rows (18934187 GH, 31570158 GB), and 31966280 is gone. UKRI's two PDQJ `Individual`
  mentions stand as accepted (unit 3): on 46806664 with no role, and on 46819715 as Tenderer and winner on lots 2 and
  4 of tender 8596401.
- **open (2026-10-15)**: 31966280, a GB provisional `Individual`, holds the four s.94 mentions: PBXY and PGYY on
  46806664 and 46819715.

## What was found

Find a Tender publishes a sole-trader supplier under the name `Individual`. Notices 46806664 and 46819715 (one
framework, about 40 suppliers each) carry four `Individual` slots, each under a different PPON:

| PPON | org it binds today | what the PPON is |
| --- | --- | --- |
| `PBXY-3232-WNHZ` | 30914328 "Withheld Section94 Supplier" | s.94 placeholder (522 condemns it) |
| `PGYY-2165-QXNQ` | 31548264 "Withheld Section94 Supplier 2" | s.94 placeholder, 2nd profile (522 condemns it) |
| `PDQJ-7126-JDHG` | **30913706 UK Research & Innovation (3,548 mentions)** | UKRI's own PPON |
| `PMGL-5552-GJZX` | 31570158 "Individual" | unknown: the supplier's own, or another stand-in |

Two problems follow:
1. **A shared name.** Once 522 lands, the s.94 `Individual`s carry no identifier and bind by `(name_norm, country)`.
   - Every withheld sole trader in the UK then lands on ONE provisional `Individual` row.
   - On these two notices that already joins two different suppliers, and their winner rows on one result collapse.
   - Today no identifier-less `individual` org exists anywhere: the Verify read returns only identified rows. So the
     class is new and small, but it grows with every s.94 individual FTS publishes.
2. **A buyer's PPON on a supplier.** UKRI's own PPON is published on an `Individual` supplier, so a withheld
   individual binds UKRI's organization.
   - A 452 `wrong` verdict cannot move it, because the resolver binds the exact triple before verdicts (see 522 unit 1).
   - It is the same shape as 466/452's wrong-identifier class, with a placeholder name as the tell.

## Proposed units

1. **Measure.** Run `name-phrase-census` with `{"phrases":["individual"],"mentions":true}`, an in-app corpus walk,
   when the queue is idle. Count, from its output:
   - whole-name `Individual` mentions;
   - how many carry an identifier;
   - how many of those bind an org whose names never include `Individual` (the UKRI shape);
   - their roles.
   Check first whether the census reports whole-name matches apart from substrings (`Individual Solutions Ltd`).
2. **Decide the name rule from the measurement.** The candidate is `individual` in `BLANK_WHOLE`.
   - Guard 3 then keeps an `Individual` with an admitted identifier (31570158's) and drops a bare one, which covers
     the s.94 ones after 522.
   - `Placeholder` would be wrong: it has no identifier guard, so it would drop identified sole traders too.
   - Cost: a withheld individual winner stops being a party. The lot keeps its other winners.
3. **Decide the buyer-PPON case separately.** A guard needs to know that a PPON is a buyer's, for example an
   `Individual`-named mention whose identifier's org serves only buyer roles. Size it from unit 1 before building
   anything.

## Unit 1 — measured (2026-10-15, job 2493)

`name-phrase-census {"phrases":["individual"],"mentions":true}` walked 5,676,483 organization names and 49,671,073
mention names in 54 s, and reached 290 orgs.
- **The census matches substrings.** 286 of the 290 are legitimate names that contain the word. Examples: Romania's
  `Cabinet Medical (Veterinar) Individual …` sole-practice names (RO, 119 orgs), `Individual Centred Care` (GB, 43
  winner mentions), `Individual Computer Applications AB` (SE). Any rule here must match the WHOLE name only.
- **Whole-name `Individual`: 4 orgs.**

  | org | identifier | mentions | roles | shape |
  | --- | --- | --- | --- | --- |
  | 18934187 GH | `P0004469968` | 2 | winner 1 | identified, named `Individual` itself |
  | 31570158 GB | `GBPPONPMGL5552GJZX` | 1 | none | identified, named `Individual` itself |
  | 31966280 GB | none (provisional) | 4 | Tenderer 1 (one tender) | 522's residual: the s.94 slots bound by name |
  | 30913706 GB, UK Research & Innovation | `GBPPONPDQJ7126JDHG` | 3,548 (some named `Individual`) | Procedure-Buyer 2,696, Tenderer 4 | the buyer-PPON case: an `Individual` mention carries UKRI's PPON |

- **Sizes for the decisions.** The bare-name class (unit 2) is one org with 4 mentions today. It grows with every s.94
  sole trader FTS publishes. The buyer-PPON class (unit 3) is one PPON, and it serves UKRI as a bidder on 4 tenders.
- `BLANK_WHOLE` with guard 3 would drop 31966280's 4 mentions and keep the two identified orgs. It would NOT fix
  UKRI: guard 3 keeps an `Individual` with an admitted identifier, and the PDQJ PPON is admitted.

## Units 2 and 3 — decided (2026-10-15)

A design panel ran with three angles: minimal drop; keep the party but stop the merge; the buyer-PPON case. A judge
re-checked their code claims.

**Unit 2: `individual` joins `BLANK_WHOLE`** (`crates/ingest/src/partyname.rs`). It changes one list and touches no
resolver, store, idgate or parser code.
- A whole-name `Individual`, or `Individual (Lot n)`, is classed `Blank`.
  - With no identifier, or one the gate refuses (the s.94 PPONs: `idgate::withheld_placeholder`), guard 3 has nothing
    to keep. The party drops, no mention is minted, and the fold retires the recorded rows.
  - With an admitted identifier, guard 3 keeps it: 31570158's PMGL, 18934187's GH number, UKRI's PDQJ.
- Names that merely contain the word stay names.
- **Cost, checked.** FTS goes through the eForms results path. BT-142 comes from the award status, so the decision
  stays `selec-w` and the awarded value stays. Only the winner row and the bid party go.
  - On tender 8596401 every lot that held 31966280 keeps 17–31 winners.
  - Worst case on a future notice: a lot whose only supplier is a bare `Individual` serves `selec-w`, its value,
    `winners: []` and a bid with no parties.
  - This is 511's ruling for withheld suppliers: the result keeps `selec-w` and nothing is bound as the winner. The
    same PBXY PPON's s.94-sentence names already drop as `Placeholder`.
- **Rejected:**
  - keep-but-isolate (mint one org per withheld supplier): it breaks 434's no-re-decide contract, has to patch at
    least seven `(name_norm, country)` reuse sites, and fills `?winner=` with identical `Individual` orgs;
  - `Placeholder`: it has no identifier guard;
  - a scoped ENG entry: the census found no non-English whole-name `Individual`.
- **Open risk, population 0 today.** On the legacy and sdk-0.1 paths, a lone, dateless, valueless bare `Individual`
  winner would read `clos-nw`, because `Blank` carries no withheld marker. Job 2493 found whole-name `Individual`
  only on FTS, apart from the identified orgs. Revisit if the Verify read ever shows a non-FTS bare one.

**Unit 3: accepted, no code.** The panel read the public API.
- UKRI is not the buyer on FASTER. The Met Office is (`GB-PPON-PVZW-3234-DTLY`).
- The PDQJ `Individual` is a supplier slot the publisher re-pointed. On 46806664, TPA-2-11 and TPA-4-16 name the s.94
  PBXY `Individual`. On 46819715 the same TPA ids name `ORG-GB-PPON-PDQJ-7126-JDHG`. The framework is for "Industrial
  and Academic Institutions", and the Met Office contracts UKRI directly elsewhere (46801716).
- Nothing published shows the PPON is wrong, and the exact-triple bind treats the literal as the org's own evidence.
- The "4 tenders" are 1 + 2 + 1:
  - 8596401 FASTER, the `Individual` case;
  - 8644596 (BGS) and 8646273 (STFC), both correct;
  - 8664993, where `Jeol (U.K.) Limited` carries PDQJ: a different class, filed as issue 545.
- Rejected rules:
  - a stand-in name refusing the triple: there is no correct destination;
  - "the identifier's org only buys": false, UKRI genuinely bids;
  - the notice's own buyer: that is the Met Office;
  - a 452 verdict: cannot move a triple bind, and would be false;
  - condemning PDQJ: it is pinned as a near miss and holds UKRI's 2,696 buyer mentions.

**Built (2026-10-15):**
- `"individual"` added to `BLANK_WHOLE` (59 → 60) with its reason. The guard-3 doc names it.
- Tests:
  - `an_fts_individual_is_blank_as_a_whole_name_only`: `Individual (Lot 2)` is Blank; `Individual Centred Care` and
    `Cabinet Medical Veterinar Individual …` are names;
  - `every_measured_placeholder_spelling_has_its_class` gains three spellings and an exact entry;
  - `an_fts_individual_drops_bare_and_keeps_an_admitted_identifier`: PBXY, PGYY and bare drop as Blank; PMGL and PDQJ
    mint with their identifiers; `Individual Centred Care` mints.
- **Pre-read for the drain (2026-10-15):**
  - 31966280 holds exactly 4 mentions: PBXY and PGYY slots on 46806664 and 46819715;
  - 18934187's two mentions (22342219 ORG-7 and ORG-20) both carry raw `P0004469968`, so guard 3 keeps them;
  - 31570158's one mention (46806664) carries PMGL. It is the drain's own canary for guard 3.
- **Drain:** `refold-notices` over `[46806664, 46819715]`. The fold retires the four mentions, and the auto
  orphan sweep removes 31966280. Reconcile any sweep count above 1 against its plan, since the sweep is global.
