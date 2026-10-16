# 538 — publisher double-escaped HTML entities (`&#224;`, `&amp;quot;`, `&amp;lt;br/&amp;gt;`, `&apos;`) reach served titles, descriptions and party names

Status: ready-for-agent — unit 1 MEASURED and unit 2 DECIDED 2026-10-16 (below): TED eForms carries the class, 0.9–2.1 % of Tenders per window; legacy about 0.2–0.35 %; FTS, DÖE and old TED about 0; org names 0.02 %. Texts: decode at projection (numeric refs plus the five XML names, case-insensitive, ≤ 3 passes, `<br/>` → newline). Names: deferred to unit 4. Next: unit 3 (build) after 533 and 524 v10 leave the tree.
Kind: data quality (served text and names) — needs a decision
Relates to:
- 11 (resolved): the text era's UTF8 variant unescapes the five XML entities (`unescape`, crates/ingest/src/text/
  mod.rs:127). It is the only entity policy in the tree. It was justified by the ISO twin of the same daily, which
  spells the content unescaped, so there the escapes were our serializer's artifact. It does not cover eForms, FTS
  or r208/r209, and there is no twin to prove anything there.
- 520 (done): u2-build-notes F28 recorded entities in text-era party names (`n&#39;a`) as "not fixed": `n&#39;a`
  folds to `n 39 a`, so a decode would change the input of every name helper. `O&Amp;K` is cited in 520's
  unit1-decision.md:127. Names carry entities too, not only titles and descriptions.
- `partyname.rs:3728–3737` (`award_mixed_segments`) decodes `&amp;` locally, for one rule only. That is the only
  other decode in the tree.
- 524 (ready-for-agent): an org's head name is written once at mint. A decoded name would not reach an existing
  org's head without 524's refresh.
- ADR-0001: the notice layer keeps the raw payload and its parsed form, and the canonical layer is derived. A decode
  therefore belongs in projection, not in the parse layer.
- ADR-0016: there is no text search, so the effect is on display, on `/v1/sql` text matches and, for names, on the
  `name_prefix` seek.

## Verify

    curl -s --max-time 20 https://tenders.zebreus.click/v1/tenders/8561858 | python3 -c "import json,sys; t=json.load(sys.stdin)['title']; print('&#' in t, t[:80])"

- **done**: `False Gara europea a procedura aperta telematica, in modalità multilotto, per l’affida`, OR a
  `## Decision` section here keeps text as published, with a /docs caveat line.
- **open (2026-10-14)**: `True Gara europea a procedura aperta telematica, in modalit&#224; multilotto, per l&#`

## Repro

Public GETs, 2026-10-14 (`/v1/tenders/{id}`, `texts[]` and `parties[]`):
- 8561112 (TED, 00458719-2026), description: `souscription &amp;amp; la gestion` — still twice escaped after our
  single decode, so the XML carried it three times.
- 8561120 (00484041-2026), lot descriptions: `épicerie bio&amp;lt;br/&amp;gt;` (HTML markup, escaped too). Its
  buyer, org 30965029, is served as `CENTRE COMMUNAL D&apos;ACTION SOCIALE DE BOLBEC`, as both `mention_name` and
  `organization_name`. An eForms org head name carries the entity.
- 8561167 (00460412-2026): `en phase &amp;quot;offre&amp;quot;`.
- 8561279 (00455590-2026), title: `l&#8217;affidamento … attivit&#224;`.
- 8561312 (00485551-2026), description: `&#61655;` (U+F0D7, a Symbol-font private-use bullet).

## What was found

**8561858** (TED eForms, notice 45886825, 00489010-2026):
- The title is "…in modalit&#224; multilotto, per l&#8217;affidamento…".
- `/v1/notices/45886825/content` already holds `&#224;` in BT-21-Procedure and BT-24-Lot, after roxmltree decoded
  the XML once. So the XML carried `&amp;#224;`: the publisher escaped twice. We do not skip a decode.
- BT-24-Procedure in the same notice is mangled differently: "modalità;". No decode can repair that one.

**Prevalence**, bounded reads of `tender_version_texts` by tender-id range (2026-10-14):

| tender ids | era | tenders | with `&#` | with a named entity | share of tenders |
|---|---|---|---|---|---|
| 8,561,000–8,563,000 | 2026 TED eForms | 1,897 (20,376 rows) | 64 rows / 26 tenders | 104 rows / 20 tenders | up to 46 (2.4%; the two sets may overlap) |
| 369,000–371,000 | 2025 eForms | 1,959 | 18 rows | 41 rows | 20 (1.0%) |
| 6,000,000–6,002,000 | legacy TED | 2,001 | 0 rows | 11 rows | 6 (0.3%) |

