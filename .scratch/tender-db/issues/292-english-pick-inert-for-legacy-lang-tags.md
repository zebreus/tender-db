# 292 — the "English wins" title pick is inert for the whole pre-eForms corpus (lang-tag vocabulary never normalized)

Status: FIX DEPLOYED 2026-08-26 (owner, rev 5036a09, /health green) — fold-boundary normalization (`normalize_lang`,
project.rs: ISO 639-1 → 639-2/T map, unknown tags pass through uppercased, None stays
None) applied at both `Fact::Text` creation sites; unit test + red-first integration
fixture (`legacy_two_letter_lang_tags_normalize_so_the_english_pick_fires`: DE+EN
two-letter variants → stored as DEU/ENG, English wins current_title; proven RED with
the normalization bypassed). Full `ops/check.sh` green (65 suites). FORWARD-ONLY until
the backfill: stored legacy rows keep `EN`/2-letter tags until an era refold — the
bounded sizing probe (how many legacy tenders carry >1 title language) decides
refold-vs-batched-UPDATE as the next unit.
Kind: correctness (title/language selection on the read + fold surfaces)
Severity: MEDIUM (wrong-language titles served for legacy-era tenders whenever a
non-English variant sorts first; silent — nothing errors)
Relates to: 291 (this is step one of the multilanguage capability), 239 (current_title
denorm), CONTEXT.md language policy.

## The defect

Three language-tag dialects coexist in `tender_version_texts.lang` / parse-layer
`notice_texts.lang`, copied verbatim with no normalization step anywhere in ingest:
- eForms / eForms-DE / DÖE sdk-0.1: 3-letter uppercase as published (`ENG`, `DEU`) —
  from `@languageID` (eforms/value.rs).
- r208/r209 legacy TED: 2-letter `LG` attribute verbatim (`EN`, `DE`, `FR`) —
  `kept_languages()` keeps `["EN", LG_ORIG]` (r209/parse.rs:199-207).
- Text era: `EN` on English renderings, NULL on names/original-language bodies.

Every "English wins" pick compares the literal `'ENG'` (owner-verified, six sites):
- canonical.rs:962 — `head_title` (`lang.as_deref() == Some("ENG")`), which stamps
  `tenders.current_title` at fold time;
- canonical.rs:609 — the v_lots title subquery `ORDER BY (x.lang = 'ENG') DESC`;
- read.rs:1348 — the tender-list title pick;
- read.rs:2362 — the lot-summary rank (`Some("ENG") => 2`);
- lib.rs:994 and lib.rs:2818 — the current_title backfill/list SQL.

Legacy rows carry `EN` or NULL, never `ENG` — so for the ENTIRE pre-eForms corpus the
English preference silently never fires and title choice falls to scan order (a
tender with FR+EN titles can serve the French one). Bilingual co-original notices
(Belgium FR+NL, Bolzano DE+IT) make this visible.

## Fix direction (also the portal-expansion prerequisite)

Normalize the language vocabulary at PARSE time (single canonical form — pick one:
ISO 639-2/T 3-letter, mapping r209's 2-letter and any future portal's dialect at the
boundary, the same "importers translate at the boundary" rule CONTEXT.md already uses
for the Tender/Bid vocabulary), then:
1. re-parse is NOT needed if normalization is applied in the projection read of
   parse-layer rows (cheaper: normalize in `Ident`/fold input) — decide layer;
2. a field-scoped refold of the affected eras re-derives `current_title` and the
   version texts under the normalized tags;
3. the six pick sites keep their single literal (now guaranteed to match), or better,
   compare through one shared helper so the vocabulary lives in ONE place;
4. pin with a fixture: an r209-style tender with `EN`+`FR` variants must serve the
   English title on list, detail, and current_title after the fold.

Sizing note before the refold: measure how many legacy tenders actually carry >1
language variant of the title (bounded snapshot read) — that is the blast radius and
the acceptance number.

## Backfill sizing (2026-08-26 probe, Aug-24 snapshot — first query in)

The 2-letter population is corpus-scale: PL 28.4M, EN 16.2M, DE 14.4M, FR 11.5M,
RO 10.7M, HU/BG ~4.7M each, … — ~100M+ tender_version_texts rows carry legacy
tags (3-letter eForms rows: DEU 8.2M, POL 5.6M, ENG only 570k; NULL 2.5M text-era
names). Consequences:
- An in-place batched UPDATE (~30 tag values over ~100M rows) is issue-63
  WAL-per-row territory — chunked-by-rowid, checkpoint-per-chunk if ever done.
- A plain `project(rebuild=false)` refold would SKIP every unchanged tender (the
  chain-as-state-key early return) — the backfill needs an EPOCH-BUMP refold
  (issue 99/179 machinery), scoped to the legacy eras or full.
- **Decision: ride the ADR-0014 `eur_cents` refold** — it needs the same epoch
  bump for the same reason (new derived satellite values on unchanged chains), so
  ONE epoch-bump refold carries both the lang normalization and the currency
  column. No separate lang backfill unless the second probe query (tenders
  actively serving a possibly-wrong-language title — still running in tmux)
  comes back large enough to justify a dedicated earlier scoped refold.
