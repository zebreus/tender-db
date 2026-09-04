# 351 — country-less mentions mint one provisional row each: 92 identical `Stadt Burghausen` rows, and 68% of prominent entities' mentions on echo rows

Status: UNITS 1+2 BUILT 2026-09-04 07:4x — unit 1 (census) deployed `b7d0eba`, running as job 647 (~1,670 rows/s; report pending); unit 2 (prevention: country-less reuse of the standing `(name_norm, NULL)` row when the N2 key is under the wall, mint over it, counters in the resolver's diag line; test `country_less_mentions_reuse_under_the_wall_and_mint_over_it`) gate running, deploys once the queue is idle. Was: UNIT 1 BUILT 2026-09-04 07:0x (gate running) — `provisional-echo-census`: a keyset walk of the `(name_norm, id)` index over provisional NULL-country identifier-less rows, one group in memory at a time, top-200 by rows with mentions and the wall's verdict; tests `provisional_echo_census.rs`. Then: deploy, run, record; units 2 (prevention) and 3 (repair) follow. Was: ready-for-agent (filed 2026-09-04 from issue 350's measurement)
Kind: data quality / identity (organization layer) — prevention + repair, the 234 shape for the country-less half
Relates to: 234 (closed the `(name_norm, country)` half; left "nameless or country-less mentions mint fresh rows"), 350 (the measurement), 349 (why the wall reads these entities as generic), 300 Stage 3 (R3 rescues NULL-country rows WITH identifiers; these have none)

## Observed

Over 168 prominent entities (issue 350's echo keys): 7,856 of 9,003 carrier
rows are provisional and 7,384 of those are NULL-country; 44,340 mentions sit
on them against 21,064 on the identified rows. They are not spelling variants —
`Stadt Burghausen` × 92, `IBM Deutschland GmbH` × 83, `STADT MANNHEIM` × 59 —
one row per mention, because the post-234 provisional path reuses
`(name_norm, country)` only, and a country-less mention has no country to
match on.

## Units

1. **Census** (read-only, one PK walk): provisional NULL-country rows by
   `name_norm` → groups ≥ 2, rows, mentions, top 200 groups, per-name-key
   genericness (the 348 probe's breakdown) so unit 2's gate can be sized.
2. **Prevention**: `resolve_one_mention`'s provisional path looks up
   `(name_norm, NULL)` for a country-less mention and reuses the row when the
   N2 key is under the wall (`name_key_is_generic_on`, the fold's own
   connection, lenient on error as the corroboration probe is). Over the wall
   → mint as today. Test: two country-less "Stadt Burghausen" mentions share a
   row; two country-less "Gemeinde Taufkirchen" mentions do not once the key is
   over the cap.
3. **Repair**: fold standing identical-`name_norm` NULL-country provisional
   rows into the lowest id per name, repointing mentions/parties/bid-parties
   through `repoint_org_references`, ledger rule `p0`, dry/wet with parity;
   gated by the same wall so the Taufkirchen shape is left standing.

## Done when

- the census is on this issue with corpus-wide numbers;
- prevention deployed and the next census shows the class not growing;
- the repair's dry plan reviewed (30-sample), wet run, and the E0 dry plan's
  `admitted_echo` drops as the keys fall back under the wall.
