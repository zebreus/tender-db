# 346 — Greek names never agree under the N3 key: upper-case drops the tonos, `match_norm` keeps it

Status: DONE 2026-09-04 — fold deployed (`d37e9bf`), keys rebuilt under `n2v2` (job 641, 92 s), and the probe (issue 348) settled the tally: **15 of the 17 folded groups read agree-distinctive and are in the E0 plan; 2 are generic; and 16 previously-distinctive GR groups crossed the wall because both casings now count as one key** — a wall weakness on fragmented Greek public bodies, filed as issue 349. Net E0 plan 1,874 → 1,873. Was: BUILT 2026-09-04 (gate green; deploying) — `match_norm` folds the precomposed Greek tonos/dialytika vowels and the final sigma after lower-casing, Greek-only (the `MÜLLER`/`MULLER` guard is pinned); `NAME_KEY_EPOCH` → `n2v2+n3v1`, so the next `build-org-match-keys` (Sunday's tick, or the one enqueued after this deploy) restarts from zero. Then: re-run the duplicate-identity census (expect GR:national agree-distinctive 134 → ~156) and the E0 dry run. Was: ready-for-agent (filed 2026-09-04 from the E0 fold's dry run, job 640, `12434d4`)
Kind: identity semantics (organization layer) — the name key, not the identifier key
Relates to: 329 (the E0 fold's 4b name rule is what this gap denies), 316 (the
genericness wall this key feeds), 300 Stage 2 (R2's name-side evidence)
Blocked by: nothing

## Observed

`crosswalk::n3_key` is `project::match_norm` (lower-case, alphanumeric tokens,
**combining marks kept**) with legal-form families abstracted. Greek orthography
writes the tonos in lower/mixed case and drops it in ALL CAPS, so the same
municipality published two ways gives two keys that differ on every accented
token:

| name as published | `match_norm` |
| --- | --- |
| `Δήμος Αβδήρων` | `δήμος αβδήρων` |
| `ΔΗΜΟΣ ΑΒΔΗΡΩΝ` | `δημος αβδηρων` |

Latin-script upper-casing keeps its diacritics (`MÜLLER` → `müller`), so this
is a Greek-specific hole — and Greek is the one scope where the E0 fold has no
identifier arm to fall back on (issue 329: the 14-character authority codes).

**Size, from a bounded read of all 210 GR:national same-triple groups on
2026-09-04** (the E0 dry run's class):

| verdict | groups |
| --- | --- |
| agree under the live key (E0 merges these) | 138 |
| **agree ONLY under an accent-insensitive, parenthesis-stripped fold** | **22** |
| contained under that fold | 14 |
| disagree under that fold | 36 |

The 22 are all of the `Δήμος X` / `ΔΗΜΟΣ X` shape — one entity, two casings.
Issue 329's specimen 311/1079 (`Ενιαία Αρχή Δημοσίων Συμβάσεων (Ε.Α.ΔΗ.ΣΥ)` vs
`ΕΝΙΑΙΑ ΑΡΧΗ ΔΗΜΟΣΙΩΝ ΣΥΜΒΑΣΕΩΝ`) is the same shape plus a parenthesised
abbreviation, i.e. `contained` even after the fold.

## Proposal

Fold combining marks in `match_norm` for **Greek script only** (NFD, drop
`Mn` marks whose base is Greek, recompose) — or, narrower still, strip the tonos
(U+0301 after a Greek vowel, and the dialytika/tonos combinations). Two reasons
to keep it Greek-scoped rather than folding every diacritic:

1. The N2/N3 keys feed `org_match_keys` and the genericness wall (316): a
   corpus-wide diacritic fold changes every DE/FR/PL/CZ key at once, moves
   carrier counts under the wall, and would need a full key rebuild plus a
   re-measure of 331/332 before anything downstream could be trusted.
2. Only Greek has the "upper-case drops the mark" property; elsewhere the fold
   buys nothing the data needs.

Then: rebuild the `org_match_keys` rows for orgs whose key changes (GR-scoped,
bounded), re-run the duplicate-identity census (expect GR:national
agree-distinctive 134 → ~156), and let the next E0 wet pass fold the 22.

Parenthesised abbreviations (`(Ε.Α.ΔΗ.ΣΥ)`) are a separate, smaller question:
stripping them from the key turns 311/1079 from `contained` into `agree`, but
`X GmbH (Niederlassung Nord)` is exactly the branch-office shape `contained` was
invented to hold. Leave them in; 311/1079 goes through a 311 case-review verdict.

## Done when

- `match_norm("ΔΗΜΟΣ ΑΒΔΗΡΩΝ") == match_norm("Δήμος Αβδήρων")` with a test that
  also pins `match_norm("MÜLLER") != match_norm("MULLER")` (the scope guard);
- the GR-scoped `org_match_keys` rebuild has run on prod and the census's
  GR:national agree-distinctive count moved by ~22;
- the E0 pass folded them (ledger `rule = 'e0'`, GR members).

## Deployed, rebuilt, re-censused (2026-09-04 05:0x–05:1x UTC)

`d37e9bf` deployed 05:09; `build-org-match-keys` wet (job 641) restarted from
zero under `n2v2` and finished in **92 s** — 13,037,751 key rows (11,310,972
n2 + 1,726,779 n3) over 12,588,839 orgs / 16,954,523 names, 1,259 windows. The
"~<2 h" estimate this issue carried was stale by a factor of ~70; the Stage-4
plan's figure predates the sorted bulk load.

Then the census (job 642) and the E0 dry run (job 643), against the rebuilt keys:

| GR:national verdict | before (639) | after (642) |
| --- | --- | --- |
| agree-distinctive | 134 | **133** |
| agree-generic | 4 | **22** |
| contained | 15 | 20 |
| disagree | 57 | **35** |

LT:national unchanged (43 / 9 / 16 / 33) — the fold is Greek-scoped as pinned.
Corpus-wide: agree-distinctive 2,030 → 2,029, agree-generic 357 → 375,
contained 600 → 605, disagree 679 → 657; E0 plan 1,874 → **1,873**,
`denied_names` 1,589 → 1,590.

**So the 22 left `disagree` as predicted (17 to agreement, 5 to `contained` —
the parenthesised-abbreviation shape, 311/1079 among them), but the wall then
called 18 more GR keys generic, and the fold bought E0 nothing.** Two readings
fit the tally equally well and I could not tell them apart from the box:

1. the 17 folded keys themselves crossed the 20-carrier wall once both casings
   counted as one key (a municipality's rows are exactly the many-rows-one-entity
   shape issue 331 asked about), or
2. the 17 read `agree-distinctive`, and 18 OTHER large-municipality keys that
   were distinctive under `n2v1` crossed the wall for the same reason.

A proxy through `organizations.name IN (both spellings)` gave ≤ 8 rows for every
folded key — which says reading 1 is wrong — but the wall counts head AND
satellite names by org id, and the proxy cannot see satellites. The window that
answers this did not exist: **filed and built as issue 348**
(`GET /admin/name-key`), deploying with 347. The verdict on this issue waits for
that probe; until then the fold stands (it is correct at the key level and
harmless: a generic key denies, it never merges).

## Settled by the probe (issue 348, `aae8dd6`, 05:3x UTC)

`GET /admin/name-key` over all 155 agreeing GR:national groups (138 that agreed
under `n2v1`, 17 the fold brought in), carriers counted under the rebuilt keys:

| class | distinctive | generic |
| --- | --- | --- |
| uniform (agreed before the fold) | 118 | **20** |
| folded (agree only since the fold) | **15** | 2 |

Reading 2 was right, and it was not hurtful in the way reading 1 would have
been: the 15 folded groups now read `agree-distinctive` and sit in the E0 plan
(`Δήμος Αβδήρων`, `Δήμος Χαλκηδόνος`, `Δήμος Γρεβενών`, …, carriers 2–8 each).
The 20 generic uniform keys were 4 before the rebuild; the other 16 crossed the
wall because their mixed-case and all-caps rows now share one key — and those
rows are the SAME entity fragmented, not a shared name: `ΓΕΝΙΚΟ ΝΟΣΟΚΟΜΕΙΟ
ΣΕΡΡΩΝ` 44 carriers (25 of the 30 shown are NULL-country, identifier-less
provisional rows), `ΔΗΜΟΣ ΑΓΡΙΝΙΟΥ` 28 (26 without identifier), `ΓΕΝΙΚΟ
ΝΟΣΟΚΟΜΕΙΟ ΒΟΛΟΥ ΑΧΙΛΛΟΠΟΥΛΕΙΟ` 36 (one identifier among 30 rows), `ΕΝΙΑΙΑ ΑΡΧΗ
ΔΗΜΟΣΙΩΝ ΣΥΜΒΑΣΕΩΝ` 54 (18 distinct hand-typed authority codes). The
distinctive keys' carrier counts run 2–20 with the mode at 5. That is issue
331's question answered for Greek public bodies — the wall's statistic conflates
fragmentation with genericness there — and it is its own issue, **349**.

311/1079 now reads `contained` (the parenthesised abbreviation), as this issue
predicted, and its key is generic anyway (54 carriers); the route for that pair
stays a 311 case-review verdict. The E0 net effect of this issue is −1 plan
group today and +15 once 349 lifts the wall for the fragmented shape.
