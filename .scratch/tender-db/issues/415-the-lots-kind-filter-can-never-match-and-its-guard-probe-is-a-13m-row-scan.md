# 415 — `/v1/lots?kind=` can never match (the parameter is lowercased, the stored lot kinds are `Lot`/`LotsGroup`/`Part`), and the guard's lots `kind` probe is a bare scan of `tender_version_lots` that walks to the 30 s deadline

Status: ready-for-agent — found 2026-09-18 09:0xZ while pinning issue 408 (b)'s handler test: the lots leg of a bounded walk on `?kind=` matched nothing on the fixture, and the same probe on prod is a 503.
Kind: defect (API contract + performance/availability — the documented `kind` filter on `/v1/lots` returns an empty page for every spelling, and the absent-value guard leg that answers it is the expensive one, not the cheap one)
Relates to: 408 (the bounded fallback walk — this walk runs BEFORE it, inside `reachable()`, so option (b) does not shorten it), 219/120 (the isolated pool and the absent-value guard: "cheap index seeks first, the bare table scans last" — the lots `kind` leg is the bare scan, and there is nothing cheaper in front of it), 371 (the guard set and the routed set are one list — they agree here; the leg is present, just slow), 217-B (the `kind` lowercasing was added so the documented `kind=VAT` identifier lookup matched — lowercase-only vocabularies on organizations and tenders), 118 (`ignored_filters` — an empty page here reads as an applied filter with no matches, which is the wrong claim)
Blocked by: nothing

## Observed (2026-09-18, live, idle box)

    curl -s -o /dev/null -w '%{http_code} %{time_total}\n' 'https://tenders.zebreus.click/v1/lots?kind=lot&limit=1'
    → 503 30.66 s
    curl -s -o /dev/null -w '%{http_code} %{time_total}\n' 'https://tenders.zebreus.click/v1/lots?kind=Lot&limit=1'
    → 200 3.40 s   items: []  more: false  ignored_filters: []
    curl -s -o /dev/null -w '%{http_code} %{time_total}\n' 'https://tenders.zebreus.click/v1/lots?kind=LotsGroup&limit=1'
    → 200 2.99 s   items: []
    curl -s 'https://tenders.zebreus.click/v1/lots?limit=3' → [(2, "Lot"), (3, "Lot"), (4, "Lot")]

Every lot the list echoes carries `kind: "Lot"`, and no spelling of it filters to a non-empty page.

## Mechanism, from the source

1. **The vocabulary.** `Params::filter` (`crates/app/src/v1/mod.rs`, the `kind` line) lowercases the
   parameter for every collection: `kind: self.kind.as_deref().map(|k| k.trim().to_ascii_lowercase())`.
   The comment says why — the organizations identifier scheme (`vat`/`national`) and the tender kinds
   (`procedure`, …) are lowercase-only, so `kind=VAT` matched nothing before. Lot kinds are the eForms
   node kinds, stored capitalised (`Lot`, `LotsGroup`, `Part` — `notice_sections.kind` carried into
   `tender_version_lots.kind`), and the predicate is `vl.kind = ?`, case-sensitive. So `Lot` → `lot`
   → nothing, for every caller, since 217-B landed.
2. **The guard leg.** `reachable()`'s `Isolated::Kind` arm for Lots calls
   `kind_reachable(conn, "tender_version_lots", kind)` = `SELECT 1 FROM tender_version_lots WHERE
   kind = ? LIMIT 1`. No index carries `kind`, so for an ABSENT value — which, per (1), is every
   value — the probe scans all ~13.2M rows to find none: 30.66 s cold (the 503), 3.4 s warm (the 200
   with nothing). The guard's own ordering comment ("the bare table scans last") knows this leg is a
   scan; on lots it is the ONLY leg for `kind`, so nothing cheaper ever short-circuits it.
3. **Issue 408 (b) does not reach it.** The bounded walk bounds the page query; `reachable()` runs
   before the page query is built. So after (b) deploys, `?kind=lot` on lots is still one 30 s scan
   in the guard, then an empty band page.

## Why it matters

A documented filter (`/docs`: "Tender/Lot kind flag"; `/v1/lots` lists `kind` among the honoured
params) returns `items: []` with `ignored_filters: []` for every value — the API claims it applied
the filter and found nothing, which is the "applied filter with an empty result" misreport 217-B
fixed on organizations. And the cheapest way to hold an isolated slot for 30 s is now
`/v1/lots?kind=anything` — issue 219's class, reachable by typing the documented example.

## Repro

1. The three curls above.
2. `grep -n "to_ascii_lowercase" crates/app/src/v1/mod.rs` → the `kind` line.
3. `grep -n "fn kind_reachable" -A 3 crates/store/src/read.rs` → the bare scan.

## Verify

    curl -s -o /dev/null -w '%{http_code} %{time_total}\n' --max-time 40 'https://tenders.zebreus.click/v1/lots?kind=Lot&limit=1'; curl -s --max-time 40 'https://tenders.zebreus.click/v1/lots?kind=Lot&limit=1' | python3 -c "import sys,json; print(len(json.load(sys.stdin)['items']))"

- **done**: `200 <well under 30 s>` and `1` — the documented lot kind filters to a lot, cheaply
- **open**: `200 …` with `0`, or `503 30.xx` (read 2026-09-18: `200 3.40` then `0`)

## Done when

- `?kind=` on `/v1/lots` matches the stored vocabulary: either the parameter is canonicalised per
  collection (`lot`/`Lot` → `Lot`, `lotsgroup` → `LotsGroup`, `part` → `Part`; unknown spellings 400,
  as `sort` does) or the lots predicate compares case-insensitively — the first is preferred, it keeps
  the predicate a plain equality and names the vocabulary in one place.
- The guard's lots `kind` leg is cheap: a presence set the projection maintains (the
  `tender_currency_presence` shape issue 371 built for the same reason — a few distinct values, no
  index worth paying write amplification for), or the leg is dropped for lots so the bounded walk
  (408 (b)) answers an absent value in one band instead of the guard answering it in one scan. Either
  way `?kind=<absent>` on lots costs seconds, not 30.
- A test drives `/v1/lots?kind=Lot` through the handler on the chain fixture and gets lots back, and
  408 (b)'s bounded-walk test gains the lots `kind` leg it had to skip.
- `/docs` names the lot kind vocabulary beside the tender one.
