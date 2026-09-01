# 331 — The genericness wall counts duplicate org rows as separate carriers

Status: NEEDS-TRIAGE 2026-09-01 — mechanism identified from a live specimen,
frequency NOT yet measured.
Kind: identity semantics (organization layer) / measurement
Relates to: 316 (which introduced the wall), 318 (which armed it in the
resolver), 300 Stage 4 (the E3 scan's stoplist), 329 (where it surfaced)
Blocked by: nothing

## The finding

`STOPLIST_CAP` is 20: a name key carried by more than 20 distinct orgs is
"generic", and agreement on it cannot corroborate a merge. The carrier count
comes from `org_match_keys`, which holds a row per **org row**.

But a fragmented organization IS several org rows. So the duplicates the merge
arms exist to fold **inflate the carrier count of their own name**, and can push
a perfectly distinctive name over the wall.

Live specimen, from the issue-329 census (job 568): `DE115302781` is held by four
org rows all named `H. Hüther GmbH`, and the census classified the group
`agree-generic`. That is not a generic name by any reading.

## Why this matters beyond a mislabelled census bucket

The wall is not just a census input. It gates:

* the R3 batch merge arm's corroboration (issue 316),
* the resolver's ingest-time anchor bind (issue 318),
* the Stage-4 E3 candidate scan's stoplist.

In each of those a false "generic" verdict is a **silent refusal to merge** —
the same invisible under-merge shape as issue 330. And it is self-reinforcing:
fragmentation makes a name look generic, looking generic prevents the merge,
the fragmentation persists.

## What is NOT known

**How often it actually bites.** A name needs >20 carrying rows to cross the
wall, and the census's own group sizes are small (the `H. Hüther` group is four).
So the mechanism is real but the frequency may be tiny — reaching 21 carriers
from duplication alone needs heavy fragmentation. **Do not change the wall on
the strength of one specimen.** Issue 312 is the standing precedent for "looks
untidy, measured false-merge rate says leave it alone".

## Suggested first step, and the shape of a fix if one is warranted

Measure first: for each name key over `STOPLIST_CAP`, count how many of its
carriers share a `(country, kind, identifier)` triple with another carrier —
i.e. how much of the count is duplication rather than genuine commonality. That
number decides whether anything needs doing.

If it does, the candidate fix is to count **distinct identities** rather than
distinct org rows — carriers collapsed by their identifier triple where they
have one. That keeps the wall's meaning ("a name many DIFFERENT bodies chose")
while making it robust to the fragmentation it sits next to. It touches three
armed walls, so it would need the same care issue 318 took.
