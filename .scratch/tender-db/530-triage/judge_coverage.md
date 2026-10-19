# 530 unit 2 — coverage judge (2026-10-19)

Inputs: issue 530 (What was found, Proposed units, Unit 1 measured), census job 2655
(`guid_census_2655.json`, decoded body breakdowns and samples), `idgate.rs` (`condemns`, `uuid_v4`, the
standard for adding a class), `guid_census.rs` (definitions), 312, 300 design:84-85 and §10.
13 public GETs (2 s apart, `/v1/organizations` only). Raw responses: scratchpad, not committed.

Lens: COVERAGE FIRST. The harm is 74,023 single-use v4-GUID rows (91.0%) that each keep one notice's
mentions away from the row the name already has. A search by buyer, review chamber or winner misses them.

## Verdict

**(d) + (a, notice-scoped), both only for a GUID with no standing row, and a dissolve that also folds
the cross anchors.** (b) is rejected because no clean boundary exists in any measured breakdown.
(c) is rejected because it leaves 74k split rows, and the class still grows (2026: 17,533 single-use rows
so far).

## What each option fixes (single-use rows reached; denominator 74,023)

| option | reached | stable rows hit | left |
|---|---|---|---|
| (a) pair | 11,258 (15.2%) | 70 | 62,765 |
| (a) notice-scoped | 18,415 (24.9%) | 114, incl. 254 and 255 | 55,608 |
| (d) cross, ≤2 bar | 55,783 (75.4%) | 11 at mint time (41 names) | 18,240 |
| (a) pair ∪ (d) | 56,280 (76.0%) | — | 17,743 |
| **(a) notice ∪ (d)** | **59,493 (80.4%)** | none, given the standing-key exemption | **14,530** |
| + cross anchors (dissolve only) | up to 63,974 (86.4%) | 0 (stable rows excluded) | at least 10,049 |

Anchors: the census counts a row as cross only when an EARLIER GUID row with ≤2 mentions exists under
its key. So the first row under each of the 4,481 cross names (Westfalen 39, Südbayern 7722, Bremen
15567, Luzern 813) is never reached, though the name proves it is a per-notice key. Folding a
cross row's anchor adds up to 4,481 rows. The overlap with the twin set is not measured, so this is an
upper bound.

Reach by slice (notice ∪ cross):
- Role: buyer 87.4%, review_or_info 93.3%, other 99.7%, **tenderer_winner 33.1%** (4,384 of 13,230).
- Country: DE 82.0%, CH 78.7%, FR 83.4%.
- Platform: doe+ted dashed-lower 81.0%, ted dashed-lower 77.6%.

Notice-scoping is the lever for winners. Pair twin reaches 39 winners and notice twin 3,585. Of the
+3,213 rows that notice scope adds over pair ∪ cross, 2,938 (91%) are winners.

## Why (a) and (d) need the standing-key exemption

- **(a) is evidence, not a category proof.** 179 twin rows are multi-tender and 70 are stable. Some
  platforms publish TWO stable keys for one org in one notice: SBH | Schulbau Hamburg 3866/3868 (4,472
  and 4,548 mentions), GMH 22040636/37, and Stromnetz Berlin, Mainova and Universitätsklinikum Erlangen
  in the twin samples. So a twin does not prove per-notice minting the way `notice_number`'s 425/425
  did. For 98.4% of twin rows it does hold.
- **(d) hits 11 stable rows at mint time.** Vergabekammer Westfalen 5351 (271 mentions) and 8756 (126)
  came after single-use row 39. Notice-scoped (a) reaches 254 and 255.
- **The exemption covers both:** a GUID that already keys a row always binds. So nothing standing
  changes, and the Verify control (254, 255, 8378) holds by construction.
- **A wrong refusal of a FUTURE stable key is cheap for coverage.** Its mentions go to the name's 234
  row, which for these names is usually the biggest row already:
  - Vergabekammer Westfalen: DE 234 row 3233 has 37,696 mentions; the "stable" GUID rows have 422, 271
    and 126, beside 185+ GUID rows in all.
  - Südbayern: 234 row 9766 has 10,423; stable row 1348 has 4,430, beside 184+ GUID rows.
  - SBH: 234 row 11219650 has 2,919, beside its two stable GUID rows.

  What is lost is linking across name variants, plus homonym separation (the 234 baseline).

## Why (b) is not clean

