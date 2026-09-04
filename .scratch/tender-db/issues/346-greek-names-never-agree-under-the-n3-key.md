# 346 — Greek names never agree under the N3 key: upper-case drops the tonos, `match_norm` keeps it

Status: BUILT 2026-09-04 (gate green; deploying) — `match_norm` folds the precomposed Greek tonos/dialytika vowels and the final sigma after lower-casing, Greek-only (the `MÜLLER`/`MULLER` guard is pinned); `NAME_KEY_EPOCH` → `n2v2+n3v1`, so the next `build-org-match-keys` (Sunday's tick, or the one enqueued after this deploy) restarts from zero. Then: re-run the duplicate-identity census (expect GR:national agree-distinctive 134 → ~156) and the E0 dry run. Was: ready-for-agent (filed 2026-09-04 from the E0 fold's dry run, job 640, `12434d4`)
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
