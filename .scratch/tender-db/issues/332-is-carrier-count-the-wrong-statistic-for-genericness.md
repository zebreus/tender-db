# 332 — Is carrier count the wrong statistic for genericness?

Status: NEEDS-TRIAGE 2026-09-01 — surfaced by issue 331's census (job 570), NOT
yet measured. A design question about the statistic, not a threshold tweak.
Kind: identity semantics (organization layer) / measurement
Relates to: 316 (which introduced the wall), 318 (which armed it in the
resolver), 300 Stage 4 (the E3 scan's stoplist), 331 (whose run surfaced this and
which deliberately did not absorb it)
Blocked by: nothing

## The observation

The genericness wall refuses a name key carried by more than `STOPLIST_CAP` (20)
distinct orgs, on the premise that such a name is one **many different bodies
chose**, so agreement on it is agreement nobody made unique. Issue 331's census
printed the carrier counts, and the top of that list does not look like that
premise at all:

```
carriers=1238   siemens §ag
carriers= 315   man truck bus deutschland §gmbh
carriers=  32   krebs kiefer ingenieure gmbh
carriers=  30   stadt roth
```

`Siemens AG` is not a name many bodies chose. Neither is `Stadt Roth` — there is
one Roth. A carrier count of 1,238 reads much more like a statement about **how
fragmented the org layer is** than about how common the name is.

If that is right, the wall is **most confident exactly where it is most wrong**:
it refuses corroboration for the largest, most distinctive, most fragmented
identities in the corpus — the ones where corroboration would help most — while
correctly refusing genuinely shared names like `Stadtverwaltung`. And it would be
self-reinforcing in the same way issue 331 described, but at a scale issue 331's
collapse cannot see.

## Why issue 331 could not answer this

Issue 331 collapsed carriers by shared `(country, kind, identifier)` triple. On
`siemens §ag` that saved **3 carriers out of 1,238** — so essentially none of
that fragmentation is identifier-provable, and the collapse is blind to it. The
question is not the threshold, and not the duplicate-collapse: it is whether
**carrier count is the right statistic at all**.

## What is NOT known — do not act on the observation above

1. **Are the 1,238 `siemens §ag` rows one company?** Unproven. They could be many
   Siemens legal entities, branches, or the same entity fragmented. The carrier
   count cannot distinguish these and neither can issue 331's collapse.
2. **Is high carrier count actually correlated with fragmentation rather than
   commonality?** The four rows above are suggestive and are also the four most
   convenient examples. A real answer needs the whole over-cap population cut by
   something independent — e.g. how many DISTINCT identifiers its carriers hold.
3. **Would a better statistic change any decision?** The wall is armed in three
   places; a change that rescues nothing in practice is not worth its blast
   radius, which is exactly the conclusion issue 331 reached for its own 59.

## Suggested first step

A census over the over-cap keys that cuts each by the **diversity of its
carriers' identifiers**, not their number:

* carriers holding no identifier at all,
* distinct `(country, kind, identifier)` triples among those that do,
* and the ratio between the two.

A key whose 1,238 carriers hold 3 distinct identifiers is a fragmented identity.
A key whose 40 carriers hold 40 distinct identifiers is a genuinely shared name.
That ratio is the candidate statistic, and the census would say whether the two
populations actually separate — measured, rather than argued from four rows.

Only if they separate cleanly is there a proposal worth writing, and it would
then go through the same ladder as every other change to an armed wall (issue
318's care being the standard).