So 0.3–2.4% of tenders, depending on the slice. The named entities seen are `&amp;`, `&quot;`, `&lt;`, `&gt;` and
`&apos;`, sometimes nested (`&amp;amp;`). One decode pass would not clear them all.

**Names.** The same escapes reach party names: 520's text-era `n&#39;a` and `O&Amp;K`, and eForms buyer 30965029
above. A name is a binding input: `name_norm` is the bind key for identifier-less orgs (issue 234), and `&#39;`
folds to ` 39 `. Decoding names therefore changes keys, and needs the re-key path that 524 is building. That is why
the name decision is kept apart from the text one below.

Not measured: FTS, r208/r209 and DÖE slices; the share of org names; whether any slot carries a legitimate literal
`&…;` that a decode would damage.

### What the external review got wrong

- "An HTML entity left in a title" implies we skip a decoding step. We do not: the parse layer decodes the XML
  once, and the publisher escaped twice (or three times).
- It reads as a one-off. It is 0.3–2.4% of tenders by slice, and it reaches names as well as titles.
- Do not re-file the verifier's first draft either:
  - "about 2%" is the top of the range, not the rate;
  - its Verify strings were cut wrongly (`per l&#8` and `per l’affidamen` are not what `[:80]` prints);
  - its scope left out party names.
  - There is no `q=` search to retokenize (ADR-0016).

## Unit 1 — measured (2026-10-16, bounded windows)

`size_538.py` (session scratchpad; plan-checked: `tender_version_texts_version` range + rowid probe, organizations
rowid range). Fourteen 2,000-tender windows of `tender_version_texts`, joined to `tenders.source`. Nine 100k-id
windows of `organizations`. One read each, no 408.

**Texts.** Tenders whose title or description holds `&#…;` or a named entity (`&amp;`, `&quot;`, `&apos;`, `&lt;`,
`&gt;`; LIKE is ASCII case-insensitive, so `&Amp;` counts):

| window | era (probe) | TED tenders | desc tenders | title tenders | rows `&#` / named |
| --- | --- | --- | --- | --- | --- |
| 60,000 | eForms 2025 | 1,900 | 30 | 13 | 43 / 165 |
| 100,000 | eForms 2024 | 1,890 | 26 | 12 | 26 / 110 |
| 369,000 | eForms 2025 | 1,905 | 20 | 4 | 18 / 41 |
| 1,000,000 | eForms 2024 | 1,882 | 30 | 5 | 15 / 121 |
| 7,960,000 | eForms 2026 (+797 FTS, 27 DÖE) | 1,117 | 10 | 6 | 18 / 17 |
| 8,561,000 | eForms 2026 | 1,896 | 40 | 28 | 64 / 104 |
| 5,000,000 | r2.0.8 2013 | 2,000 | 3 | 4 | 19 / 11 |
| 6,000,000 | r2.0.9 2018 | 2,000 | 3 | 5 | 0 / 11 |
| 8,300,000 | 2013 | 2,000 | 1 | 0 | 0 / 1 |
| 2,000,000 / 3,000,000 / 4,000,000 / 7,000,000 | 1995 / 2003 / 2009 / 2021 | 2,000 each | 0 / 1 / 0 / 0 | 0 | — |
| 8,700,000 | FTS 2024 | 1,640 FTS | 1 | 0 | 0 / 2 |

Reading by era and source:
- **TED eForms carries the class:** 0.9–2.1 % of Tenders per window.
- **Legacy r2.0.8/r2.0.9:** about 0.2–0.35 %, mostly text-era `&Amp;`. That is a title-cased `&amp;`, in which the
  parse layer's single decode found no entity.
- **Old TED, FTS (OCDS JSON) and DÖE:** about 0. The DÖE Tenders in the windows (27–70 each) never hit.

The dominant shapes are:
- `&amp;lt;br/&amp;gt;`: double-escaped HTML line breaks. This is most French platform descriptions, and the first
  shape in 60092, 369157, 1000122 and 7960906.
- `&#8217;` / `&#8220;` / `&#224;`: Italian platforms (100083, 8561858).
- `&amp;amp;`: three times escaped (8561112).

No legitimate literal entity was seen in any example read. After the parse layer's own decode, a remaining `&amp;`
means the publisher escaped twice. FTS is JSON and has no entity layer, so a `&amp;` there is also a publisher
escape.

**Names** (`organizations.name` with the same patterns, per 100k-id window; `&#` 0 everywhere):

