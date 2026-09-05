# 354 — the merge loops leave the deleted rows' `org_match_keys` behind, so the wall over-counts until the weekly rebuild

Status: ready-for-agent (filed 2026-09-05 from issue 353's rebuild measurement)
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

## Proposal

In every wet merge loop that deletes an organization row with foreign
keys off — `fold_provisional_plan` (p0), `match_org_identifiers_r2` (R2,
E0) and R3 through it — delete the loser's `org_match_keys` rows in the
same transaction, right before `DELETE FROM organizations`:

```sql
DELETE FROM org_match_keys WHERE org_id = ?
```

One indexed delete per loser (the table is keyed by `org_id`), so the
1,400 rows/s pace holds. The keep's keys stay; a rebuild is then only an
epoch change, not a correctness step. Pin it in `provisional_echo_fold.rs`
and `r2_merge.rs`: after the wet run the loser ids have no key rows and
the keep's are intact.

Also worth a line in `docs/operations.md`: `build-org-match-keys` defaults
to a DRY run (job 683 stored nothing); the wet form is
`{"dry_run":false}`.

## Not in scope

Cascading deletes with foreign keys on (issue 352 measured why they are
off), or changing the wall to join `organizations` (an extra join on a hot
path the resolver walks per mention; deleting the keys is cheaper and
keeps the counts honest by construction).
