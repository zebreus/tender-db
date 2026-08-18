# 234 — an identifier-less mention mints a NEW provisional Organization every time, so legacy buyer rollups cannot aggregate

Status: needs-triage, RAISED — SIZED 2026-08-18: 23,462,294 of 24,618,292 organizations (95.30%) are
provisional, so this is the corpus's present state, not a risk the text-era re-parse would introduce
Kind: canonical identity gap (correct per-notice, useless per-organization)
Blocked by: —
Relates to: 232 (the text-era buyer fix that makes this bite at 3.79M scale), 04 (where provisional
organizations were introduced and measured at ~8 % of an eForms daily), 217-B (`name_norm` backfill —
the column that would make the fix possible), 62 (org index deferral), 172 (cross-era analytics
caveats)

## What

`store::Db::resolve_one_mention` resolves an Organization two ways:

- **With a usable identifier** — keyed `(country, identifier_kind, identifier)` through `org_of`, so
  every mention of the same legal entity converges on one `organizations` row. Correct.
- **Without one** — an unconditional `INSERT INTO organizations(… provisional, …) VALUES(… 1 …)`.
  No lookup, no key, no reuse. **Every mention mints a brand-new organization row.**

So two notices naming the same authority — byte-identical name, same country — become two
Organizations. A thousand notices become a thousand.

`name_norm` (the lowercased name) is written on every insert and there is even an
`organizations_name_norm_id` index, but nothing in the resolve path ever *reads* either. The material
for a name-based merge is already being produced and then ignored.

## Why it matters now

This has been latent and tolerable: issue 04 measured 1,216 provisional profiles out of 8,221 on an
eForms daily — ~8 %, a minority, and eForms carries BT-501 for the rest.

The legacy eras are the other way round. The text era (1993–2010, **3,786,955 tender-versions, ~27 %
of the corpus**) publishes **no organization identifier at all** — its entire field inventory is 36
codes and none of them is a registry number or VAT id (see issue 232 for the full list). So when the
text-era buyer fix (issue 232, `0074b61`) is landed by re-parsing that era, it will mint on the order
of **one provisional Organization per notice**.

The result would be a buyer that is *present* but not *aggregatable*: "MAIRIE DE PARIS" would appear
as thousands of distinct Organizations, so the buyer rollups and authority-level analytics that
motivated issue 232 still would not work. The field-completeness number would go green while the
capability stayed broken — which is the exact failure mode issue 230 exists to stop us celebrating.

## SIZED (2026-08-18) — and it has already happened, at scale

The acceptance item below asked for these counts "before re-parsing the text era, so the growth is a
prediction that gets checked rather than a surprise". They were free all along: the dashboard's
`tender_db_canonical_rows` gauges publish them, no query needed.

    total organizations      24,618,292
      canonical               1,155,998   ( 4.70%)
      provisional            23,462,294   (95.30%)
    organization_mentions    41,297,219
    provisional per mention        0.568

**95.3 % of the organization layer is provisional.** So this is not a latent risk that the text-era
re-parse would introduce — it is the corpus's present state. Issue 04 measured 1,216 provisional profiles
out of 8,221 (~15 %) on one eForms daily and treated that as the tolerable minority case; at full-corpus
scale, with the legacy eras included, the ratio is inverted almost exactly.

0.568 provisional organizations per mention is the number that names the defect precisely: mentions
that carry no usable identifier mint a fresh row nearly every time, so the "canonical Organization"
concept currently describes 4.7 % of the table. A buyer or winner rollup over the legacy corpus is
therefore not fragmented at the margins — it is mostly fragments.

Two consequences worth stating plainly:
- **This raises the issue's priority above the text-era re-parse it was filed alongside.** Issue 232's
  buyer fix would add ~3.8M more provisional rows (one per text-era notice), a ~16 % increase on top of
  23.5M — meaningful, but no longer the thing that "causes" the problem. The problem is here now.
- **The `name_norm` material already exists for all 23.5M rows** — every insert writes it, and
  `organizations_name_norm_id` indexes it — so a name-based merge has its input ready and unread. That
  makes option (1) below cheaper than it looks, and makes the fact that nothing reads it more glaring.

### How much would a merge collapse? Measured — and the answer is NOT one number

Four bounded 200k id-windows over provisional rows, each ~0.1 s (`id BETWEEN` rides the PK, so no scan
of the 24.6M table was needed):

    ids  2,000,000..    199,026 provisional →  70,050 distinct   2.84×
    ids  7,000,000..    198,383 provisional →  71,030 distinct   2.79×
    ids 12,000,000..    195,376 provisional →  56,226 distinct   3.47×
    ids 18,000,000..    194,796 provisional →  48,015 distinct   4.06×
    ids 23,000,000..     92,488 provisional →   6,432 distinct  14.38×

distinct = `COUNT(DISTINCT COALESCE(country,'?') || '|' || COALESCE(name_norm,''))`, i.e. what a
`(country, name_norm)` merge would leave.

