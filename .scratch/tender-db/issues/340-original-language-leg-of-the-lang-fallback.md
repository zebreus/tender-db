# 340 — the `?lang=` fallback's "original language" leg (ADR-0013 D3, third leg)

Status: CLOSED 2026-09-03 — deployed (`9f0bcea`), backfilled (job 632, 7,924,745 tenders), served; the DE 1.x / sdk-0.1 residue is issue 344. Was: BUILT 2026-09-02 (`4b91542`, gate green, red-first on both rank
implementations). NOT YET DEPLOYED — deploy frozen until the issue-304 campaign's
job 610 lands (a restart re-runs the running job from the top); then
`backfill-original-lang` queues behind the deploy.
Kind: capability (language model) — closes the one gap ADR-0013's first amendment
left open
Relates to: 291 (the capability line; Lennart's 2026-09-02 question), 292 (the
vocabulary this compares against), 304 (the campaign it deploys behind), ADR-0013
Blocked by: 304 (deploy freeze only)

## The gap

D3 decided the chain `requested → ENG → original → any labelled → unlabelled`.
The 2026-08-27 amendment shipped it without the third leg, on the finding that
nothing persisted which variant was the original — it had looked at the
per-copy `LG` attribute.

## What was actually there

The notice-LEVEL statement is stored as a PROCEDURE code in `notice_codes` for
every era, verified on prod per era before building:

| era | field | seen |
| --- | --- | --- |
| eForms, every SDK incl. DE | `BT-702(a)-notice` | `SPA`, `DEU` |
| ted-export r208/r209 | `TED-LG_ORIG` | `EN` |
| text era | `TXT-OL` (the `OL:` line) | `FR`; absent on early-1990s notices |

## Built

* `tender_versions.original_lang`, nullable, boot-time `ALTER` (the `eur_cents`
  pattern). The fold derives it from the first of the three codes through
  `normalize_lang`, so it compares by equality with `tender_version_texts.lang`.
* Both rank implementations carry the leg: the SQL `title_rank` (list, detail
  header) and the in-memory rank behind `/v1/lots`. A NULL original ranks exactly
  as before the column existed.
* Exposed on `v_tenders`, `v_tender_notices`, `/v1/sql` (column note), list and
  detail JSON beside `notice_subtype`. `/docs` states the four-leg chain.
* `backfill-original-lang` (admin job): batched PK-seek walk, idempotent,
  resumable; stamps `NULL` rows only, leaves fold-written rows alone. ~14.3M
  versions; cost is per row, never per corpus.
* Tests: fold derivation on the eForms chain fixture (read back from the parse
  layer, not hard-coded); five read tests incl. the lots path; two backfill tests
  across all three eras plus the no-code, non-language-code, already-stamped
  and idempotence cases. Mutating both ranks turns exactly the leg-dependent
  tests red.

## To finish (after 610 lands)

1. Deploy the accumulated commits (boot runs the O(1) `ALTER`).
2. Enqueue `backfill-original-lang`; read its counts line.
3. Probe: a text-era tender with a French original and no English variant (there
   are none with EN by construction of the era — the EN edition IS the text) —
   after the 304 stage-2 acquisition that case becomes real; for now a legacy
   r209 tender with DE original + EN copy served by default as EN and with
   `?lang=xx` (absent) as DE proves the leg on prod.
4. Close this issue with the backfill's numbers: stamped vs left NULL per era —
   the NULL share IS the 1990s text-era share, and should say so.

## 2026-09-03 — deployed (`9f0bcea`, 09:03 UTC); backfill 632 running, and slow for a reason I wrote

The column landed at boot (`ALTER`, O(1)); `backfill-original-lang` enqueued as
job 632 at 09:05 UTC. Reads at 09:50 UTC:

| | |
| --- | --- |
| progress | 1,060,000 tenders walked in 2,662 s — **398 tenders/s**, 5.5 h projected |
| the low windows | tenders 0–20,000: 42,681 versions stamped, 7,841 NULL (the 1990s text era, expected); 500k and 1M windows ~89% stamped |
| the writer | held **20–40 s per 10,000-tender batch**; six `[store] writer acquired after a 13–38 s wait` lines behind it (the new 241 line, doing its job); WAL 9.3 GB and growing; no request cut |

Why: `backfill_original_lang` issued each version's UPDATE as its own
autocommit — ~9,200 WAL appends and fsyncs per batch. The sibling backfills are
one statement per batch; this one cannot be (the 639-2/T map is Rust), so the
fix is one `BEGIN IMMEDIATE … COMMIT` around the batch's rows. Deploying it
over the running job is safe: the walk is idempotent (stamped rows are skipped),
so the re-enqueued job re-covers the first 1.1M tenders in seconds and does the
rest at transaction speed.

## CLOSED 2026-09-03 10:3x UTC — column live, backfilled, served; one era gap filed as 344

```
632 ok: original_lang backfill walked 7924745 tenders — 4,187 s (70 min)
```

The "5.5 h" projection was the LOW ids' rate (398/s: legacy tenders, many
versions, many stamps); the mid and high ids ran at ~4,000–4,700 tenders/s, so
the per-row autocommit cost what it cost and the walk still finished in 70 min.
The transaction fix (`b02a222`) is for the next walk.

`/v1/tenders/6287622` now carries `"original_lang": "ENG"`; `?lang=de` / `fr`
flip as before. NULL share per era, 20k-tender PK windows:

| window | era (from the notices' profiles) | NULL |
| --- | --- | --- |
| 0–20,000 | 1990s text notices | 15.5% (7,841 of 50,522) — the OL-less years, expected |
| 300,000–320,000 | mixed eForms, incl. DE 1.x and sdk-0.1 | 10.1% — **all of it DE 1.x + sdk-0.1** |
| 1,000,000–1,020,000 | mixed | 9.9% |
| 1,500,000–1,520,000 | DÖE sdk-0.1 (98.6%) + DE 1.x | **99.9%** |
| 3.0M / 5.0M / 6.3M / 6.5M / 7.5M / 7.8M | eForms SDK 1.x, DE 2.x, r209 | **0%** |

So the leg is complete for every era whose parse records a language code, and
the r208/r209 + EU-eForms + DE 2.x corpus is fully stamped. The residue is not
"the era never said": the DÖE sdk-0.1 and eForms-DE 1.x XML DO carry
`cbc:NoticeLanguageCode` (`DEU`; the fixtures show it, line 12 of the sdk-0.1
one), but those two parsers do not record it — the sdk-0.1 notice's code rows
are five `SDK01-*` values with no language, the DE 1.x one has only
`UBL-DocumentLanguageID`, which is the procurement DOCUMENTS' language
(BT-708-shaped, "document plumbing" in the parse-only list), not the notice's.
That is a parser-inventory gap, filed as **issue 344**: record BT-702 for the
two profiles, re-parse them (the 251 machinery; DE 1.x + sdk-0.1 are ~7% of
versions), re-run the backfill (idempotent, now one transaction per batch).
Until then those versions rank with the leg absent — the old chain — which is
the correct fallback, never a wrong guess.
