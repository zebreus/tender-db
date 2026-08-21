# 234 — an identifier-less mention mints a NEW provisional Organization every time, so legacy buyer rollups cannot aggregate

Status: IMPLEMENTED 2026-08-20 (owner); the merge is DEPLOYED and is **prevention only** — the
epoch bump that rode the commit was WRONG and was reverted 2026-08-21 (see "The epoch correction"
below). Remaining deliverable: the `merge-provisional-orgs`
backfill job (IMPLEMENTED 2026-08-21, see "The epoch correction" below) — deploy, dry-run read,
execute, re-measure. Option (1) as decided: `(name_norm, country)` reuse for
identifier-less mentions, scoped `identifier IS NULL`, nameless and country-less mentions never
merge, rows stay provisional. Red-checked test + the issue-104 golden fired on it (details at the
bottom). Was: needs-triage, RAISED — SIZED 2026-08-18 at 95.30 % provisional
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


## The text-era campaign is now adding to this, measured (2026-08-19)

Not a projection any more. `MAX(organizations.id)` was **26,047,161** after 28 packages of issue 244's
campaign, against the 24,618,292 rows counted on 2026-08-18 — so roughly **1.43M new organization rows in
one afternoon**, from ~550k re-parsed notices. Every one of them is provisional: of the rows above id
26,000,000, all 47,161 sampled carry `provisional = 1`, and they are exactly the era's parties —
`MINDEF/DGA/DCE/CEG, MINISTERE DE LA DEFENSE`, `ONIC (OFFICE NATIONAL INTERPROFESSIONNEL DES CEREALES)`,
`WHITECHAPEL ART GALLERY`.

Extrapolating the same rate over all 215 packages: **~11M more provisional organizations**, taking the
table from ~24.6M to ~35M and the provisional share from 95.3% to about 97%.

Two things follow, and they point the same way:

- **This does not argue against the campaign.** The awards and buyers it extracts are real published facts
  that were previously absent entirely; one row per mention is the correct per-notice answer, and issue 232
  and 244 are both worth having. What grows is the cost of NOT having a name-based merge.
- **It moves this issue from latent to load-bearing.** A name-based merge applied afterwards has ~35M rows
  to collapse rather than ~24M, and every month of delay adds more. The material is all there: every row
  above carries `name_norm`, and `organizations_name_norm_id` is built.

The cheapest useful next step is unchanged — measure how much the corpus would actually collapse under a
`(country, name_norm)` merge before designing one — but the number to measure it against has moved, so
measure it after the campaign rather than before.

## The tail check the issue asked for — run 2026-08-20, and it changes the conclusion

The over-merge section above ends: *"this is the top 12 by size, which says nothing about the tail. A
generic string could sit at rank 50 and still merge thousands of unrelated bodies. Before
implementing, run the same grouping filtered to groups whose name is short or matches a generic
pattern."* Done. Same `(country, name_norm)` grouping, filtered to SHORT names, over three windows.

**Newest window (ids 23,000,000.., the 14× one):**

     3,433  IE  the high court of ireland          487  PL  krajowa izba odwoławcza
     2,647  CH  bundesverwaltungsgericht           261  DE  vergabekammer des bundes
       851  BE  european commission                219  DE  finanzbehörde
       …                                           100  DE  beschaffungsstelle

**Older windows (ids 12,000,000.. and 2,000,000..):**

     2,051  --  (empty name)                     1,428  NL  (empty name)
     1,021  FR  tribunal administratif           1,517  FR  tribunal administratif
       674  FI  markkinaoikeus                     564  SE  tendsign
       552  DK  klagenævnet for udbud              442  SE  förvaltningsrätten

Three over-merge classes the head check could not see, in descending seriousness:

1. **Empty names.** `name_norm = ''` at 1,953 / 398 / 2,051 / 5,077 per 200k provisional window
   (0 in the two newest). A `(country, name_norm)` merge collapses every nameless row in a country
   into ONE organization. That is the worst available over-merge and it is not marginal.
2. **Class names denoting many real bodies.** `tribunal administratif` (France has ~42),
   `förvaltningsrätten` (Sweden has 12). Real over-merge, bounded to bodies of one kind.
3. **Generic role nouns.** `beschaffungsstelle` ("procurement office"), `finanzbehörde`. Genuinely
   unrelated bodies sharing a common noun.

**So the recorded conclusion — "option (1) is both worthwhile and safe on current evidence" — was
half right and I should not have written the second half from the head alone.** Worthwhile: yes,
unchanged. Safe as stated: no.

### But the harm is bounded, and measured rather than assumed

