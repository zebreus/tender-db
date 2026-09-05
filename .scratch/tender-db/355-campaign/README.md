# Same-name cross-border review campaign — the tooling (issue 355, cohort `xb-country-2026-09-05`)

The first campaign through the verdict-gated country path: the 487 cases of issue 311's
re-cut cohort (one normalized name, rows under several country codes), read per case by a
reviewer and an adversarial challenger per 20-case stratified batch, blind second readers on
every 9th case. Record: `../355-xb-country-verdicts.json`; numbers in the issue file.

- `rubric.md` — the v3 rubric these agents read (the 357 campaign's `rubric-v3c.md` is its
  successor, extended for the identifier-keyed spray shape).
- `split.py` — cuts the `xb-country-cases` packet (`xb-cases.json`, keyed by `root`) into
  20-case batches stratified by identical/different identifiers × how many rows agree with
  their own country, plus every-9th-case sample files.
- `post.py` — joins by root set, prints agreement statistics, applies the first floor set
  (agreeing row, weight without any anchor, row with standing, arithmetic without an anchor,
  shared register, shared checksum family, third country on soft evidence) and writes the
  `POST /admin/country-verdicts` body.

The workflow script was the 357 campaign's `review.js` with `root` in place of `identifier`
(see `../357-campaign/`). Run order is the same as the 357 README's step 5.
