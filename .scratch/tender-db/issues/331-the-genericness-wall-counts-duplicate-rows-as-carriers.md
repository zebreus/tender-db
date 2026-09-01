# 331 — The genericness wall counts duplicate org rows as separate carriers

Status: MEASURED 2026-09-01 (job 570, `7543612`, 3 s).
**Not inert: 59 keys are provably falsely generic today.** Small, and all within
3 carriers of the wall. But the run surfaced a LARGER finding that this issue did
not ask about and should not absorb — filed separately as issue 332.
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

## The measurement (job 570, `7543612`, 3 seconds)

| | |
| --- | --- |
| duplicate identity groups feeding the collapse | 3,458 (7,305 org rows) |
| name keys whose count is inflated at all | 4,264 |
| …of which over the stoplist cap of 20 today | 635 |
| **…falsely generic** (over the cap now, at or under it collapsed) | **59** |
| upper bound on orgs behind them | 1,245 |

So the answer to the question as filed is **not zero** — the mechanism is live, not
inert. I expected inert and was wrong.

But the shape of the 59 argues against acting on them, and it is the shape rather
than the count that decides. **Every one sits within three carriers of the wall**:

```
carriers=21 savings=3 collapsed=18   köhler transfer gmbh co kg
carriers=21 savings=2 collapsed=19   dekra automobil gmbh
carriers=22 savings=2 collapsed=20   r v allgemeine versicherung ag
carriers=21 savings=1 collapsed=20   glaxosmithkline gmbh co kg
carriers=21 savings=1 collapsed=20   entega ag
```

Carriers 21–22 against a cap of 20, savings of 1–3. A collapse-duplicates fix
would rescue only keys already grazing the threshold, and it would touch three
armed walls (R3's corroboration, the resolver's anchor bind, the E3 scan) to do
it. That is a poor trade on its own merits, and issue 312's precedent covers it:
measured, real, and not worth the blast radius.

**Recommendation: no change to the wall on these grounds.** Leave the cap and the
counting as they are.

## What the run actually surfaced — and it is not this issue

The contrast rows are the interesting output:

```
carriers=1238  savings=3  siemens §ag
carriers= 315  savings=4  man truck bus deutschland §gmbh
carriers=  32  savings=3  krebs kiefer ingenieure gmbh
carriers=  30  savings=3  stadt roth
```

`siemens §ag` is carried by **1,238 org rows**. The wall's premise is that a key
carried by many orgs is *a name many different bodies chose*, so agreement on it
is agreement nobody made unique. For `siemens §ag` that premise looks
straightforwardly false: this is one highly distinctive company name, and 1,238
carriers is a statement about how fragmented the org layer is, not about how
common the name is. Same for MAN Truck & Bus at 315 and `stadt roth` — one town —
at 30.

If that reading holds, the wall is not merely inflated at the margin; it is
**most confident exactly where it is most wrong**, refusing corroboration for the
biggest, most distinctive, most fragmented identities in the corpus — the ones
where corroboration would do the most good.

**I am not asserting it holds.** The carrier count alone does not prove the 1,238
rows are one company rather than many Siemens entities, and my collapse cannot
tell: it proves duplication only through a shared
`(country, kind, identifier)` triple, and the savings on `siemens §ag` are 3 out
of 1,238 — so almost none of that fragmentation is identifier-provable, which is
the conservative limitation this census was built with and stated up front.

That is a different question, needing a different measurement, and it is about
the *statistic* rather than the *threshold*. **Filed as issue 332** rather than
folded in here, so this issue can close on the answer it actually asked for.
