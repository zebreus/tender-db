# 348 — the genericness wall is unobservable: no way to ask "why is this name key generic?"

Status: DONE 2026-09-04 — deployed `aae8dd6` (with 347); first use settled issue 346 in one pass (155 keys, ~1 s each through `admin.sh raw GET "/admin/name-key?name=<urlencoded>&show=30"`) and surfaced issue 349. Was: BUILT 2026-09-04 (same firing it was filed; gate running) — `GET /admin/name-key?name=…` returns the N2/N3 keys, the cap, and per kind the distinct-carrier count (bounded at 1,000) plus the first carriers' org rows. Then: deploy, and use it to settle issue 346's open question.
Kind: operability (organization layer) — small
Relates to: 346 (the question that filed this), 316/318 (the wall), 331/332 (its statistic), 347 (the other blind spot found the same morning)

## Observed

After the Greek tonos fold (issue 346, `d37e9bf`) and the key rebuild (job 641),
the census moved 22 GR:national groups out of `disagree` — but GR
`agree-generic` went 4 → 22 and `agree-distinctive` 134 → 133, and the E0 plan
shrank by one. Two readings fit the tally equally well: the 17 folded groups
now read `agree-generic` (their merged casings crossed the 20-carrier wall), or
they read `agree-distinctive` while 18 OTHER big-municipality keys crossed the
wall once both casings counted together. Which one is true decides whether the
fold helped or hurt — and **nothing on the box can answer it**:

- `org_match_keys` is outside `/v1/sql`'s public surface (correct: it is a
  scratch satellite);
- the census listing did not show GR at all (issue 347) and lists verdicts, not
  carrier counts;
- the proxy through `organizations.name IN (both spellings)` gave ≤ 8 rows per
  key, which says the wall should NOT fire — but the wall counts head + satellite
  names by org id, and the proxy cannot see satellites.

The wall decides what R3 rescues, what E0 folds and what the resolver's
prevention hook refuses. A decision that important needs a window.

## Built

`GET /admin/name-key?name=<name>[&show=N]` (operator secret): `n2`, `n3`,
`stoplist_cap`, and under `kinds` — in the wall's own probe order, `n3` then
`n2`, skipping an N3 that equals its N2 — the key, `carriers` (distinct org
rows, counted up to 1,000), `generic`, and the first `show` (default 20, max
100) carrier rows as `(org_id, country, identifier_kind, identifier, name)`.
Store side: `Db::name_key_carriers(kind, key, limit, show)` — the wall's own
`GENERIC_KEY_SQL` seek plus one `DISTINCT org_id … LIMIT show` on the same
index. Read-only, bounded.

## Done when

- deployed, and `admin.sh raw GET "/admin/name-key?name=…"` answers for
  `Δήμος Αβδήρων` and `Δήμος Χανίων`;
- issue 346's tally is explained from carrier rows, not inferred.

## First use (05:3x UTC)

All 155 agreeing GR:national groups probed; the tally that filed this
(`agree-generic` 4 → 22) decomposed into 15 folded-distinctive / 2
folded-generic / 118 uniform-distinctive / 20 uniform-generic, with carrier rows
that show WHY each generic key is generic — recorded on 346 and 349. The
endpoint is what made the difference between a guess and an answer; leave it.
One rough edge: `admin.sh raw` pipes through `jq .`, so batch callers want
`| jq -c .` to get one line per key.
