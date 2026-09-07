# 341 — eForms-era notices are served in their original language only; the translations are not in TED's bulk

Status: ready-for-agent — DECIDED 2026-09-07 (owner): option 1, the eForms era serves the notice's original language and the docs say so; the one remaining unit is the `/v1/docs` note and a README line (decision entry at the bottom). Was: needs-decision (filed 2026-09-02 on Lennart's "didn't you create issues
for the per-era gaps"). Owner recommendation below; the decision is Lennart's
because every option is a product/cost call, not a build.
Kind: coverage gap (language) — the one era where "full multilanguage" is not
recoverable from data we hold or can download in bulk
Relates to: 291 (the capability line), 304 (the other two eras' campaigns), 340
(the fallback's original leg, which makes the gap visible rather than hidden),
ADR-0013 D5 ("coverage stays EN + original until storage says otherwise")

## Measured 2026-09-02 (bounded window, tender ids 7,850,000–7,900,000)

| | |
| --- | --- |
| versions with a title | 70,175 |
| versions with a title in 2+ languages | **128 (0.18%)** |
| title languages | POL 24,610 · FRA 18,441 · DEU 10,269 · SPA 5,550 · CES 5,326 · BUL 4,296 · ITA 3,854 · **ENG 3,601** · RON 3,498 · … |

An eForms notice in TED's bulk carries its original language and, in 0.18% of
cases, a second one the publisher supplied. The per-language renderings TED shows
in its interface are machine translations produced by the Publications Office and
are **not part of the bulk packages** — so unlike r208/r209 (translation copies
inside stored XML, issue 304 stage 1) and the text era (per-language zips that
exist and were never downloaded, 304 stage 2), there is nothing to re-parse or
re-fetch. For 2023-10 onward, `?lang=de` on a Polish notice can only fall back.

## Options

1. **Document it and stop.** `/docs` says eForms serves the original; the
   fallback chain (now with the original leg, 340) serves the notice's own
   language honestly. Zero cost. Loses nothing that exists today.
2. **TED's per-language renderings.** Fetch each notice's translated views from
   TED (the HTML/API surface, not the bulk). Unknowns that would need a research
   pass before any decision: whether the API exposes translations per language,
   rate limits and terms for ~3–4k notices/day × 23 languages, storage (the
   text layer would grow ~20×; the 304 measurement says +52% per era for a
   1.5× copy count), and provenance labelling — a machine translation must be
   served as one, not as published text.
3. **Own machine translation.** Same storage and labelling questions plus model
   cost and quality ownership; nothing in the stack for it today.

## Recommendation

Option 1 now, with 2 kept as a research issue if a product need names it. The
argument: the original-language content IS the published record, the fallback
chain serves it, and the two recoverable eras are being recovered. Translating
23 languages of new notices daily is a different product than a canonical
tender registry, and it would be the first place the API served text the
publisher never wrote.

## What is already true regardless

Organization names are multilingual where published (38.7M variants), amounts
are EUR-normalised across every era, and `?lang=` works wherever a variant
exists. This issue is only about eForms text variants that do not exist in the
data.

## Decision (2026-09-07, owner)

Option 1. TED's per-language renderings are machine translations outside the bulk, on
an interface whose terms and stability for bulk fetching are unresearched, and our own
MT would put text in the corpus that no publisher wrote. The fallback chain already
serves the notice's own language honestly (requested → ENG → original → any labelled),
and 0.18 % of versions carry a second published language, which we do serve. So: the
eForms era serves the original, documented in `/v1/docs` and the README. Reversible by
a later entry if a bulk translation source appears. Unit: the two doc lines (a `/v1/docs`
note is a string in `sql.rs`, so it rides the next code deploy).
