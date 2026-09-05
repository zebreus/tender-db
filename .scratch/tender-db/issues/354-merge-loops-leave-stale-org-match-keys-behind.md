# 354 — the merge loops leave the deleted rows' `org_match_keys` behind, so the wall over-counts until the weekly rebuild

Status: BUILT 2026-09-05 03:3x UTC (gate running) — the two carrier counts (`GENERIC_KEY_SQL`, `NAME_KEY_CARRIERS_SQL`) now JOIN `organizations`, so a merged-away row's key stops counting the moment the row is gone; `provisional_echo_fold.rs` pins a phantom key that must not count and `stadt big` reading under the wall after its echoes fold while its 32 stale key rows remain. Deploys at the next idle window. Was: ready-for-agent (filed 2026-09-05 from issue 353's rebuild measurement)
Kind: correctness of a live gate (organization layer) — small
Relates to: 353 (the measurement), 351 (the provisional fold), 352 (foreign keys off in the merge loops), 329/300 (R2/E0/R3 use the same wall), the weekly tick (`build-org-match-keys`)

## Observed

Issue 353's campaign deleted 5.7M provisional rows with foreign keys off
(issue 352's bracket), so nothing cascaded and their `org_match_keys` rows
stayed. The wall (`NAME_KEY_CARRIERS_SQL`) counts `DISTINCT org_id` on
that table WITHOUT joining `organizations`, so every deleted row kept
counting as a carrier of its name key. Measured on 2026-09-05: a dry fold
before the rebuild (job 684) planned 0 groups with 15,260 names standing
over the wall; a wet `build-org-match-keys` (job 685, 50 s) and the same
dry fold after it (job 686) found **8,618 of those names under the wall**
— 17,555 rows that the stale keys alone had kept fragmented, folded by
job 687.

Between a fold and the Sunday tick the same over-count also steers the
LIVE resolver (issue 351 unit 2 mints a fresh row for a country-less
mention whose key reads over the wall) and the E0/R2 name denials.

## Built (the join, not the delete)

`org_match_keys` declares no foreign key and is indexed `(key_kind, key,
org_id)` only — there is no index leading on `org_id`, so `DELETE FROM
org_match_keys WHERE org_id = ?` per loser would scan the 6.6M-row table
each time (the docs at `NAME_KEY_CARRIERS_SQL` say why that index does not
exist). The cheaper and complete fix is on the reading side: both carrier
counts now JOIN `organizations`:

```sql
SELECT COUNT(*) FROM (SELECT DISTINCT k.org_id FROM org_match_keys k
   JOIN organizations o ON o.id = k.org_id
   WHERE k.key_kind = ? AND k.key = ? LIMIT ?)
```

Still bounded: the index seek stops after `LIMIT` LIVE carriers, each one
a PK lookup; a stale-heavy key walks its stale entries once per probe until
the weekly rebuild purges them (`build-org-match-keys` — note it defaults
to a DRY run; the wet form is `{"dry_run":false}`). The breakdown query
already joined, so the tiers and shapes were consistent with this reading
all along; only the raw count lagged.

Pinned in `provisional_echo_fold.rs`: `Gemeinde Generic` stays over the
wall with twelve LIVE carriers (three in the class, nine standing
non-provisional rows); a phantom key row for `stadt echo` (org 99999, no
row) does not count; after the wet fold `stadt big` reads `UnderWall`
(one country-less row + two identified) while its 32 stale key rows are
still in the table.

Deleting losers' keys inside the merge loops stays an option if an
`org_id` index is ever added (the rebuild would have to drop and recreate
it like `org_match_keys_kk`); today it is not needed.

## Not in scope

Cascading deletes with foreign keys on (issue 352 measured why they are
off), or changing the wall to join `organizations` (an extra join on a hot
path the resolver walks per mention; deleting the keys is cheaper and
keeps the counts honest by construction).
