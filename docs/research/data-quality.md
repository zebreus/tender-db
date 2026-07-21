# Data-quality report — first run (issue 27)

`crates/ingest/src/bin/data-quality` is the first *semantic* measurement of the
tender DB. Coverage (`bin/verify`) answers "how much did we import?" and
quarantine answers "what failed to parse?"; neither says how *complete* the data
we did import is. This tool measures that, per era, from the product's own
read-only `/v1/sql` endpoint.

## What it measures

Unit is the **tender-version** (≈ one per notice — a satellite fact is keyed
`(tender_id, seq)`, and each stored version already holds the *resolved* state at
that point, so supersession neither under- nor over-counts). Era is the **mapping
profile of the version's causing notice** — the same split the dashboard and
`Db::award_linkage` use.

1. **Field completeness** — share of versions carrying a title, buyer, value,
   CPV code, deadline, or a named winner. Each is a single satellite-driven scan
   (`FROM <satellite> … JOIN tender_versions`), so it stays bounded on the full
   archive.
2. **Award→notice linkage** — of award-bearing Tenders (any `lot_results`), the
   share that chained to a contract notice rather than standing as a
   single-notice island (mirrors the dashboard's unchained-award metric).
3. **Results materialisation** — of notices whose raw layer holds a `LotResult`
   section (a contract-award notice, era-agnostic: legacy award blocks
   synthesise the same section kind), the share that actually produced a
   canonical `lot_results` row. A shortfall is the issue-22 gap quantified.
4. **TED↔DÖE merge** — of the Tenders DÖE contributes to, the share that also
   carry a TED notice under the same BT-04 procedure key (ADR-0003, issue 12).

Design: the measurement logic lives in `ingest::data_quality` (transport-free,
unit-tested); the bin only moves rows over `/v1/sql`. A sibling binary, not a
`--report` mode on `verify` — `verify` is a pass/fail acceptance harness against
*external* ground truth with exit-code semantics; this is a descriptive,
self-referential measurement (rates/densities, no pass/fail), and folding it in
would muddy that contract. Both are validated end-to-end
(`crates/ingest/tests/data_quality.rs` drives the identical SQL against a scratch
DB projected from real fixtures).

## Status of this run — fixture validation, not the prod baseline

**The definitive baseline has not been taken yet.** Two blockers, both expected:

- **No prod API token.** `/v1/sql` is account-gated. An `acceptance-verify`
  account exists in prod, but its credentials are documented nowhere reachable
  (repo, `docs/operations.md`, the VPS `/opt/tender-db/`, shell history), the
  admin API mints only ingestion jobs (not account tokens), and minting a token
  needs the account password (login → `create_token`), which must not be
  guessed. The prior token (issue 15) was revoked. **Token gap reported to the
  team lead; the definitive run is `data-quality --token …` once a token is
  minted from the dashboard.**
- **Backfill in flight.** Prod data is partial (1993→~2011 + a few 2026 days),
  and the 33 GB DB is under active write (it refused even a read-only `sqlite3`
  open during the backfill). Full-archive numbers are only meaningful after the
  issue-15 backfill and the issue-22 results re-projection complete.

So the numbers below come from the **real fixture corpus** projected into a
scratch DB — they validate that the tool reads the schema correctly and computes
sane rates, and they already surface one real, scale-independent anomaly. They
are *not* the production baseline.

## Fixture-validation snapshot

```
tender-db data-quality — FIXTURE VALIDATION (not prod)
unit: tender-version (≈ one per notice); era = mapping profile of the version's notice

== 1. Field completeness (share of versions carrying each field) ==
  era                     versions   title  buyer  value    cpv deadline winner
  eForms-DE                      2  100.0% 100.0%  50.0% 100.0%    50.0%  50.0%
  DÖE sdk-0.1 island             2    0.0%   0.0%   0.0%   0.0%     0.0%   0.0%
  eForms EU                      5  100.0% 100.0%  80.0% 100.0%   100.0%  20.0%
  TED_EXPORT r2.0.8              2  100.0% 100.0%   0.0% 100.0%     0.0%   0.0%
  TED_EXPORT r2.0.9              3  100.0% 100.0%  33.3% 100.0%    66.7%  33.3%

== 2. Award→notice linkage (award Tenders chained to a contract notice) ==
  era                      awards  unchained   linked
  eForms-DE                     1          1     0.0%
  eForms EU                     1          0   100.0%
  TED_EXPORT r2.0.9             1          1     0.0%

== 3. Results materialisation (award notices → lot_results) ==
  era                  award-notices with lot_results  density
  eForms-DE                     1              1   100.0%
  eForms EU                     1              1   100.0%
  TED_EXPORT r2.0.9             1              1   100.0%

== 4. TED↔DÖE merge (of DÖE procedures, share also seen on TED) ==
  DÖE procedure Tenders: 2; merged with TED: 1 (50.0%)
```

(Merge shows 1 of 2 because one DÖE procedure has a TED twin — the merge case —
and one is a DÖE-only island; both are correct.)

## Anomalies called out

- **DÖE sdk-0.1 projects to empty canonical Tenders — 0 % on every field.**
  The strongest signal here, and scale-independent (identical on 2 fixtures or
  the full archive). sdk-0.1 parses richly at the notice layer but its `SDK01-*`
  field stems are absent from the projection's canonical mapping tables, so ~40 %
  of German volume yields contentless island Tenders. **Filed as issue 29.**
- **Results materialisation reads 100 % on fresh projection.** In these fixtures
  every award notice materialised its `lot_results` — so the projection path is
  correct; the prod "2,529 CANs, zero `lot_results`" (**issue 22**) is a
  deploy/re-projection state, not a code gap. This tool's section 3 is the metric
  that confirms issue 22 closed once the re-projection lands.
- **Low-confidence, to confirm at scale (not yet filed):** r2.0.8 shows 0 %
  value and 0 % deadline, and eForms-DE 50 % value/deadline — but at n=2 these
  are noise, not findings. The post-backfill run over millions of legacy notices
  will say whether the r2.0.8 estimated-value / `DATE_RECEIPT_TENDERS` mappings
  have a real gap or the fixtures are simply sparse.

## Re-run after the backfill

```
# mint a token from the dashboard for the acceptance-verify account, then:
TENDER_API_TOKEN=tdb_… data-quality                 # human report
TENDER_API_TOKEN=tdb_… data-quality --json > dq.json # machine
```

The full-archive report is the definitive baseline; harvest any new era-scale
anomalies it surfaces into fresh issues at that point.
