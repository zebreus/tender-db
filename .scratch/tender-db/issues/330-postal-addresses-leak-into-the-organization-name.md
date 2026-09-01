# 330 — Postal addresses leak into `organizations.name`

Status: MEASURED 2026-09-01 (job 569, `8f4b2dc`, 7 s).
**NOT a parser defect — 0 of 356,749 were introduced downstream.** The remaining
question is a normalisation policy, and it is NOT decided here.
Kind: data quality (organization layer)
Relates to: 329 (where it surfaced), 300 Stage 4 (`org_match_keys` is built from
these names, so a polluted name is a polluted key)
Blocked by: nothing

## The finding

Issue 329's census listing shows organization names carrying an embedded postal
address, newlines and all:

```
DE355604198  4 rows, 12,249 mentions
  Vergabekammer Rheinland-Pfalz
  Vergabekammer Rheinland-Pfalz
  Stiftsstraße 9
  55116 Mainz
```

That is ONE `organizations.name` value with the street and postcode appended.

## Why it is worth an issue rather than a shrug

The name is not cosmetic here — it is load-bearing in three places:

* `n2_key` / `n3_key` are computed FROM it, so an address-polluted name produces
  a key that matches nothing, and the org silently drops out of every
  name-corroborated arm (R3, the Stage-4 E3 scan, the resolver's anchor bind).
* It defeats the exact-name corroboration those arms require, so the failure is
  a **silent under-merge** — the shape that leaves no trace to notice.
* The specimen carries 12,249 mentions, so it is not a curiosity of the tail.

## What is NOT yet known, and must be measured before anything is changed

1. **How many.** No count has been run. The specimen is one row in a 400-row
   capped listing; the class could be a handful or tens of thousands.
2. **Where it enters.** Publisher-side (the notice really does put the address in
   the name element) or parser-side (a mount that concatenates sibling elements)?
   These want opposite fixes, and guessing wrong means "repairing" data that was
   published correctly.
3. **Whether the published string must survive.** Issue 328's precedent says yes:
   `organization_mentions.raw_identifier` keeps what the notice said, and the
   repair only moves the derived value.

## Suggested first step

A census, not a repair: count `organizations` whose `name` contains a newline or
matches a postcode-shaped trailing token, cut by country and by profile, and
sample twenty against their notices to answer (2). Only then decide.

## The measurement (job 569, `8f4b2dc`, 7 seconds)

| | |
| --- | --- |
| organization rows walked | 12,588,066 |
| names carrying a line break | **356,749** (2.8%) |
| mentions attached to them | 377,521 |
| **published that way** (some mention carries the break too) | **356,665** |
| **derived-only** (introduced downstream — would be ours) | **0** |
| no mentions to compare | 84 |
| name length p50 / p90 / p99 / max | 24 / 62 / 137 / **7,497** characters |

### Question 2 is answered, and the answer closes a branch

**Zero derived-only.** Every single one was published that way. This is not a
parser defect and no parser change would have prevented it, so the "concatenating
mount" hypothesis in the original filing is dead. That was worth knowing before
touching anything: a repair built on it would have rewritten data the publisher
sent correctly.

### The scale depends on WHICH layer, and my first cut of this was wrong

353,306 of the 356,749 — **99.0%** — carry **no country at all**: the
unresolved/provisional tail. I first wrote that those are harmless because no
merge arm keys them. **That is only true of the identifier-keyed arms.**
`build_org_match_keys_batch` walks `SELECT id, name FROM organizations WHERE id
> ?` with **no filter at all**, so every one of the 356,749 polluted names is
keyed into `org_match_keys` and produces a superset key there. So:

* **identifier-keyed arms** (R2 through `canonical_key`, which needs country +
  kind + identifier): only the 3,443 country-bearing rows can matter.
* **name-key layer** (`org_match_keys`, the Stage-4 E3 candidate scan, R3's name
  corroboration, the resolver's ingest-time anchor bind): **all 356,749 matter.**

The country-bearing remainder, for the identifier arms:

```
DE 1769   PL 587   FR 292   ES 245   IT 213   GR 135   AT 60   IE 21   BE 20
…and 3,443 in total across 34 countries
```

## THE MECHANISM, corrected — my filing overstated it

The original text said a polluted name "produces a key that matches nothing".
**That is wrong, and the correction matters because it changes the severity.**
`project::match_norm` maps every non-alphanumeric character to a gap, so a
newline is already just a space:

```
"Vergabekammer Rheinland-Pfalz\nStiftsstraße 9\n55116 Mainz"
  → "vergabekammer rheinland pfalz stiftsstraße 9 55116 mainz"
```

The normaliser is not defeated by the break. What survives is the **extra address
tokens**, so the polluted name yields a key that is a strict token **superset** of
the clean one — not a key that matches nothing.

### The harm is real, bounded, and has a live specimen

That superset relation is exactly what issue 329's census bucketed as
`contained`, and `DE355604198` is the proof: its two names are

```
Vergabekammer Rheinland-Pfalz
Vergabekammer Rheinland-Pfalz\nStiftsstraße 9\n55116 Mainz
```

so the group landed in **`contained`** — the undecided bucket — when it should
have read as **`agree-distinctive`**, the clean fold signal. The pollution does
not hide the organization; it **demotes an unambiguous agreement into an
ambiguous one**.

The direction of the error is worth stating, because it bounds the risk: a
superset key is *more* specific, so it matches fewer things. The failure mode is
therefore uniformly **under-merge**, never false-merge — these rows quietly fail
to find their twins rather than finding the wrong ones. That is the benign
direction to fail in, and it is why this is a normalisation question rather than
an incident.

## Next — and this is a decision to be measured, not taken here

The candidate response is a key-builder-side strip of a trailing
postcode-and-city block, so the clean and polluted spellings produce one key. It
is **not** a repair of `organizations.name`: the publisher sent that string and
issue 328's precedent is that the published value stays.

Before proposing it, two things need measuring, and neither is done:

1. **How many of the 3,443 would actually gain a merge** if the address were
   stripped — i.e. how many sit in a duplicate group that a strip would move from
   `contained` to `agree-distinctive`. The issue-329 census can answer this by
   re-running against stripped keys.
2. **The false-merge risk of the strip itself.** A trailing token that looks like
   a postcode can be part of a real name, and a strip that eats it would fabricate
   agreement. This needs the same denial-stack care every other arm got.

The `p99 = 137` / `max = 7,497` character distribution also gives the filing's
other candidate signal ("suspiciously long name") a measured shape for the first
time, if anyone wants to pursue it separately.