**The ratio is not stable — it climbs from 2.8× to 14.4× across the id space**, so any single sample
extrapolated to the table would have been wrong, and I nearly recorded the 3.47× window alone as "the"
collapse ratio. Ids are AUTOINCREMENT and therefore roughly chronological, so the newest slice is over
14× more repetitive than the oldest. The plausible reading is that recent eForms notices name the same
few bodies over and over — issue 225 measured one review-body organization holding 1.79M party rows —
while older legacy eras name genuinely varied parties. That is a hypothesis about WHY, not part of the
measurement.

What this settles and what it does not:
- **Settles:** a name-based merge is not a marginal dedup. Even the most conservative window removes
  ~64 % of provisional rows, and the newest removes ~93 %. Option (1) is worth doing.
- **Does not settle:** the whole-table figure (needs a full pass or a stratified estimate — cheap now
  that the windowed shape is known), nor whether the collapse is CORRECT. Ratio measures how much would
  merge, never how much should: two distinct bodies sharing a name in one country merge here too, and
  the 14× window is exactly where that risk concentrates.

### The over-merge check: done, and it comes back clean

Largest merge groups in the 14× window (ids 23,000,000.., provisional, by `(country, name_norm)`):

     51,388  LU  Publications Office of the European Union
      3,433  IE  The High Court of Ireland
      2,647  CH  Bundesverwaltungsgericht
      1,210  CH  Tribunal administratif fédéral
      1,184  DE  1. Vergabekammer des Freistaates Sachsen bei der Landesdirektion…
        867  LU  Court of Justice of the European Union
        851  BE  European Commission
        809  MT  Public Contracts Review Board
        712  DE  Bundeskartellamt, Vergabekammern des Bundes
        677  DE  Bundeskartellamt - Vergabekammern des Bundes
        710  LU  Juridictions administratives
        552  ES  Tribunal Català de Contractes del Sector Públic

**Every one is a specific, identifiable institution — courts, review bodies, EU organs. Not a single
generic string** like "Contracting Authority", which is the shape issue 04's finding #3 warns about. So
on this evidence a `(country, name_norm)` merge is collapsing repeated mentions of ONE body, which is
exactly what it should do.

Two observations that make the case stronger, not weaker:

- **The merge is CONSERVATIVE — it under-merges rather than over-merges.** Rows 3 and 4 are the same
  Swiss court in two languages (`Bundesverwaltungsgericht` = `Tribunal administratif fédéral`), and rows
  9 and 10 are one German body differing only by comma-versus-dash. `name_norm` is lowercased but not
  punctuation- or language-normalised, so those stay separate. The collapse this merge achieves is
  therefore a **floor**, and the safe direction to err in.
- **A handful of EU-level bodies dominate.** 51,388 rows in a single 200k window are the Publications
  Office alone — plausibly because it appears on essentially every TED notice as the eSender/service
  provider (the `Procedure-SProvider` role seen elsewhere today), though that attribution is inference
  and not measured here. It suggests much of the 23.5M is a small set of ubiquitous parties, which is
  also why the newest window collapses hardest.

**Caveat kept honest:** this is the top 12 by size, which says nothing about the tail. A generic string
could sit at rank 50 and still merge thousands of unrelated bodies. Before implementing, run the same
grouping filtered to groups whose name is short or matches a generic pattern — cheap now that the query
shape is proven at ~0.1 s.

Conclusion for the decision above: **option (1) is both worthwhile and safe on current evidence**, with
punctuation/language normalisation as a deliberate LATER refinement rather than part of the first cut —
merging more aggressively is the step that would need its own evidence.

## What to decide

1. **Name-based merge for identifier-less mentions, scoped by country.** Look up
   `(country, name_norm)` before inserting; reuse on hit. `organizations_name_norm_id` exists;
   `(country, name_norm)` may want its own index. The risk is the mirror of issue 04's finding #3:
   over-merging two genuinely different bodies that share a name. Legal-entity names are not unique,
   so scope it by country at minimum and keep the row `provisional` so a later identifier can split it.
2. **Or: decide the merge belongs downstream** — leave one org per mention and expose a resolved-entity
   view built on `name_norm`. Honest, and it keeps the canonical layer free of a guess, but it moves
   the cost to every reader.
3. **Either way, size it first.** Count today's `organizations` rows and the provisional share before
   re-parsing the text era, so the growth is a prediction that gets checked rather than a surprise.

## Do NOT bundle this with the 232 re-parse

Tempting, because both touch the same era, but they are different claims with different risks: 232's
fix is a parse-layer mapping backed by six vintages of fixture evidence, while this is an identity
policy that can silently merge distinct entities. If they land together and buyer rollups look wrong
afterwards, there is no way to tell which change did it. Land 232, measure the org growth it causes,
then decide this with that number in hand.

## Acceptance

- A recorded count of `organizations` (total and provisional) before and after the text-era re-parse.
- A decision recorded here between (1) and (2), with the over-merge risk addressed explicitly.
- If (1): a red-checked test that two notices naming the same authority in the same country resolve to
  ONE organization, and that two same-named bodies in different countries stay separate.
