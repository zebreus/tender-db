# ADR-0013 — The language model: one vocabulary, wholesale supersession, explicit fallback

Status: ACCEPTED 2026-08-26 (owner, on Lennart's direction: "make the ADR-sized
decisions, with maintainability, scalability and flexibility in mind"). Decides
the language half of issue 291; step 1 (vocabulary) already shipped as issue 292.

## Context

The canonical model has been multilingual since the 2026-07-19 representation
decisions: `Fact::Text { field, lang, value }`, `tender_version_texts.lang`, and
the detail API serves every stored variant. Three questions were never decided:
the tag vocabulary (three dialects had accumulated — issue 292), what a
correction does to the OTHER languages of a corrected field, and which language a
reader gets when they don't say. Portal expansion multiplies all three.

## Decisions

### D1 — Canonical vocabulary: ISO 639-2/T, three-letter uppercase, mapped at the fold boundary

The eForms codelist form (`ENG`, `DEU`, `FRA`) is canonical. Every importer's
dialect is mapped in exactly one place — `normalize_lang` at the two
`Fact::Text` creation sites in the projection (issue 292) — never in the read
layer, never per-parser. Unknown tags pass through UPPERCASED so a new source's
dialect fails visible (a tag the picks ignore) instead of silently splitting a
language across spellings. The parse layer keeps the source's tags verbatim
(raw is as-published; the canonical layer is ours).

*Maintainability*: one map, one place; a new portal is one review of one match
statement. *Scalability*: normalization is O(1) per fact at fold time — no
read-time cost. *Flexibility*: the raw tags survive in the parse layer, so a
future re-mapping is a refold, never a re-fetch.

### D2 — Supersession stays WHOLESALE per field (all language variants replace as one unit)

`Fact::key()` continues to treat every language variant of a field as one
replaceable unit: a notice that republishes a field replaces the whole set, in
whatever languages it carries.

Why NOT per-language carry-forward, though it looks friendlier to coverage: the
variants of a field are translations of ONE logical value. A corrigendum that
changes the ENG title has changed the *value*; carrying the old DEU text forward
would serve a stale translation as if it were current — factually wrong in the
reader's language, and undetectably so. A missing translation is visible and
recoverable (fallback chain, D3); a stale one is silent misinformation. The
supersession fold's correctness bar (issue 202, ted-empirical-checks) stays
value-first.

*Flexibility*: the superseded variants are not lost — they remain on the earlier
versions of the chain, so a future "show historical translations" feature reads
them from `tender_version_texts` at seq < head with zero model change.

### D3 — Reader selection: explicit `?lang=`, fallback chain `requested → ENG → notice original → any (deterministic)`

The read layer's picks are parameterized, not multiplied: the existing
ORDER-BY-rank picks (head_title, list title, lot summaries) take the requested
language as the top rank, then `ENG`, then the version's original language, then
the deterministic first-seen. No per-language denormalized columns until a
measured list-latency need exists (the pick is one indexed subquery today);
if that need arrives, denormalize per REQUESTED language lazily, not per
language eagerly.

*Scalability*: no schema growth per language. *Maintainability*: one pick
expression, shared (292 already counseled a single helper). *Flexibility*: the
chain is data-driven — a per-user or per-portal default is a parameter, not a
schema change.

### D4 — Organization names go multilingual via ONE satellite

`organization_names(org_id, lang, name, name_norm)` beside the existing single
`name` (which becomes the ENG-or-original denormalized head, same pattern as
`current_title`). Mention capture widens to keep all language variants of a
name. The provisional-merge key stays `(name_norm, country)` computed over a
DESIGNATED language (the same one the single `name` holds), so merge semantics
do not change under this ADR; any smarter cross-language matching belongs to
issue 300 and must clear issue 168's false-merge bar first.

*Maintainability*: merge logic untouched now; the satellite is additive.
*Scalability*: names are small; the satellite grows with orgs × languages
actually published, not the corpus. *Flexibility*: 300's matcher gets its
multilingual input without another migration.

### D5 — Coverage stays "EN + original" until storage says otherwise

Widening to all languages (r208/r209 translation copies re-parse; text-era
non-EN re-dispatch) is a measured storage decision per era (multilingual text
≈86% of parsed size; all-language ≈188 GB), taken era by era behind a capacity
check — never implied by this ADR.

## Consequences

- Issue 292's backfill (era refold or batched UPDATE, sized first) makes stored
  legacy tags canonical; until then the interim state is mixed and documented.
- The picks' `'ENG'` literals become correct everywhere after the backfill;
  consolidate them behind one helper when touched (292 note).
- New-portal onboarding checklist gains one line: "map your language dialect in
  `normalize_lang`."
- /docs gains a language paragraph (what `?lang=` does, the fallback chain, the
  wholesale-supersession behavior on corrections).

## Amendment 2026-08-27 — D3 shipped; the "original" leg awaits a data source

The `?lang=` selector shipped with the chain **requested → ENG → any labelled →
unlabelled**, applied at the read-time picks (the tender list's and detail's
title, lot titles, SSE `include_data`) — the survey that preceded the build
found the REST list computes its title per request, so no denormalized
per-language columns were needed. The fold-time surfaces (`v_tenders`/`v_lots`
on /v1/sql, `current_title`) deliberately keep the deterministic default.

The decided chain's third leg — the notice's ORIGINAL language — turned out to
have no persisted data source: `tender_version_texts.lang` records each
variant's own tag, and nothing marks which was the original (the r209 parser
sees the `LG` attribute but never stores it). Honoring that leg needs one
additive column at parse/fold time plus a refold; until someone wants it, the
chain skips from ENG to the deterministic tail, and `lang` is documented as a
projection SELECTOR (never a predicate, never in `ignored_filters`).

## Amendment 2026-09-02 — the "original" leg has a data source after all; built

The first amendment was wrong about the data: it looked at the per-copy `LG`
attribute and concluded "nothing marks which was the original". The notice-LEVEL
statement exists in every era and was already stored as a PROCEDURE code in
`notice_codes`:

| era | field | seen on prod |
| --- | --- | --- |
| eForms, every SDK incl. DE | `BT-702(a)-notice` | `SPA`, `DEU` |
| ted-export r208/r209 | `TED-LG_ORIG` (the element, not the copy attribute) | `EN` |
| text era | `TXT-OL` (the `OL:` line; absent on early-1990s notices) | `FR` |

So honouring the leg is a fold-time read, not a parser change: `original_lang`
in `ingest::project` takes the first of those three the parse carries and passes
it through the same `normalize_lang` every text tag goes through, so the stored
value compares by equality with `tender_version_texts.lang`. It rides into a new
nullable `tender_versions.original_lang` (boot-time `ALTER`, the `eur_cents`
pattern) and is exposed on `v_tenders`, `v_tender_notices`, `/v1/sql` and the
list/detail JSON. The chain is now the one D3 decided:

**requested → ENG → original → any labelled → unlabelled**

applied in the SQL `title_rank` and the Rust-side ranks alike. The standing corpus
is stamped by the batched `backfill-original-lang` job (PK seeks into
`notice_codes`, `original_lang IS NULL` rows only, resumable by watermark) rather
than a corpus refold; new folds write it directly. Unchanged: `lang` is a
selector, never a predicate; the fold-time surfaces keep the deterministic default.