The obvious next question is whether classes 2 and 3 touch the rollups this issue exists to enable.
They do not. Roles held by the merge groups in question:

    tribunal administratif  ->  review-body 1,035 | APPEAL_PROCEDURE_BODY_RESPONSIBLE 40
                                ADDRESS_MEDIATION_BODY 39 | … — zero buyer, zero winner
    beschaffungsstelle      ->  Procedure-SProvider 100 — the eSender, not a buyer

Every one is a review, appeal, mediation or eSender role. **Not a single buyer or winner row.** So a
`(country, name_norm)` merge over these fragments the *procedural addressee* layer and leaves buyer
and winner aggregation — the capability issue 232 wanted — untouched. That is a real answer to the
over-merge risk rather than a hope, and it is why classes 2 and 3 do not block option (1).

### Class 1 is not a merge-policy question at all — it is issue 259

Chasing the empty names to their origin found the mechanism, and it is a defect rather than a
publisher gap: the legacy vocabulary declares both `WINNER` and `ADDRESS_WINNER` as `Rule::Org`, so
one party opens two Organization sections — an empty wrapper (which the award references) and an
inner one carrying `OFFICIALNAME` (which nothing references). The nameless rows are **awarded
contractors whose name we had and dropped**: in one sampled window, 2,411 of their party rows are
`ADDRESS_CONTRACTOR` and 153 are `winner`. Filed and fixed as issue 259.

That removes class 1 from this issue's scope entirely: after 259's refold the nameless population
should approach zero, so there is nothing left for a merge to catastrophically collapse.

## Decision — option (1), with one guard and one sequencing constraint

**Merge identifier-less mentions on `(country, name_norm)`, and never merge a nameless one.** The
empty-name guard stays permanently even after 259 drains the population: it is one condition, it
costs nothing, and "every party we failed to name in this country is one organization" must never be
representable, whatever produces such a row next.

Punctuation and language normalisation stay OUT of the first cut, as the issue already argued — the
Swiss court appearing as both `bundesverwaltungsgericht` and `tribunal administratif fédéral` means
the merge under-merges, which is the safe direction.

**Sequence: 259 first, then this.** Not because they conflict, but because 259 changes the input: it
removes the class-1 population and reduces the row count that this merge would otherwise be measured
against. Landing them together would make the "how much did it collapse?" number unattributable —
the same argument this issue already makes for not bundling with the 232 re-parse.

### Implementation note for whoever picks this up

`org_of` is preloaded once per run and holds only identifier-bearing organizations (1.16M — see
`Db::mention_resolver`). A name map cannot work the same way: 23.5M `(country, name_norm)` keys will
not sit in RAM. So the lookup has to be a per-mention indexed SELECT, and
`organizations_name_norm_id` is on `(name_norm, id)` — it can seek by name but then filters country
row by row, which is fine for a rare name and bad for `tribunal administratif` at 1,500 rows. Add
`(name_norm, country)` and take the first hit. Net cost is probably NEGATIVE: it replaces an
unconditional INSERT with a seek that hits 3–14× of the time, and it stops writing tens of millions
of rows.

## Acceptance, restated against what is now known

- [x] A recorded count of `organizations` (total and provisional) — 24,618,292 / 23,462,294 (95.30%).
- [x] A decision between (1) and (2), with the over-merge risk addressed explicitly — option (1),
      above, with the risk measured by role rather than argued.
- [ ] The red-checked test: two notices naming the same authority in the same country resolve to ONE
      organization; two same-named bodies in DIFFERENT countries stay separate; and a nameless mention
      never merges with another nameless one.
- [ ] Land 259's refold first and re-measure the provisional share before implementing.

## Implemented (2026-08-20) — option (1), conservative on every edge the evidence flagged

`resolve_one_mention`'s identifier-less arm now probes `(name_norm, country)` — with a per-run
lazy cache on the resolver, since 24.6M keys cannot preload the way `org_of`'s 1.16M do (issue 57),
and one review body alone was 51,388 mentions in a 200k window — and reuses the hit instead of
minting. The probe rides a new deferred index `organizations(name_norm, country)`; the
`(name_norm, id)` index could seek the name but then filtered country row by row, a scan for
`tribunal administratif` at 1,500 rows.

The edges, each from this issue's own measurements:

- **Nameless mentions never merge** — a window's nameless rows included 2,411 awarded contractors;
  they are distinct unknown parties, and merging every nameless org in a country would be
  corpus-scale cross-linking.
- **Country-less names never merge** — that is where the platform strings concentrate
  (`tendsign` with NULL country, 369 rows in one window).
- **The probe is scoped `identifier IS NULL`** — a name matching a CANONICAL organization does not
  capture it; promoting by bare name is the over-merge this issue declines. Two bodies can share a
  name with only one of them registered.
