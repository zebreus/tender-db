# 357 — The identifier-under-several-codes residue, case by case through the verdict path

Status: SLICE 1 APPLIED 2026-09-05 11:5x UTC — 590 verdicts recorded under
`cluster-country-2026-09-05`; dry plan (job 702) **488 moves, exact against the recorded highs, 0
no-ops**; wet (job 703) **488 rows moved**; R2 dry/wet (jobs 704/705) **316 groups merged, 394
org rows removed, 1,395 mentions, 2,254 parties, 926 bid-parties, 9,437 winners repointed, 833
tenders touched**; `project` (706) behind it. Slice 2 RUNNING (packet job 707: 3,519 clusters, 2,441 reviewable, 600 carried, of which 222
are slice-1 clusters that persist because their strangers were parked or kept on purpose —
excluded rather than re-litigated; the **378 new** clusters run in 16 batches + 2 blind sample
batches). Packet job
deployed `3be3f45`; packet run 701: 3,897 clusters, 2,819 reviewable.
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

## Slice 1 reviewed (2026-09-05, 49 agents, ~4.8M tokens, 2h38m)

600/600 cases. Verdicts: wrong-country 485 (no-one-letter-pair/spray 396 of 493), distinct-entities
73, same-entity-two-registrations 27, needs-more-evidence 15. 590 moves (580 high): weight 402,
arithmetic 158, national-format 27, name-language 3. Challenger: agreed on 569 cases, disputed 30
moves (strangers with real standing — Lithuanian firms under LV with 6–22 mentions; an Italian
trade agency's German office; a Belgian development agency's country offices), flagged 1 missed
move. Blind readers (60 cases): 57 same verdict, 58 same HIGH move set. Floor parked 66 highs
(35 mover-not-a-stray, 26 shared-register, 5 name-says-foreign-filing). **Apply set: 488 HIGH**
(weight 333, arithmetic 134, national-format 21): VU→BG 59, VA→BG 45, VN→BG 26, VE→BG 25,
AD→CZ 23, GA→BG 18, SE→FI 17, NO→DK 13, EE→ES 10 … — the Bulgarian spray class (ministry
directorates and municipalities with thousands of mentions beside single-mention copies tagged
VU/VA/VE/VN/GA/GH/IO/DE/FR) is most of it, read case by case and holding. Record:
`357-cluster-country-verdicts-slice1.json`.

## Not in scope

Merging the standing duplicate identities a move creates: R2's arm, run after the wet apply
as in 355. The `too-short` and `footprint-excluded` classes.
