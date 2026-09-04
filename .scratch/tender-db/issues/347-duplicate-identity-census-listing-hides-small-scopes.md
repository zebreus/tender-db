# 347 — the duplicate-identity census listing hides every small scope: size-then-alphabetical under a 400-row cap

Status: BUILT 2026-09-04 (same firing it was filed; gate running) — per-scope quota lane (`min(cap / scopes, 25)` rows per `country:kind`, in the listing's existing order; the reserve keeps the general lane from crowding late scopes), test `the_listing_shows_every_scope_under_the_cap`. Verdict-ordered quotas were NOT done: the verdict is only known once a group is judged, and the quota takes each scope's first rows in size-then-identifier order, which is its largest groups — good enough for the question that filed this. Was: ready-for-agent (filed 2026-09-04 while reading job 639's report for issue 346)
Kind: report shape (organization layer census) — small
Relates to: 329 (the census this listing belongs to), 346 (the question it could not answer), 335 (report history)

## Observed

`duplicate_identity_census` sorts the unkeyed groups by member count
descending, then by `(country, kind, identifier)` ascending, and lists the
first `CAP = 400` (`canonical.rs`, `unkeyed.sort_by`). With 3,666 groups of
which all but a few dozen are pairs, the listing is: the handful of size-3+
groups, then the size-2 groups in alphabetical triple order — `AE`, `AT`, `AX`,
`AZ`, `BF`, `BQ`, `CH`, `CY`, then `DE` until the cap. **`GR`, `LT`, `RO` and
everything after `DE` never appear**, however many groups they hold (GR:national
has 210). On 2026-09-04 the question "what shape are the Greek `disagree`
groups?" could not be answered from the report at all; it took a bounded SQL
read of the live table (issue 346's measurement).

`verdicts_by_scope` gives the counts per scope, so the report knows the scopes
exist; it just cannot show one example of any of them.

## Proposal

Keep the cap, change what fills it: a per-scope quota — up to `min(cap / scopes,
25)` rows per `country:kind` scope in verdict order (disagree, contained,
agree-generic, agree-distinctive, unnamed), the remainder of the cap by the
existing size-then-alphabetical rule. Every scope with groups then shows at least
a few rows, and the largest groups still lead. Pass 4's mention counts stay
listed-rows-only, so the run time does not move.

Test: seed three scopes (one with 500 pairs, two with 3) under a cap of 20 and
assert every scope is represented.

## Done when

- the listing shows rows from every scope in `verdicts_by_scope`;
- the next Sunday census report has GR:national rows in it.
