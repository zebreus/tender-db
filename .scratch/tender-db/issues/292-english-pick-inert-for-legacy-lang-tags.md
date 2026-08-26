# 292 — the "English wins" title pick is inert for the whole pre-eForms corpus (lang-tag vocabulary never normalized)

Status: DIAGNOSED 2026-08-26 (owner — found by the issue-291 multilanguage survey;
every location owner-verified against the code)
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