| window | orgs | named entity | examples |
| --- | --- | --- | --- |
| 0 | 24,307 | 7 | 2482 `Martin &amp; Servera …`, 5653 `Johnson &amp; Johnson …`, 19495 `… &quot;Bode Wipper&quot;` |
| 5,000,000 | 17,322 | 5 | text era `&Amp;` (5001545, 5012758) |
| 10,000,000 | 12,318 | 11 | text era `&Amp;` (10005239 `w.l. Gore &Amp; Associés`, 10007330 `iss H&Amp;P`) |
| 22,000,000 | 10,667 | 6 | 22023251 `Scholz &amp; Volkmer GmbH`, 22072601 `&quot;UNSER LAGERHAUS&quot; …` |
| 1M / 15M / 20M / 25M / 30M | 1,507–20,441 | 0 | — |

That is 29 of 123,490 (0.02 %); on the order of 1–2 thousand orgs corpus-wide. They are mostly `&amp;` and `&Amp;`
in company names (`Johnson & Johnson`, `Gore & Associés`). `name_norm` folds them to `amp`, so a decode re-keys
every one.

## Decision (2026-10-16, owner)

1. **Texts: option (a).** At projection, decode titles and descriptions (lot ones included). The parse layer
   stays as delivered (ADR-0001).
   - Decode HTML character references (decimal and hex) and the five XML named entities. The named entities are
     matched ASCII case-insensitively, so text-era `&Amp;` decodes too.
   - Repeat while one remains, bounded at 3 passes.
   - A decoded `<br>` / `<br/>` / `<br />` becomes a newline. No other markup is touched (none was seen).
   - A private-use code point (U+E000–U+F8FF, the Symbol-font bullet `&#61655;`) becomes `•`.
   - Mangled forms (`modalità;`) stay as they are: no decode can repair them.

   The full HTML named set is NOT accepted. Only the five names were seen, and `&nbsp;`-style names would widen
   the input a decode touches without evidence.
2. **Names: not in this issue's build.** The class is about 0.02 % of orgs. A decode changes `name_norm`, the bind
   key of identifier-less orgs (234), so it needs a re-key path. It also needs the head refresh to elect decoded
   spellings, and 524's refresh elects among mention spellings that are all escaped the same way. Recorded as a
   follow-up unit (unit 4 below), sized from this measurement, behind 524's wet runs.
3. **Build after 533 and 524 v10 leave the tree** (`crates/ingest/src/project.rs` is in 533's change). Drain sized
   from the carriers of the eForms text fields. 495's compare engine announces each changed row as a D5
   correction, so expect about 1–2 % of eForms Tenders to get a correction row.

## Proposed units

1. **Measure (S).** Run the same bounded `tender_version_texts` read over 4–5 more slices: TED eForms 2023 and
   2024, FTS, r209, DÖE, and the text era.
   - Count rows and tenders per field (title, description, other), split numeric, named, and nested (`&amp;amp;`,
     `&amp;lt;`).
   - Count markup (`<br/>` after decode) and private-use code points.
   - In the same id windows, do a bounded read of mention names and org head names carrying `&…;`.
   - Look for legitimate literals a decode would damage. Stay within `prod-box-reads.md`: index ranges with
     `LIMIT`, no scans.
2. **Decide (S), text and names separately.**
   - Texts, option (a): at projection, decode HTML character references (numeric, plus the five XML names; decide
     whether to accept the full HTML named set), repeating while one remains, bounded at 3 passes, in title and
     description. The parse layer stays as delivered (ADR-0001). Also decide whether a decoded `<br/>` becomes a
     newline, and what a private-use bullet becomes.
   - Texts, option (b): serve as published, with a /docs caveat line.
   - Names: decode or not, given the `name_norm` re-key and 524's head refresh. If names are excluded, record why
     here, so the next review does not re-file it.
3. **Build (M), if (2) picks (a) for texts.**
   - Pin it with `a_double_escaped_title_is_served_decoded`, using 8561858's title, 8561112's `&amp;amp;` and
     8561120's `&amp;lt;br/&amp;gt;`, and a "modalità;" case left unchanged.
   - Drain with `refold-fields` over the text fields. The 495 compare engine announces each changed row as a D5
     correction, so size the change rows from unit 1 before draining.
   - A names build, if decided, is its own unit behind 524's re-key path.
4. **Names (decided 2026-10-16: deferred, its own unit).** Decode mention and head names in the resolver, with a
   re-key of the identifier-less orgs whose `name_norm` changes (about 1–2 k orgs). Only after 524's wet runs.
