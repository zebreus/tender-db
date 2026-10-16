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

Status: **effective** with unit 2b. Recorded with unit 2a, the dry `refresh-org-names` plan
(`store::elect_head`, `Db::plan_org_name_refresh`), so the plan's semantics were written down before
any head moved; unit 2b builds the wet run (`refresh-org-names` with `dry_run: false`,
`Db::apply_org_name_refresh`) and its undo (`revert-org-name-refresh`). A head is rewritten only by
an operator's wet run of a stored dry plan elected under the same rule epoch by the same binary, and
from that run on `organizations.name` is the elected head, not the first-seen spelling.

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
   Dumbrăveni` and `COMUNA DUMBRAVENI` vote together. The fold is the head election's alone (the vote
   key, the twin test of 5, the family hold of 6); the N2/N3 keys, `altid_name_key` and the merge
   vetoes keep their diacritics.
3. **When the head changes.** Only when one of these holds:
   - (a) the head is empty or a non-name (placeholder, blank, void_lot) and some name votes;
   - (b) a cluster other than the head's holds at least 55% of the votes, at least 3 votes, and at
     least twice the head's votes;
   - (c) the head's cluster holds at most 1 vote, and a unique top cluster holds at least 3 votes and
     at least 25% — unless the change is an initialism of the head (6).

   Otherwise the head stays byte for byte. No single mention, near tie or legal-form difference moves
   it.

   **(b) is HELD for a rename in progress (rule `524u2-v7`, the recency decision of 2026-10-15,
   panel wf_ce13ee62-9e9).** The capped run 2517 put five organizations back to a FORMER name: their
   head was already the current name, and (b)'s all-history majority restored the old one. So when
   the head's own cluster holds at least 2 votes, (b) reads the organization's dated procedures. An
   ITEM is a distinct (vote key, tender) pair of the voting mentions, dated by its newest
   `tender_versions.published_at` (a notice's procedure is its smallest `tender_id`); k is the head
   cluster's items; every item is ranked newest first (an elected or other item before a head item on
   an equal instant, then tender, then key), and the window is the newest k. When k ≥ 2 and the head
   holds at least two thirds of the window, the change is HELD `recent` with the spelling it would
   serve; a third cluster's items count against the head. A head that is unfit to serve even against
   itself for a reason other than being all lower case (a control character, mojibake, over 200
   characters, a non-name), or whose vote key carries a token of decimal digits only that the elected
   key lacks (an address number, a postcode, a NIP, `LOT 2`'s `2`), is junk and not protected: (b)
   applies as before, and the release is counted. A caller without dates gets HOLD `undated` (it
   fails closed); a finished plan dates every candidate and must hold none, and the wet run refuses
   a plan that does. (a) never reads dates.

   **(b) and (c) are HELD when the head names the newest procedure (rule `524u2-v8`, the owner's
   decision of 2026-10-15 after plan 2523's dated re-judge).** The window leaves a residue: an H = 1
   or single-procedure rename looks, by dates, like a stray newest mention. Plan 2523 changed about
   800 such heads, and the re-judge read 9 of its 30-sample wrong (an organ served for its entity, a
   majority made of one notice's lot repeats, misspellings, a placeholder identifier's pile-up) and
   6 right. So when the organization's single newest dated item is a head item, a (b) or (c) change
   is HELD `residue` with the spelling it would serve — unless the junk exemption above releases it.
   "Single" is the window's tie rule: on an equal instant (a date-only notice's day) any other item
   ranks newer, so the head must be the only one of the organization's names printed on its newest
   publication day. One notice that prints both the entity (the head) and an organ (the elected
   name) on that day does not hold; whether such a same-tender tie should is a rule question, not
   decided here.
   An undated (b) with H ≥ 2 holds `undated` first; an undated H ≤ 1 change changes as before. (a)
   is exempt: a defective head always changes. The Sheffield shape (an H = 1 stray printed newest)
   is held too, a missed fix, never a regression (issue 548). The same re-judge narrowed the junk
   exemption: v7 released 1846, whose `CA2BM` acronym carries a digit inside a word, and 3909, whose
   registered current name `act legal s.r.o., advokátní kancelář` is deliberately lower case — both
   back to a former name.

   **(b) and (c) are HELD on five pure tests of the two names (rule `524u2-v9`, the owner's decision
   of 2026-10-15 after plan 2526's dated re-judge, panel wf_5cca11b1-622).** Plan 2526 failed its
   change:R1, change:R2 and released bars (2, 2 and 5 wrong). Its wrongs had causes a test of the
   head and the served spelling can see, and the panel measured each test on the whole plan, so
   every piece is a pure function of (head, display, rule, keys): the dry plan and the wet re-check
   agree by construction, with no DB read. A (b) or (c) change that reaches the served spelling past
   the guards of 6 is HELD, the first that holds naming it:
   - `review_body`: the new name is a procurement review body's (a review chamber or court, by a
     folded phrase list) and the head is not review-like (no such phrase, no review stem, not led by
     `TA`, `TAR` or `VK`): another entity's procedures naming the chamber that heard a complaint
     (`Universitätsklinikum Freiburg` → `Vergabekammer BW`). A review body's self-rename changes.
   - `organ`: the new name is led by a contracting organ (`Junta de Gobierno`, `Alcaldía`, `Pleno`,
     `Presidencia`, `Rectorado` …, a folded phrase list) and the head is not: PLACSP's organ served for
     its entity. The reverse (an organ head to its entity) changes.
   - `code_added`: the new name adds a whole number the head's letters and digits do not spell, in a
     code position (in brackets, joined by `_`, the last token, a leading list number), and its vote
     key less that number is the head's (`Commune de Chiconi (976)`).
   - `address_added`: such a number sits after a comma or semicolon (an appended address or seat),
     unless the name is a Czech or Slovak school's, which carries its seat by law.
   - `caps_variant`: the new name is an ALL-CAPS respelling of a mixed-case head's same words, legal
     forms spelled out or not aside.

   The same decision narrows the junk exemption's numbers twice: a number the elected name prints
   glued to the head word beside it (`SMD 3` against `Smd3`) is the name's own, not junk; and the
   elected name must be the head minus junk, losing at most one other word of it (one, so an expanded
   acronym still releases: `SDIS 49` → `Service départemental …`). A release either stops returns to
   its hold and is counted (`release_withheld`): 15 in plan 2526, 8 of them right fixes forgone —
   the decision's accepted cost. (a) is exempt from all of it. The vote key, the N3 key, the family
   table and their epoch do not move. The plan's uniform samples are salted with the rule's epoch, so
   a re-judge reads a fresh draw; and a wet run may drop the class a re-judge failed (`exclude_rules`,
   `exclude_released`) before its re-check, with no re-plan.

   **(b) is HELD while the head is still in print, and (c) is planned only (rule `524u2-v10`, the
   owner's decision of 2026-10-16 after plan 2527's dated re-judge).** Plan 2527 failed change:R1 and
   change:R2 again (4 and 4 wrong of 30), each wrong a new shape no test of the two names sees (a
   registry marker appended, an eSender's abbreviation, another entity on a shared identifier, an
   agent's tail). Pooled over the three dated re-judges, the head's recency separated them: a (b)
   change on a head last printed 3 or more years before the plan read wrong once in 61 under v9, on a
   head printed within 3 years 4 times in 29. So a (b) change whose head's newest dated item is within
   1,095 days of the plan's start (`as_of`, stored in the plan; the wet re-check elects with the
   plan's, so a head printed since reads held and is skipped) is HELD `head_current` with the spelling
   it would serve — unless the junk exemption releases it, as it releases `recent` and `residue`. It
   is asked after the tests above, so a change they hold keeps their reason; an undated (b) follows
   v7; (a) is exempt. (c) showed no such split and keeps failing: the planner still elects and lists
   it, for issue 548, but every wet run drops it (`exclude_rules: ["R2"]`, v9's pre-committed filter).

   **(b) is HELD on a stale top with a rival, and three narrow holds join (rule `524u2-v11`, the
   owner's decision of 2026-10-16 after the capped run 2555's dated spot check, panel wf_5d3b6fa8-186).**
   The capped run's lowest ids read a revert and three wrongs. Three of the four had a voting cluster
   other than the elected one and the head's (a rival) and an elected spelling last printed more than a
   year before the plan: a newer third name had superseded it, the head's name was printed again in
   another spelling, or the elected spelling was another entity's. With no rival, the holds above
   already make the elected spelling the organization's newest printed name, so neither failure can
   occur there. So a (b) change that passes every hold above is HELD `top_stale` with the spelling it
   would serve when a rival votes and the elected cluster's newest dated item is before the plan's
   `as_of` less 365 days. Undated counts as stale. A head the window, the residue holdout or
   `head_current` already released ships; the junk exemption is not asked again. The served-spelling
   tests gain `doubled` (the display's words are X then X, for a head that is not doubled) and
   `seat_dropped` (the head is the display plus a 2–3 word seat tail led by `w`, `we`, `v`, `ve` or
   `vo`), for (b) and (c). The bind guards gain `placeholder_id` for every rule: the identifier is
   `KEINEANGABE`, `UNBEKANNT` or `NICHTVORHANDEN` with any digits, or all zeros. The vote key and its
   epoch do not move. The plain-age rule, every stale (b) whatever its rivals, is pre-built as a wet-run
   filter (`exclude_stale_top`), which every wet run passes if the dated re-judge of the stale no-rival
   cell fails. Plan entries also carry `rival_newest`, the rival's newest print, which no verdict reads
   (for issue 548's direct supersede test).

   **A head whose only defect is a line break is released only for its own lines, else served
   collapsed when it wraps (rule `524u2-v12`, issue 552's decision of 2026-10-16, revised after the
   adversarial check wf_0744b445-0ee).** Plan 2543's dated re-judge read 5 of its `unfit` junk releases
   wrong, all five on heads whose only defect was control whitespace (a `\n`, `\r` or `\t`): the
   release served a shorter sibling spelling (a review chamber without its Land, a school without its
   patron and town) or a truncated or foreign one. So a CONTROL-ONLY head — `control` against itself,
   its whitespace-collapsed form free of control characters and U+FFFD, and that form fit (or lower
   case with no non-name class) — is released `unfit` from `recent`, `residue` or `head_current` only
   when the elected key's tokens sit inside one of the head's lines' vote keys (lines split on the
   characters that are both control and whitespace), or equal the union of a contiguous run of whole
   lines. Otherwise the release is withheld (`release_withheld: lines`) and the hold stands. Such a
   withheld head is instead CHANGED to itself with its whitespace collapsed (rule `R0w`) when it has a
   break and every break is a WRAP: the line before ends in no legal form (the multi-token forms
   included) and in no `,`, `;` or `:`, and its last word ends in an unspaced hyphen, or is a connector
   word with no `.` (`und`, `of`, `z`, `και`, …), or the next line starts lower case — and when the
   collapsed head would itself stay held: the junk exemption, asked of it against the same elected
   key, does not release it (a bare number the elected key lacks would release it `digits` on the
   next plan and serve the withheld spelling after all; then the `lines` hold stands). Only the bind
   guards are asked of R0w (a reason holds it, as for (a)); the recency, residue, `head_current`,
   `top_stale` and serve holds are not. No address is cut: the collapsed head keys as the raw head
   (`head_vote_key` collapses whitespace), so it stays in its own cluster and the holds that protected
   it keep standing on later plans; a cut would re-key it and lose them. Every other unfit head
   (control plus `long`, control plus mojibake) releases as before. Every wet run passes
   `exclude_rules: ["R2", "R0w"]` and keeps `exclude_released: ["unfit"]` until a dated re-judge of the
   guard-passed releases and every R0w change passes. Accepted limit: an R0w head is fit, so once its
   `head_current` lapses, v10's majority may again replace it (issue 548's over-blocking work).
4. **D4's ENG-or-original survives as a guard.** A head whose dominant script differs from the elected
   spelling's (TED's Latin rendering over Cyrillic or Greek mentions) is kept: it is the ENG head. The
   guard does not apply to an empty or non-name head (3 (a)): a placeholder is no rendering of
   anything, so it is replaced whatever script the mentions are written in.
5. **The served spelling.** Within the winning cluster, the job serves a FIT spelling:
   - no control character or U+FFFD;
   - no non-name class;
   - no placeholder or void net hit;
   - no consortium mark the old head lacks;
   - no mojibake (UTF-8 read as cp1252 or Latin-1: `Ã`, `Â`, `Ä` or `Å` directly before a
     continuation byte's character or `€`/`™`, `È™`/`È›`, or `Ã`/`Â` after a lower-case letter at a
     word's end: `MÃ¼nchen`, `SOCIETÃ€`, `SocietÃ  Cooperativa`);
   - at most 200 characters;
   - not all lower case.

   Each fit spelling is served with its whitespace collapsed (trimmed, every run one space; a tab
   or a newline is a control character, so unfit, and a spelling must be fit both as published and
   collapsed), and the fit spellings that collapse alike are counted together (rule `v6`, after dry
   plan 2515 planned 137 names with a whitespace run, a dropped `&` among them). So no change
   serves a doubled space (a head the rule keeps stays byte for byte), and a served head is not
   always byte for byte a published spelling, while its `name_norm` and its vote key are theirs:
   the vote key reads whitespace runs as one space, so the served head stays in its own cluster,
   and a display equal to the head is a keep. Among the collapsed spellings, case decides first (a
   spelling that is not ALL-CAPS before one that is; `ß`, `ª` and `º` have no one-letter capital
   and do not make a spelling mixed case), then the most
   votes, then byte order. Then, if that winner carries no diacritic (no non-ASCII letter), the
   most-voted fit spelling of its case that is its TWIN (equal once lower-cased, diacritics folded and
   punctuation dropped) and does carry one is served instead. So "The University of Sheffield" (1
   vote) is served over "UNIVERSITY OF SHEFFIELD" (14), and "Dotación y Equipamiento, S. L." (1) over
   its twin "Dotacion y Equipamiento S.L." (4); but "Acme Ltd t/a Café Rouge" (1), which only shares a
   cluster with "Acme Ltd" (20), is no twin and never wins on its accent, and a winner that already has
   its accents ("sociálnych", 717) keeps its place against a typo twin ("sociálných", 1). With no fit spelling, nothing changes. Register names are not
   used until a register-name cache exists.
6. **Held for review.** A change is held when it would:
   - swap one legal-form family for another (`legal_form_family` or `gb_legal_family`, read on the
     diacritic-folded names, and on them with dotted single letters joined, set on both sides and
     different: `Acme Schweiz AG` → `Acme Sàrl` or `Acme S.à r.l.`);
   - drop a consortium mark;
   - fail the NHS ODS name check the old head passed;
   - by (c) only, replace the head with an initialism of it: a token of 3 to 5 upper-case letters, no
     legal form or designation (`SRL`, `GMBH`, `SCOP`, `SPL`, `PPH`, …), that is the initials of a run
     of the head's words, short particles inside the run skippable (`CDC` for `Communauté de communes`,
     `CDCPC` for `Communauté de communes du Pays de Caux`), that the new name no longer spells out and
     that is not the head's own dotted initials (`S.A.V.E.T.` → `SAVET`) — in a new name that is
     either nothing but that token or not ALL-CAPS. A 2-letter token (`EY`) is none. A head cut to
     some of its words, or stripped of an address, still changes. (b)'s majority is not
     second-guessed.

   The head is an input to the resolver's and merge planners' name gates. (c)'s 25% minority can be a
   publisher's initialism of a correct full head (dry plan 2501's judged samples).
7. **Keys.** `name_norm` follows, as `org_name_norm(new head)`. For identifier-bearing rows it is the
   name-search key, never a bind key. `organization_names` and `org_name_verdicts` are untouched.
   `org_match_keys` catches up at its next wholesale build.
8. **Announcement.** Every change writes a byte-exact pre-image to `org_name_refresh_log` (revertible by
   job: `revert-org-name-refresh` restores it under a compare-and-set on the pair the job wrote) and
   one seq-less `organization changed` row. The rename itself is a compare-and-set on the plan's
   pre-image (`name`, `name_norm`, `identifier IS NOT NULL`), after a pre-pass that re-elects every
   planned organization and aborts the run, writing nothing, when more than max(2%, 5) have drifted
   or gone since the plan. No Tender or lot rows are written:
   `parties[].organization_name` is a read-time join of the organization, so under ADR-0017 D5 the
   moved entity is the organization. This follows the identifier-restore precedent.
9. **Upkeep.** A weekly capped auto-run keeps heads current after merges and drops. NOT BUILT YET: that
   is issue 524 unit 2c, and until it lands the operator runs `refresh-org-names` by hand. A
   from-archive rebuild restores first-seen heads until the job runs again.

What a notice published stays available as `parties[].mention_name` (issue 456 design 3). Issue 456's
design question 2 is answered here.
