# 330 — Postal addresses leak into `organizations.name`

Status: NEEDS-TRIAGE 2026-09-01 — noticed while reading the issue-329 census
report, not yet sized.
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
