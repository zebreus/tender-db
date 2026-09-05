# 357 — The identifier-under-several-codes residue, case by case through the verdict path

Status: CAMPAIGN SLICE 1 RUNNING 2026-09-05 09:3x UTC — packet job deployed (`3be3f45`) and run
(job 701, 4 s): **3,897 clusters, 2,819 reviewable** (no-one-letter-pair 2,580, nobody-asked
148, anchor-names-one 66, anchor-names-several 18, asked-and-refused 7); the 600 heaviest
carried (heaviest member 45–645+ mentions by quartile; 503 pairs, 97 triples or more). Split
by census class × shape — **spray** (heaviest member ≥ 10× the next: 573) vs **balanced** (27)
— into 23 batches of ≤ 35 plus 3 blind sample batches (60 cases); rubric v3c (`357-rubric.md`:
v3 plus the census fields and the spray rule), fast model for reviewer + challenger, session
model for the blind readers. Then: floors (355's plus the spray checks), POST under cohort
`cluster-country-2026-09-05`, dry → wet → R2 dry/wet → project; slices 2–5 (the remaining
2,219) at 600 per firing.
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