- **The reused row stays `provisional = 1`** — the merge is a reuse policy, not a promotion; a
  later identifier can still split or canonicalise it.

Gate: `an_identifierless_mention_reuses_its_named_organization` (store) — same name+country → ONE
org (case-folded); same name other country → separate; country-less and nameless → never merged;
durable across resolver runs (a fresh resolver reuses through the table probe, not the cache); an
identifier-bearing mention keeps its own row and the probe still finds the provisional one.
Falsified: with the probe disabled it fails `left: 1, right: 2`.

**The issue-104 golden fired on this, first change since it landed.** `project_golden` went red —
identifier-less mentions in its corpus now merge and later surrogate ids renumber — which is
exactly the epoch-discipline prompt it was built to give: the regeneration + `PROJECTION_EPOCH`
3→4 ride this same commit. And the snapshot's epoch header earned its keep within a minute of
that: the first regeneration attempt silently missed the bump (a patch text-mismatch), and the
header reading `3` atop a fold-changing diff is what caught it.

## The epoch correction (2026-08-21) — the bump was wrong, and why

The plan below said the epoch bump's whole-corpus rewrite "IS the deliverable." Its first live
outing falsified that: job 293 (the post-deploy full projection) walked **0 notices → 0 tenders**,
because an epoch bump creates no plan delta by itself — and forcing the walk would have achieved
nothing either. The mechanism: `MentionResolver`'s `(notice, section)` idempotency preload returns
the already-recorded Organization for every mention that exists in the layer, so a re-fold of a
stored chain NEVER re-resolves its mentions — output is byte-identical with or without the merge in
the build. That is the exact definition of "stored chains remain valid state keys," i.e. the
condition under which PROJECTION_EPOCH must NOT move. Reverted to 3 in `71553d1`; the doc comment
on the constant now carries the lesson.

Consequence: the shipped merge is **prevention** — it stops NEW mentions (fresh ingests, re-parses)
from minting duplicates. It cannot retro-collapse the stock of 23.46M provisional rows via any
fold. That collapse is the **`merge-provisional-orgs` admin job**, implemented 2026-08-21:

- `Db::merge_provisional_organizations_batch` (store): one ordered pass over the
  `organizations_name_country` index (EQP-gated, per the planner's record — 239/248/256), groups
  `(name_norm, country)` adjacently in Rust, keeps each group's minimum id, repoints
  `organization_mentions` / `tender_version_parties` / `tender_version_bid_parties` /
  `tender_version_result_winners`, deletes the losers. Winners' PK ends in `organization_id`, so a
  lot_result already naming the survivor DROPS the loser's row instead of colliding
  (`winner_dups`). Losers get `removed` change rows, survivors `updated`.
- Batch boundaries are NAME-aligned (a cut-mid-name trailing name is dropped and resumed; a name
  bigger than the batch is refetched unbounded), so no group is ever split. Restart-safe without a
  durable cursor: merged groups leave the scan's scope as singletons.
- Supervisor: `merge-provisional-orgs` kind, `dry_run` default true (destructive-safe-default
  convention), in `STOPPABLE_KINDS` (stop honoured between batches), TRUNCATE checkpoint per
  batch, refuses with a remedy when `organizations_name_country` is missing (deferred index —
  `reindex` builds it; already built on prod by job 292).
- Gates: `the_org_merge_backfill_collapses_and_repoints_every_reference`,
  `the_org_merge_walk_advances_name_aligned_batches`,
  `the_org_merge_scan_walks_the_name_index_in_order` (store).

## Run plan + verification still owed

1. ~~Deploy~~ DONE 2026-08-21 (rev 0913989, after the data-quality run finished).
2. ~~Dry run~~ DONE 2026-08-21, job 295 (~5 min for the full scan): **890,199 duplicate groups,
   18,258,668 provisional orgs to remove** — 77.8 % of the provisional stock, org table
   24.6M → ~6.4M rows. STRONGER than the windowed 2.8×–14.4× prediction (≈21.5× mean within the
   duplicate set), which is expected: windows cannot see cross-window duplicates, and the heavy
   names recur across the whole corpus.
3. Real run LAUNCHED 2026-08-21 ~02:05 UTC as job 296 (`dry_run:false`); scope 20,270,188 rows.
   Stoppable between batches; restart-safe from `''`.
4. Re-measure once 296 completes: total/provisional organizations (was 24,618,292 / 23,462,294
   = 95.30 %) and `provisional per mention` (was 0.568). Record here; then close.
- Interim, prevention-only signal: the provisional-per-mention ratio for mentions recorded AFTER
  the deploy should sit far below 0.568.
