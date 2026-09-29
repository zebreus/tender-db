# 432 — a provisional organization's reuse key keeps trailing whitespace and punctuation, so one identifier-less buyer mints two rows

Status: done — 2026-09-29. Fix `0797820` deployed (ancestor of prod `92ebde0`); `repair-provisional-name-norm` wet 1613 folded 42,547 groups / 48,863 rows (residual 0); the country-less half folded by `fold-provisional-echoes` wet 1652 (9,728 groups / 17,139 rows, after dry 1649 planned the same). The three tenders now carry ONE buyer organization across both versions each (bounded `/v1/sql`, below). The weekly DQ line re-reads on the next Sunday run.
Was status (before 2026-09-29): ready-for-agent — filed 2026-09-27 from issue 386's close-out: the three FTS Tenders the refined weld gauge
still counts are this defect, not welds.
Kind: defect (org layer — the provisional reuse key in `crates/store/src/canonical.rs`)
Relates to: 386 (the FTS weld gauge reads 3 because of this and should read 0 once it is fixed), 234 (the
identifier-less mention policy), 351 (the country-less half, which folds identical `name_norm` rows but not
near-identical ones), 300 (N2 `match_norm`, the aggressive key this one deliberately is not), 329 (E0 merges
same-triple duplicates, which are identifier-keyed, so it cannot reach identifier-less twins)

## Observed (2026-09-27, `/v1/sql`)

`tender_version_parties` × `organizations` for the three tenders the refined FTS weld gauge counts:

| tender | seq 1 buyer | seq 2 buyer |
|---|---|---|
| 7957971 | 30915165 `name_norm` `'scotrail trains limited .'` | 30914231 `'scotrail trains limited'` |
| 8579928 | 30972111 `'procurement for housing '` | 1880052 `'procurement for housing'` |
| 8579929 | 1880052 `'procurement for housing'` | 30972111 `'procurement for housing '` |

All four rows are `provisional = 1`, `country = 'GB'`, no identifier. Each pair is one buyer. On 8579929 both
releases point at the SAME FTS organization section (`ORG-GB-FTS-161789`); only the published name's trailing
space differs.

## Mechanism

