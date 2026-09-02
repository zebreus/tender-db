# 340 — the `?lang=` fallback's "original language" leg (ADR-0013 D3, third leg)

Status: BUILT 2026-09-02 (`4b91542`, gate green, red-first on both rank
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
