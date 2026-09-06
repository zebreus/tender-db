# 330 — Postal addresses leak into `organizations.name`

Status: **DECIDED 2026-09-06 (owner) — no key-builder strip, the class is too small to
earn a key-semantics change.** Both open measurements ran (job 774, `b7fe297`, seconds):
the address-shaped subset is **1,443** of the 106,447 line-broken names, **224** with a
country, and a strip would move **10** same-triple pairs from `contained` to `agree`.
The strip exists (`ingest::address::strip_trailing_address`, tested on the live
specimens) and the census keeps measuring with it; it feeds no key. Reopen if the
class grows or the E0 arm wants those ten. Was: MEASURED 2026-09-01 (job 569).
**NOT a parser defect — 0 of 356,749 were introduced downstream.**
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

## The second measurement (job 774, `b7fe297`, 2026-09-06 10:2x UTC)

Built the strip the "Next" section asked for as a **measurement input**, not a key
change: `ingest::address::strip_trailing_address` (last line a postcode-and-locality
in the five-digit / PL-PT dashed / CZ-SK-NL spaced / country-prefixed shapes, up to two
street-shaped lines before it, one line always kept, a bare four-digit code needing
corroboration because it is also a year), and the census now runs it over every
line-broken name, seeks the same-triple twins and the stripped key's carriers, and
reports under `address`. The org layer itself has changed since job 569 — the 351
provisional-echo fold and the 357/359 folds took it from 12,588,066 rows to
**6,808,407**, and the line-broken class from 356,749 to **106,447** (103,079 of them
NULL-country, still 97%).

| | |
| --- | --- |
| address-shaped (a trailing postal block the strip recognises) | **1,443** (1.4% of the line-broken class) |
| …with a country | **224** — DE 194, PL 16, FR 11, BE/CH/ES 1 each |
| …with a same-triple twin (the E0 class) | 20 |
| already agree / **would gain agreement** / still differ | 1 / **10** / 9 |
| stripped key already held by another identifier, same country | 38 |

**Question 1 answered: ten pairs.** The gain of a key-builder strip on the identifier
arms is ten E0 pairs moving from `contained` to `agree` — the issue's own specimen
among them (`22149904` → twins `2171, 10620, 22762191`, "Vergabekammer
Rheinland-Pfalz"), plus `22412187→18808732` SWEG Schienenwege, `22548249→22377041`
kommIT, `22611511→22149720,23348580` WISS, `22613968→22200296` Bucher Municipal,
`22717230→24497978` MVG Germany, `22749425→22867022` Malerbetrieb Geibel,
`22774437→22716766,22730735` Landesanstalt für Landwirtschaft Sachsen-Anhalt,
`23266075→23050872,23130167` KFB Jessen, and one past the listing cap. The nine
`still-differs` rows are department suffixes the strip correctly leaves
(`Landratsamt Kelheim\nKreisfinanzverwaltung`, `Bundeskartellamt\nVergabekammern des
Bundes`, `Sana Kliniken … Sommerfeld`) — an address strip is the wrong tool for them
and nothing here should try.

**Question 2 answered: the strip does not fabricate agreement.** Every stripped value in
the listing reads as the entity's own name (`Siemens AG`, `Ziehm Imaging GmbH`,
`Janssen-Cilag Polska Sp. z o.o.,`), and the 38 "collisions" are that name's OTHER rows
under other identifiers — `Siemens AG` with 41 carriers, `Komtur Polska` 65 — i.e. the
entity's echoes and second registrations, exactly what the clean spelling already
collides with. The strip hands a row the key its clean twins hold; whether those rows
merge stays with the arms' denial stack, unchanged.

**Decision: not built into the key builder.** 1,443 rows of 6.8 M (0.02%), ten pairs on
the identifier arms, and a strip in `match_norm`/N3 is a `NAME_KEY_EPOCH` bump — a
from-zero rebuild of `org_match_keys` for a class this size. The strip stays as a
tested function and the census keeps reporting the class each run, so the decision is
cheap to reverse: if `address.twins.gains_agreement` or `address.shaped` grows, or the
E0 arm is running and wants its ten pairs, build it then. The ten pairs are listed
above for a verdict path if one is wanted sooner.

**Adversarial audit (2026-09-06 17:xx UTC), two follow-ups landed.** (1) The census's
per-row seeks were bounded only by the measured size of the class, not by code: a
`ADDRESS_SEEK_CEILING` of 20,000 country-bearing address-shaped rows now stops the
twin/carrier seeks (rows past it count and list as `no-twin`, `address.seeks_truncated`
says so, the tallies become a lower bound) so the walk stays seconds whatever the class
does. (2) The strip's corroboration asymmetry is now documented and pinned: only the
bare four-digit shape needs a street line or a country prefix; a bare `<five digits>
<word>` line is stripped as an address even when it is a labelled number
(`Kommission\n54321 Sonderfall` → `Kommission`). Accepted while the strip feeds only the
census — whose listing shows every stripped value — and to be tightened before any key
builder consumes it. Also pinned: the `still-differs` verdict (a department line
surviving above the postal block), the two-letter country prefix, and the two-line
walk-back cap.