- **Platform** (source + raw format): doe+ted dashed-lower holds 40,388 single-use rows AND 338 of the
  345 stable rows (254, 255, 8378). ted dashed-lower is 94.0% single-use with 0 stable, but has 1,821
  multi-tender rows. Swiss bidder keys are stable: FR_Swisscom (Schweiz) AG is one row, 75 mentions,
  53 tenders. Swiss buyer keys are not: Genève Aéroport has 180 rows of 1 mention each and no other row.
- **Profile**: eforms-de-1.1+sdk-1.7 is 97.3% single-use with 0 stable, but it is a schema era. 254's
  and 255's notices carry the same profiles, so a per-notice profile rule would strip their mentions.
  Minting continues under de-2.1+sdk-1.13 (4,495/5,054) and sdk-1.12/1.14.
- **Era** (single-use share by year first seen): 2023 70.6%, 2024 89.0%, 2025 92.2%, 2026 96.1%. There
  is no cutover. Stable rows were mostly born in 2023 (291/46/8/0), but that is maturity-biased.
- **Country** CH: 94.0% single-use, 0 stable, 0 twin. But it is per-role (buyers mint per notice,
  bidders keep stable keys), and country is not a publisher.
- **The publisher fields** (eSender party, submission URL) are `not_measured`.

## Also considered and not chosen: refuse every new GUID and grandfather the standing rows

This reaches 100% of new mints going forward, but it is (b)-by-era with no measured boundary. Its
marginal gain is the 14,530 residue, and refusing a residue row often gains nothing: when the GUID row
is the only row of its name, a fold only swaps a GUID singleton for a provisional one. Two such names:
- Baumaschinen Beckschulte KG: 1 row in the corpus.
- DE_Fiorentino Haustechnik GmbH: 1 row.

Its cost is every future stable key, and the census cannot size that.

## Rule wording (gate, beside `idgate::condemns`; `condemns` and `uuid_v4` unchanged)

> Applies to an ORG mention whose normalised identifier is kind `national` and `idgate::uuid_v4(value)`.
> If an organization row is already keyed by its exact (country, kind, value) triple, the mention binds
> that row, as today. Otherwise the GUID is REFUSED — the mention takes the issue-234 identifier-less
> path to the (org_name_norm(name), country) provisional row, `raw_identifier` keeps the published
> value — when either:
> (a) TWIN NOTICE: the mention's notice publishes, on two different ORG sections, two different v4
>     GUIDs under one org_name_norm, neither of which keys a standing row (the pair need not include
>     this mention); or
> (d) KNOWN NAME: its (org_name_norm(name), country) already holds a different v4-GUID row with at most
>     2 mentions (`GUID_CENSUS_FEW_MENTIONS`), or is a key under which this rule or the 530 dissolve has
>     refused or folded a v4 GUID.
> Otherwise it mints as today.

