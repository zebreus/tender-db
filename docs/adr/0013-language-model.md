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

## Amendment 2026-10-15 — D4's head is ELECTED from the org's mentions, not first-seen (issue 524)

Status: recorded with unit 2a, the dry `refresh-org-names` plan (`store::elect_head`,
`Db::plan_org_name_refresh`), so the plan's semantics are written down before any head moves. It
takes effect at unit 2b's first wet run, which marks it effective; until then no head is rewritten.

D4 called `organizations.name` "the ENG-or-original denormalized head". The mint never did that. It
writes the first mention's first-seen spelling, in whatever language (`ingest::project`: "the
designated single head keeps its first-seen semantics"), and nothing ever rewrote it. So merges, drops
and in-place mention refreshes left heads that few or none of an org's mentions say. Census job 2492
counted 56,144 minority heads and 1,585 absent heads, among them 31558903 served as "Sheffield Hallam
University" over 16 Sheffield mentions, and 22713056 served as "s.o.".

Decided:

1. **Who.** The mint still writes the first-seen spelling. The dry-first, ledgered job
   `refresh-org-names` re-elects the head of IDENTIFIER-BEARING organizations only. An identifier-NULL
   organization keeps its head, because its `name_norm` is the issue-234 bind and provisional-merge
   key. So D4's provisional-merge key and merge semantics do not change. An organization whose
   identifier triple carries a `wrong` identifier verdict, or a `related` one not yet applied (the
   predicate `ORG_COLUMNS` serves `identifier_verdict` by), keeps its head as well: there the bind is
   the defect (issue 456 design 1), and the mentions it gathered do not name the entity.
2. **Votes.** A mention votes unless its name is NULL, blank, or a `not_a_name` class other than
   `award_summary`. Votes group by the head vote key: `crosswalk::altid_name_key` of the name with its
   Latin diacritics folded (NFD with the combining marks dropped; `ø` → `o`, `ł` → `l`, `ß` → `ss`,
   `æ` → `ae`, `œ` → `oe`), with every legal-form token dropped. Casing, spelling marks, diacritics,
   "the" and a legal form (Ltd/Limited/none, S.R.L./SRL/none) are therefore one name: `Comuna
   Dumbrăveni` and `COMUNA DUMBRAVENI` vote together. The fold is the vote key's alone; the N2/N3 keys
   and `altid_name_key` keep their diacritics.
3. **When the head changes.** Only when one of these holds:
   - (a) the head is empty or a non-name (placeholder, blank, void_lot) and some name votes;
   - (b) a cluster other than the head's holds at least 55% of the votes, at least 3 votes, and at
     least twice the head's votes;
   - (c) the head's cluster holds at most 1 vote, and a unique top cluster holds at least 3 votes and
     at least 25% — unless the change is a shorthand of the head (6).

   Otherwise the head stays byte for byte. No single mention, near tie or legal-form difference moves
   it.
4. **D4's ENG-or-original survives as a guard.** A head whose dominant script differs from the elected
   spelling's (TED's Latin rendering over Cyrillic or Greek mentions) is kept: it is the ENG head. The
   guard does not apply to an empty or non-name head (3 (a)): a placeholder is no rendering of
   anything, so it is replaced whatever script the mentions are written in.
5. **The served spelling.** Within the winning cluster, the job serves a FIT spelling:
   - no control character or U+FFFD;
   - no non-name class;
   - no placeholder or void net hit;
   - no consortium mark the old head lacks;
   - at most 200 characters;
   - not all lower case.

   Among the fit spellings, case decides first (a spelling that is not ALL-CAPS before one that is;
   `ß`, `ª` and `º` have no one-letter capital and do not make a spelling mixed case), then a spelling
   that carries diacritics over its ASCII-folded twin (the one with more non-ASCII letters; every two
   spellings of one cluster are such twins, since the vote key folds diacritics), then the most votes,
   then byte order. So "The University of Sheffield" (1 vote) is served over "UNIVERSITY OF SHEFFIELD"
   (14), and "Dotación y Equipamiento, S. L." (1) over "Dotacion y Equipamiento S.L." (4). With no fit
   spelling, nothing changes. Register names are not used until a register-name cache exists.
6. **Held for review.** A change is held when it would:
   - swap one legal-form family for another (`legal_form_family` or `gb_legal_family` set on both sides
     and different);
   - drop a consortium mark;
   - fail the NHS ODS name check the old head passed;
   - by (c) only, replace the head with a shorthand of it: a token of 2 to 5 upper-case letters that is
     the initials of a run of the head's words (`CDC` for `Communauté de communes`), or the head's words
     cut to a proper subset that drops only words, one of three letters or more (`SAS C3i` for `C 3 I -
     CONCEPT EN INGENIERIE D'INFRASTRUCTURES INNOVANTES`). A head that loses digits, a postcode or an
     address with its number still changes. (b)'s majority is not second-guessed.

   The head is an input to the resolver's and merge planners' name gates. (c)'s 25% minority can be a
   publisher's shorthand of a correct full head (dry plan 2501's judged samples).
7. **Keys.** `name_norm` follows, as `org_name_norm(new head)`. For identifier-bearing rows it is the
   name-search key, never a bind key. `organization_names` and `org_name_verdicts` are untouched.
   `org_match_keys` catches up at its next wholesale build.
8. **Announcement.** Every change writes a byte-exact pre-image to `org_name_refresh_log` (revertible by
   job) and one seq-less `organization changed` row. No Tender or lot rows are written:
   `parties[].organization_name` is a read-time join of the organization, so under ADR-0017 D5 the
   moved entity is the organization. This follows the identifier-restore precedent.
9. **Upkeep.** A weekly capped auto-run keeps heads current after merges and drops. A from-archive
   rebuild restores first-seen heads until the job runs again.

What a notice published stays available as `parties[].mention_name` (issue 456 design 3). Issue 456's
design question 2 is answered here.
