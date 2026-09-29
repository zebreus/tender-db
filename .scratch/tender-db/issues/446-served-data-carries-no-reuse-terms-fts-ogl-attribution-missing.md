# 446 — the served data carries no reuse terms: FTS's OGL statement and TED's attribution are missing

Status: in-progress — filed and built 2026-09-29 (hourly check-in, while writing issue 342's step-12 docs).
Kind: compliance (licence conditions on the data we republish), small
Relates to: 342 (FTS, the source whose licence ends without the statement), docs/research/SUMMARY.md D11 (the
decision this completes), docs/research/uk-fts.md §1, docs/research/ted-access-channels.md §7,
docs/research/german-portals.md §7

## What is wrong

tender-db republishes notices from three publishers and credited none of them. `/v1` said `"license":
"AGPL-3.0-or-later"`, which is the code's licence, and `/docs`, the dashboard footer and the OpenAPI document said
nothing about the data. The research decision D11 ("TED attribution + 'data transformed' notice (footer + API `source`
field)") shipped only its AGPL half.

- **FTS** (live since 2026-09-07; 14,235 notices served on 09-27): the Open Government Licence v3.0 grants reuse on
  the condition that the re-user "acknowledge the source of the Information in your product or application by
  including or linking to any attribution statement". Absent a specified statement, the statement is "Contains
  public sector information licensed under the Open Government Licence v3.0.". The licence also says that "If you
  fail to comply with them the rights granted to you under this licence ... will end automatically"
  (uk-fts.md:24–25). So FTS data has been served for three weeks without the condition its grant depends on. The
  statement sat only in a Rust doc comment (`crates/ingest/src/fts/mod.rs`) and inside each archived member.
- **TED**: free reuse under Commission Decision 2011/833/EU, with the obligation to "attribute TED/Publications Office
  as source and indicate that data has been transformed" (ted-access-channels.md §7).
- **DÖE**: CC0, so nothing is owed. The Open-Data-Richtlinie's liability note (content responsibility stays with
  the publishing authority) was recommended for mirroring (german-portals.md §7).

## Fix (built 2026-09-29)

One table, `tender_db::v1::DATA_SOURCES`, holds per source the name, url, licence, licence url and the attribution
statement verbatim. It is served three ways:

- `/v1` → `data_sources[]` (and the OpenAPI schema documents it);
- `/docs#data-sources`, a new section repeating each statement and licence link word for word, linked from the
  nav and the footer;
- the dashboard footer links `/docs#data-sources`. The OGL is satisfied by "including **or linking to**" the
  statement.

`ingest::fetch::SOURCES` is now the one list of archived sources (`register_archive` walks it). The `/docs` test
`every_archived_source_carries_its_reuse_terms_on_the_docs` fails when a source is added there without terms, or when
`/docs` drifts from the table. `tests/api.rs` pins `/v1`'s list to the registry and the FTS statement to the OGL's
own wording.

## Verify

```sh
curl -s https://tenders.zebreus.click/v1 | python3 -c "import sys,json; d=json.load(sys.stdin); print([ (s['source'], s['attribution'][:40]) for s in d['data_sources']])"
curl -s https://tenders.zebreus.click/docs | grep -c 'Contains public sector information licensed under the Open Government Licence v3.0.'
```

Expect three entries (ted, doe, fts) and a count ≥ 1.