Notes on the wording:
- **The second (d) clause** (a recorded key: a marker, or a v4 raw_identifier on the key's 234 row)
  is what keeps the rule working after the dissolve. Without it, folding removes every ≤2 row and each
  name regrows one row. Göttingen would read 1 instead of 0.
- **The "neither keys a standing row" clause** keeps stable-twin notices (SBH) from refusing new
  winners there. On a minting platform the pair is always new, so it costs nothing there.

**Dissolve (unit 3, through 440's path with an id-set selector).** `repair_placeholder_orgs_batch` takes
only a pure value predicate, so unit 3 needs an id-set selector. A row is folded when all of these hold:
- it is a v4-GUID row whose mentions span ONE distinct Tender;
- it has fewer than 100 mentions and is not 254, 255 or 8378;
- it is twin-notice-reached, cross-reached, or the anchor of a cross-reached row.

Each fold records its key for (d). Göttingen → 2294259 (DE 234 row, 216 mentions), never VAT row
22183653 (300 §10). The dissolve must re-run periodically, because the gate still lets the first GUID of
a never-seen name mint.

## Expected effect

- **Stock:** 59,493–63,974 single-use rows folded (80.4–86.4%), with 0 multi-tender and 0 stable rows.
  - Verify line 1: Göttingen 52 → 0 (twin, cross and anchor), and 2294259 → about 320 mentions.
  - Südbayern 30971684/734/757 and 7722 → 9766.
  - Bremen 30532145/147 and 15567 → its DE 234 row.
  - Genève Aéroport 180 → 1 (a new CH 234 row).
  - 254, 255 and 8378 are untouched (Verify lines 2-3).
- **Forward:** at least the stock's 80% of new per-notice GUIDs are refused at the gate. The marker
  covers every folded key, and the rest are folded at the next dissolve once their name recurs.
- **Left: 10,049–14,530 single-use rows.**
  - By role: winners 8,846, buyers 3,862, review 1,811.
  - By country: DE 7,556, CH 5,936, FR 597.
  - By year first seen: 2025 5,739, 2026 4,091, 2024 4,031.
  - The reads show three shapes:
    - True singletons (Beckschulte, Fiorentino).
    - Per-procedure families above the ≤2 bar: the Brandenburg "beim Ministerium" variant (844 and 1214,
      4 mentions each) and BW "im Regierungspräsidium Karlsruhe" (9 GUID rows, 43 mentions).
    - One GUID row beside a 234 row (Gemeinde Asbach-Bäumenheim: GUID 2 mentions, DE 234 row 5).
  - The 7,300 multi-tender rows stay as well.

## Risks

1. **Homonym folds:** a (name_norm, country) key shared by different entities, the 234 baseline.
   Read R3 checks the biggest keys.
2. **A future stable key born under a known name is refused forever.** The historical rate is 11 of
   345. Its mentions land on the 234 row, which costs name-variant linking.
3. **The rule depends on order.** The first GUID of a new name mints. A full rebuild regrows up to one
   row per key until the next dissolve, so the marker must persist or be derivable from raw_identifier.
4. **Gate cost:** (d) needs an indexed (name_norm, country) seek per new-GUID mention.
5. **Refold alone does not move recorded mentions** (434 `RecordedMention::publishes`), so the dissolve
   is required.
6. **Typo variants stay apart** (the BW Würtemberg family has 3 keys, each with its own 234 row). This
   is no worse than today.

## Read that must pass before the build ships (the unit-3 dry plan, same rev)

- **R1, controls (deterministic):**
  - The planned folds fall within [59,493, 63,974]. Rows minted after job 2655's max id 31,970,152
    are counted separately.
  - 0 planned rows with ≥2 tenders or ≥100 mentions.
  - None of the 345 stable rows is planned (254, 255, 8378, and Westfalen's 5351, 8756 and 21987063
    among them).
  - Göttingen 52/52 → 2294259, Südbayern three + 7722 → 9766, Bremen two + 15567 → one row.
- **R2, sample:** 60 random planned folds, 20 buyer / 20 review_or_info / 20 tenderer_winner, about
  34 DE / 23 CH / 3 FR or other.
  - For each, check that the GUID occurs in one tender only, and that the target 234 row is the same
    entity, judged by name plus town or address on the notices.
  - Bar: **0 W, ≤3 N**. Any W excludes that shape, and a fresh 60 is read.
- **R3, generic names:** the 20 keys with the most planned folds must each be one body per country,
  with 0 generic keys admitted. A generic key is excluded from both the dissolve and (d).

## 300 design:84-85, answered

"Merge-inert singletons, not junk to dissolve":
- **On the merge side it stands.** No v4 GUID becomes a merge key (312 step 1a). `condemns` is
  unchanged, and the value is never refused for its shape alone. All 7,300 multi-tender rows, the 345
  stable ones among them, keep binding.
- **On the split side it is wrong for the v4 subclass.** "Singleton" is the harm, not a reason to leave
  the rows alone: 91.0% of the class spans one tender, and each such row is split-active. Westfalen has
  185+ GUID rows beside a 37,696-mention 234 row; Genève Aéroport has 180 one-mention rows.
- **So the single-use rows that the notice (twin) or the name (cross, anchor) shows to be per-notice
  keys ARE dissolved.** They go to their 234 row as per-notice keys, not as junk values, and the value
  survives on raw_identifier.
- **What remains a merge-inert singleton in 300's sense:** the 10–14.5k unreached rows, which are mostly
  the only row of their name.
- **The rest of the 273,982 hex class is not decided here.**

## Follow-ups (not part of this decision; the dry run counts them)

- **F1:** widen (d)'s bar from ≤2 mentions to "one distinct tender" (the per-procedure families), if the
  dry-run count shows 0 stable collisions.
- **F2:** residue rows whose key already holds a 234 row (the Asbach shape).
- **F3, new issue:** per-notice hex outside `uuid_v4`.
  - Avenue-Web Systèmes (FR) has 191 one-mention 32-hex national rows; only 6 are v4.
  - FR "ted other" rows are random hex that pass the v4 nibbles by chance (1/64). The predicted
    nibble-only count is 3 × 3,511 = 10,533, and the census found 10,548.
  - So the class is about 225k rows (an estimate), roughly 300's hex-hash population, and it is
    unmeasured on the split side.
