# 332 — Is carrier count the wrong statistic for genericness?

Status: MEASURED AND CLOSED NEGATIVE 2026-09-01 (job 572, `2248a57`, 13 s).
**No. Carrier count is not inverted, and the premise this issue doubted holds
where it can be checked: 99.5% of decidable over-cap keys are genuinely shared
names.** My hypothesis was wrong. No change to the wall.
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

## The measurement (job 572, `2248a57`, 13 seconds)

4,176,452 name keys walked across `n2` and `n3`; **61,934 are over the cap of
20**, covering 5,805,215 carrier rows. Each cut by how many distinct
`(country, kind, identifier)` triples its carriers hold:

| verdict | keys | share of decidable |
| --- | --- | --- |
| `mostly-distinct` — a genuinely shared name | **13,717** | **99.5%** |
| `single-identity` — one identity, fragmented | 72 | 0.5% |
| `mostly-fragmented` | 3 | 0.02% |
| `no-identifiers` — undecidable | 39,649 | — |
| `too-little-evidence` (one identified carrier) | 8,493 | — |

Identity ratio p10 / p25 / p50 / p75 / p90 = **100 / 100 / 100 / 100 / 100%**.

## The answer, and I was wrong

**Not bimodal. One population, and it sits flat against the ceiling.** Where the
cut can decide at all, a high-carrier key is a name many different bodies really
did choose — 13,717 against 75. The wall's premise holds, and the statistic needs
no replacement.

The four rows that prompted this issue were four convenient rows, exactly as the
filing warned they might be. The specimens that settle it are in the listing:

```
carriers=62084  with_id=62080  distinct=62080   avenue web systèmes
carriers=25029  with_id= 1579  distinct= 1579   tribunal administratif de paris
carriers=16892  with_id= 5995  distinct= 5995   tribunal administratif de lyon
```

Every identifier-bearing carrier holds its **own** identity. And `siemens §ag` —
the 1,238-carrier row that prompted the whole question — falls in
`mostly-distinct` too: those rows carry many distinct identifiers, so they are
many registered entities rather than one identity smeared 1,238 ways. The
fragmentation reading was mine, not the data's.

**Recommendation: no change.** Same shape as issues 327 and 331 — a negative,
measured, recorded.

## Two things worth carrying forward

**77.7% of over-cap keys are undecidable by this route** (39,649 with no
identifier-bearing carrier, 8,493 with exactly one). That is not a failure of the
census; it is a fact about the corpus — the provisional layer is 11.5M of 12.6M
rows and mostly identifier-less. Any future attempt to improve the wall's
statistic from identifier evidence hits the same ceiling, so this route is closed
rather than merely unproductive today.

**A defect in the first version of this census, recorded because the mechanism of
catching it is the reusable part.** Job 571 reported **8,565** keys as
`single-identity`. It had seen no such thing: `distinct == 1` is arithmetically
true whenever exactly one carrier holds an identifier, and `enel spa` has 3,004
carriers and one identifier between them. The tell was not in the summary line
but in the shape of two numbers side by side — the ratio distribution read 100%
at every percentile while thousands of keys supposedly sat at one-over-many, which
cannot both be true. Reading the listed VALUES rather than the counts is what
resolved it. `with_identifier < 2` is now its own bucket, and
`one_identified_carrier_among_many_is_not_evidence_of_fragmentation` pins it.
