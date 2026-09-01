# 336 — An unmatchable filter value is indistinguishable from "no results"

Status: CLOSED NOT-WORTH-IT 2026-09-01 — measured the same firing it was filed
in. **Zero real callers use a non-matching country value.** The one apparent
oddity in the logs (lowercase) is already handled. Filed, measured, closed; the
reasoning is kept because the next person to notice this deserves the evidence
rather than the intuition.
Kind: usability (public API)
Relates to: 48 (where it surfaced — the alpha-3 habit is exactly who this bites),
118 (which introduced `ignored_filters`), 120 (bounding expensive queries)
Blocked by: nothing

## The observation

```
GET /v1/tenders?country=DEU&limit=1  ->  200, items: [], ignored_filters: []
GET /v1/tenders?country=ZZ&limit=1   ->  200, items: [], ignored_filters: []
GET /v1/tenders?country=DE&limit=1   ->  200, items: [ 1 ],  ignored_filters: []
```

A caller cannot distinguish **"there are no tenders in that country"** from
**"that value can never match anything this API stores"**. Both are a plausible,
successful, empty answer.

Who it bites is not hypothetical: issue 48 existed because `/docs` used to
document alpha-3 (`DEU`) and callers used it. The docs are fixed, but anyone
working from an old integration, a cached page, or the general habit of alpha-3
gets silence rather than a correction.

## `ignored_filters` is the wrong vehicle — do not reach for it

Its contract is in `Collection::honoured_params`:

> A parameter outside this set is accepted and then changes nothing — an
> unfiltered answer that looks filtered […] The list handler diffs the request's
> parameters against this set and echoes the leftovers as `ignored_filters` so the
> response says what it did (issue 118).

So it names **parameter names the collection does not honour at all**. `country`
*is* honoured by Tenders — the filter really was applied, and `DEU` really matched
nothing. Adding an unmatchable *value* to that list would make it lie about what
applied, and `honoured_params_match_the_emitted_sql` exists specifically to keep
it honest by byte-comparing the emitted SQL with and without each parameter.

Any fix here needs its own field, not this one.

## What is NOT known

1. **Is this worth an API change at all?** A REST collection returning an empty
   list for a filter that matches nothing is conventional, not broken. The case
   for acting is the specific alpha-3 legacy, which is a shrinking population.
2. **What is the right shape?** Candidates, none evaluated: a separate
   `unmatched_filters` field; validating `country` against a known-values list and
   returning `400`; or a `warnings` array. A `400` is the loudest and also the most
   likely to break a working client that passes a country with no current tenders.
3. **How many callers actually do this?** nginx access logs would say. If the
   answer is "nobody since the docs were fixed", the honest outcome is to close
   this as not-worth-it — the same shape issues 327, 331 and 332 landed on.

## Suggested first step

Read the nginx access log for `country=` values that are not two characters, over
whatever window is retained. That is a bounded metadata read and it decides the
issue: a real caller population justifies designing a field, and no callers means
close it. **Do not design the field first.**

## Measured: the caller population is empty

`suggested first step` said read the access log before designing anything. Done —
15 retained log files, 22 Aug to 1 Sep, aggregated (no raw lines, no addresses):

```
country=CY   111      country=MT     2
country=DE    67      country=AQ     1
country=LU    22      country=cy     3   (lowercase)
country=      13      country=DEU    2   <- mine, minutes ago
                      country=ZZ     1   <- mine, minutes ago
```

**Every real request used a valid two-character code.** The only alpha-3 hits in
ten days are the two I made while verifying issue 48. The legacy population this
issue was worried about does not exist in the traffic.

### The lowercase hits are not a finding either

`country=cy` appeared three times from a real caller, which looked like a
case-sensitivity wart worth chasing. It is not — checked directly:

```
country=CY -> 3 items    country=cy -> 3 items    country=Cy -> 3 items
```

The filter is already case-insensitive, so those requests were served correctly.

## Closing, and why that is the right answer

An empty list for a filter that matches nothing is conventional REST behaviour.
The only argument for changing it was a specific legacy caller population, and
that population measures zero. Designing an `unmatched_filters` field, or
returning `400` and risking breaking clients that legitimately query a country
with no current tenders, would be work for nobody.

Same shape as issues 327, 331 and 332: a mechanism that is real, an effect that is
not, and a negative recorded rather than a change shipped. Reopen if the traffic
ever says otherwise — the query above is the test.
