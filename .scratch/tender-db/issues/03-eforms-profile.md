# 03 — eForms mapping profile + completeness harness

Status: ready-for-agent
Blocked by: 02

Goal: eForms notices parse into the notice-parsed layer with the
mapped-or-ignored guarantee enforced by tests — the ADR-0002/0004 machinery
exists and is real.

Scope:
- roxmltree-based parser core: namespace-URI matching, exhaustive
  consumption bookkeeping (every element/attribute must be claimed by a
  mapping or an ignore rule; leftovers ⇒ quarantine the notice).
- Notice-parsed tables for the eForms core: procedure fields, lots (+
  LotsGroups, Parts-as-lots-with-kind), organization mentions + roles,
  amounts (cents+currency), texts (EN + original), classifications (CPV,
  NUTS), dates (UTC + offset), withheld-field satellite, id-refs.
- Mapping registry as data: field id → target, or documented exclusion
  (pointless BTs, OPP plumbing). Completeness test walks the pinned SDK
  fields.json (vendor the fields.json of the pinned SDK version into the
  repo) and fails on unaccounted fields. Multi-version: registry keyed by
  (sdk-version range); wild CustomizationIDs outside known range ⇒
  quarantine.
- Fixture notices (real, committed): one per major notice type (CN, CAN,
  corrigendum/change, PIN, veat, BRIN) — parse tests assert extracted
  values.

Acceptance: the fetched day's eForms notices parse with zero unexplained
quarantines; completeness test green against the vendored fields.json;
`cargo test -p ingest` covers fixtures.
