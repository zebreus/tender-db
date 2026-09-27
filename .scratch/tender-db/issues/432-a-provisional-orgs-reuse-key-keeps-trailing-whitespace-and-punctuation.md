# 432 — a provisional organization's reuse key keeps trailing whitespace and punctuation, so one identifier-less buyer mints two rows

Status: ready-for-agent — filed 2026-09-27 from issue 386's close-out: the three FTS Tenders the refined weld gauge
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
