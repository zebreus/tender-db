# 234 — an identifier-less mention mints a NEW provisional Organization every time, so legacy buyer rollups cannot aggregate

Status: needs-triage — found 2026-08-18 reading `resolve_one_mention` while fixing issue 232
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
