# 357 — The identifier-under-several-codes residue, case by case through the verdict path

Status: PACKET JOB BUILT 2026-09-05 (gate running) — `country-cluster-packet` (cap via
`max_groups`, default 600): the cluster census's residue with every member row's org id and
the issue-314 per-member evidence, heaviest first. Next: deploy, run, size the classes, then
the review campaign in the issue-355 shape, and apply through `apply-country-verdicts`.
Kind: data quality (organization layer) — campaign
Relates to: 326 (the census and the predicate that decided what it could), 355 (the
verdict path and the campaign shape), 314 (the per-member evidence), 311 (the principle)

## The residue

Issue 326's census groups every identifier standing under two or more country codes: 4,303
clusters on 2026-09-01. Its predicate moved 312 rows where the arithmetic named exactly one
survivor and the mistyped code was one letter off, and left the rest on purpose:
`no-one-letter-pair` (the codes differ by more than a letter — a Slovak IČO under `AD`, a
Norwegian orgnr under `LT`), `nobody-asked` (no scheme of the value's shape for any code in
the cluster — DE and ES registers carry no checksum), `anchor-names-several`,
`asked-and-refused`, and the stranger codes still standing in `anchor-names-one` clusters
(`GA`, `VA`, `VU` beside a Bulgarian EIK). Two classes are excluded for good: `too-short`
(a two-digit value is not an identity) and `footprint-excluded` (an embassy is one entity
filed from everywhere it operates).

This is exactly Lennart's steer in issue 311: the rule is the deny-direction floor; what it
cannot settle gets individual review with the full evidence. The 355 campaign showed the
shape works on the same-name cross-border cut (487 cases, 153 moves, 88/99 agreement with an
independent earlier review); this is the wider cut, keyed by identifier rather than by name.

## Built: `country-cluster-packet`

`Db::country_cluster_packet` runs the census uncapped, drops the two excluded classes,
orders the rest by total mentions (the reviewer's first read and the campaign's budget), takes
`cases_cap`, and for each cluster lists the member rows by one identity-index seek per
(code, kind) — the org ids a country verdict needs — each with the per-member evidence the
314 packet established (`anchors`, `country_probed`/`country_agrees`, mentions, name
variants, a few publication ids). The census's own reading (`asked`, `named`,
`named_schemes`, `one_letter_pair`, `heavy_one_letter`, mentions per code) travels with the
case. Stored as report `country-cluster-packet`; stoppable. Two store tests: exclusions,
member ids and evidence, heaviest-first order, the cap truncating the listing and not the
tally.

## The campaign (next)

Rubric v3 with one addition for clusters: a spray shape (one heavy code, several one-mention
strangers sharing its number) is the 326 census's own signature and the strangers move to the
heavy code — unless the name says the row is a foreign branch or mission. Stratify by census
verdict and by shape (spray vs balanced); the 355 floor applies (no agreeing row moves, no
weight-only moves, shared-checksum and shared-register pairs park). Two concurrent agents on
this box: size the first slice to what finishes in a firing.

## Not in scope

Merging the standing duplicate identities a move creates: R2's arm, run after the wet apply
as in 355. The `too-short` and `footprint-excluded` classes.