The provisional reuse (issue 234's reuse policy) keys on `(name_norm, country)`, with
`name_norm = m.name.to_lowercase()` (`canonical.rs`, the resolver's identifier-less arm, and the same expression in
the second mint path near line 19999). Lower-casing is the ONLY step: no trim, no whitespace collapse, no
trailing-punctuation strip. So a publisher's trailing space or ` .` mints a second row. The tender layer is
unaffected: the key election falls back to N2 `match_norm`, which folds both, so the three tenders are correctly ONE
Tender each. Only the org layer splits.

## Size, bounded

One 100k-id band of the FTS mint era (ids 30,900,000–31,000,000, `country = 'GB'`, identifier-less): **3,092
rows, 42 with an untrimmed `name_norm`, 23 of which have a trimmed twin already standing**. So about 0.7 % of
identifier-less GB rows are twins in that band. The corpus-wide count (`name_norm <> trim(name_norm)` over all of
`organizations`) exceeds `/v1/sql`'s 10 s cap, so the fix's own dry run does the corpus sizing. The ` .` shape
(7957971) is not in the trim count and needs its own term.

## What to build

1. A light canonical form for the reuse key: trim, collapse internal whitespace runs, and strip trailing
   `.`/`,`/`;` plus the space before them. It stays far short of N2 (no legal-form stripping, no transliteration),
   because issue 351's genericness wall exists to keep N2 from over-merging; this is spelling noise, not identity.
   Apply it at both mint paths, and to `name_norm` itself so the row and the key agree.
2. `repair-provisional-name-norm` (dry by default): re-derive `name_norm` for identifier-less rows. Where the new
   key collides with a standing row in the same country, fold the twin through the existing merge arms (ledger rule,
   parity on mentions). Size it dry first.
3. A store test: `'Procurement for Housing'` and `'Procurement for Housing '` bind one row, and so do
   `'ScotRail Trains Limited'` and `'ScotRail Trains Limited .'`. Decided now: `'ACME Ltd'` and `'ACME Ltd.'` fold
   too. A trailing period is spelling, and the legal form itself is untouched. The control is
   `'ACME Ltd'` vs `'ACME'`, which stay two rows (legal-form stripping is N2's job, behind the wall).

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh /admin/reports/data-quality" | python3 -c "import sys,json; d=json.load(sys.stdin); b=d['body']; l=[x.strip() for x in b.split(chr(10)) if 'FTS Tenders whose versions disagree on the buyer' in x]; print(d['computed_at'], l[0][:80] if l else 'no line')"

- **done**: `… disagree on the buyer: 0` on a report computed after the repair (the three twins merged)
- **open**: `3` (tenders 7957971, 8579928, 8579929; read 2026-09-27 through `/v1/sql`; the stored weekly report predates
  the refined line)
- **read 2026-09-29 11:2x UTC, after wet 1613 and 1652**, the gauge's three tenders directly (bounded `/v1/sql`):
  `SELECT tender_id, COUNT(DISTINCT seq), COUNT(DISTINCT organization_id) FROM tender_version_parties WHERE tender_id
  IN (7957971, 8579928, 8579929) AND role LIKE '%Buyer' GROUP BY tender_id` → `[7957971,2,1]`, `[8579928,2,1]`,
  `[8579929,2,1]`: two versions, one buyer each (ScotRail Trains Limited 30914231; Procurement for Housing 1880052).
  **Done** on the rows; the report line itself is recomputed with the Sunday data-quality run.

## Built (2026-09-27)

Uncommitted, not deployed, not run on prod. Gate (`ops/check.sh`) not yet run; focused suites below are green.

**The key.** `store::org_name_norm(name)` (`crates/store/src/canonical.rs`, beside `register_jurisdiction`): Unicode
`to_lowercase`, whitespace trimmed and every internal run (tab, newline, NBSP included) collapsed to one space, then
trailing `.`/`,`/`;` stripped together with the whitespace before them, repeatedly. Nothing else: no legal-form
stripping, no transliteration, no accent folding. `ACME Ltd.` = `ACME Ltd`; `ACME Ltd` ≠ `ACME`. A name that is ALL
trailing punctuation keeps its trimmed/collapsed form (`"."` stays `"."`, not `""`, so it does not silently join the
nameless class); a blank or whitespace-only name keys `""` (so it is no longer reused per country — the nameless
policy).

**Where it is used** (every derivation of `organizations.name_norm`, and the keys that must agree with it): the
identifier-bearing mint and the 234 reuse arm in `resolve_one_mention` (so the country-less 351 arm keys on it too),
the placeholder dissolve's re-resolve AND its tier-3 "all mentions share one key" check, `backfill_org_name_norm`
(217-B), and `POST /admin/name-verdicts` (the verdict key, was `trim().to_lowercase()`). Left as bare lowercase on
purpose: `organization_names.name_norm` (the satellite; not a reuse key). **Review addition:** the `/v1`
`name_prefix` is now normalised by `org_name_norm` too (`v1/mod.rs`). Left as bare lowercase, `ACME Ltd.` would stop
finding the row re-keyed `acme ltd` — a prefix longer than the stored key. Stripping the prefix's trailing period only
widens the match (`acme ltd` also prefixes `acme ltd. group` and `acme ltda`), which a prefix search already does
for anyone who types no period. `/docs` says so; `organizations_can_be_searched_by_name_prefix` asserts a noisy spelling
of a real name (`doubled  space  .`) finds it.

**The repair.** `repair-provisional-name-norm` (dry default, stoppable, `max_groups` caps a wet run) →
`Db::repair_provisional_name_norm`. Walks identifier-less rows by PK; a row whose stored key ≠ `org_name_norm(name)`
is re-normalised; with a country and a non-empty key it is hashed on `(key, country)` and the resolver's own
`(name_norm, country)` probe finds rows already on that key. Candidate groups are re-read and confirmed (exact key,
same country, identifier-less), so a hash collision costs a read, never a merge. Groups fold through the echo
fold's own loop (`fold_provisional_plan`, now parameterised by `ProvisionalFoldRule`) under ledger rule **`p1`**,
keep = lowest id, still provisional, its key rewritten inside the fold's transaction; full repoint (mentions,
parties, bid parties, winners + winner dups, `organization_names`). No wall (234's reuse merges these without one);
country-less rows are re-keyed, never folded here — run `fold-provisional-echoes` after. `org_name_verdicts` keys
are re-keyed; a key already taken is left standing and listed (`verdict_conflicts`). Then the bulk `name_norm`
rewrite (uncapped runs only) in 20k-row slices with a TRUNCATE checkpoint between. Dry stores
`provisional-name-norm-plan` (`groups`, `rows`, counts, top-200 listing, 100-row sample); wet aborts outside
max(2%, 50) on either count and re-records the residual (the echo fold's rule).

**Tests** (`crates/store/tests/provisional_name_norm.rs`, all ok):
`the_key_folds_whitespace_and_trailing_punctuation_and_nothing_else`,
`the_fts_spellings_of_one_identifierless_buyer_bind_one_row`,
`the_repair_rekeys_the_stock_and_folds_the_same_country_twins`. Also green: store `provisional_echo_fold`,
`placeholder_dissolve`, `resolver_prevention`, `org_name_search`, `provisional_echo_census`, `anchor_wall`, the
store lib's org/name/mention/merge tests, all of `cargo test -p ingest`, and `cargo test -p tender-db --features
server --lib` (147/147 — after boxing the store future inside the new arm: with only the arm's outer `Box::pin`,
`an_execute_without_an_expected_count_is_refused` overflowed its stack, CLAUDE.md's run_spec trap).

**Run order on prod — decided at review: AFTER the text-era and r208/r209 re-parses and their full fold** (issue
393/397's campaign, queued 2026-09-27). That fold re-resolves the mentions of ~11M re-parsed notices; under the new key
every `… Ltd.`-shaped name whose stock row still carries the old key would miss the probe and mint a fresh twin,
turning a bounded repair into a corpus-scale one. So the chain runs on the old key, then this deploys, then dry →
wet promptly: until the repair runs, a new mention of a buyer whose stock
row still carries the old key misses the probe and mints ONE fresh row on the corrected key (then reuses it); the
repair folds those too, but the dry/wet parity tolerates only max(2%, 50) of drift between the two runs.

## 2026-09-28: the repair ran (dry 1601 → re-dry 1612 → wet 1613), ahead of the full fold

The run-order note above assumed the chain would fold on the OLD key and this would deploy after. It deployed with
the 434–440 bundle instead (`9dedf49`), so the fold that began at the 07:35 tick (1610) was resolving mentions on
the NEW key against ~464k stock rows still on the old one. The owner cancelled it in planning (see 434's 2026-09-28
note: 505,116 orgs minted by then) and ran this repair first.

- **Dry 1601 (07:04):** 5,714,068 identifier-less rows walked, 464,327 off the corrected key (208,028
  country-less), 34,503 collision groups over 75,051 rows, a fold removes 40,548; 5,664 verdict keys to re-key, 937
  on a conflict.
- **Re-dry 1612 (after 1610's partial planning):** 6,219,184 walked. The same 464,327 are off-key, which confirms
  the new mints sit on the corrected key. Groups 42,547 over 91,410 rows, fold removes 48,863: about 8.3k of the
  505k new rows were twins of old-key stock.
- **Wet 1613:** folded 42,547 groups under `p1`, **48,863 rows removed**, 394,404 mentions / 53,136 parties / 3,704
  bid-parties / 268,084 winners repointed, 162 winner dups deleted, 81,356 tenders touched; 5,664 verdict keys
  moved (937 left standing on a conflict); 413,126 `name_norm` rewritten; **residual 0**.

The Verify (the refined weld gauge's 3 FTS Tenders) is read after the full fold (job 1616) and the next weekly report.
