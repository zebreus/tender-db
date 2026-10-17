# 564 — `name_prefix` no longer matches a renamed organization's former head, and `/docs` does not say so

Status: done (pending deploy) — 2026-10-17: the sentence is on all three surfaces (the `/docs` `name_prefix` row, the
`/docs` lookups section, the OpenAPI `name_prefix` parameter), pinned by
`the_docs_say_name_prefix_no_longer_matches_a_former_spelling`. It ships with the next code deploy; then run Verify.
Kind: docs (served API reference)
Relates to:
- 524 (the head refresh renamed 16,002 identifier-bearing heads in place on 2026-10-16: jobs 2565, 2567, 2568).

## Verify

    curl -s https://tenders.zebreus.click/docs | grep -o 'former[^<]*' | head -3

- **done**: the `name_prefix` row says that it matches the organization's served name only, and that after a head
  refresh a former spelling is matched no longer. The row points to `/v1/changes` (`organization changed`) for the
  history.
- **was (2026-10-17)**: the row reads "Organization-name prefix, Unicode case-insensitive … switches the list to
  name order". Only CHANGELOG.md (the 524 entry: "its first-seen spelling is matched no longer") states the
  behaviour.

## What was found

The audit read the code and probed the live API (64 GETs, all 200):
- `GET /v1/organizations?name_prefix=` range-seeks `organizations.name_norm` (crates/store/src/read.rs:5065, 5092)
  over `organizations_name_norm_id`. The rename's compare-and-set updates `name` and `name_norm` together
  (head_election.rs:1958, 2542), so a renamed org is found under its new key at once.
- 16 renamed orgs, 8 each from jobs 2567 and 2568, were all found by their NEW name at rank 1 or 2. Rank 2 is
  name-twin order: an org with a lower id carries the same name. Lower-case, all-caps and partial forms matched too.
- None of the 16 was found by its OLD name. Six searches returned nothing; ten returned only other orgs that still
  carry the old name.
- Detail pages, tender `parties[]` joins, the list sort and the `organization changed` change rows were all
  consistent. `mention_name` keeps the spelling the notice printed.

So the behaviour is designed and in the CHANGELOG, but a reader of `/docs` searching by a spelling they saw on an old
notice gets no hit and no explanation.

Not a defect, recorded for the record: `org_name_verdicts` is keyed by a bare NAME (single / generic / platform /
non-name), not by an org, so a rename does not detach a verdict from anything. Every renamed org carries an
identifier, so its new mentions bind by identifier, not by the country-less name reuse.
