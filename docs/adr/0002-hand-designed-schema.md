# The canonical schema is hand-designed, not generated from the eForms SDK

The eForms SDK ships machine-readable field metadata (fields.json), so
generating tables/columns for all ~1000 fields was a real option — complete by
construction and cheap to update per SDK release. We rejected it: a generated
schema mirrors eForms XML node structure, and the whole point of tender-db is
an *idiomatic* SQL shape (clean Tender/Lot/Bid/Organization aggregates) that
users can query directly, including via the public SQL endpoint. Every table
is designed by hand.

The "all business terms, no omissions" requirement is instead verified
mechanically: a test walks fields.json and asserts every field ID has an
explicit mapping (to a column, a satellite table, or a documented deliberate
exclusion such as the SDK's "pointless BTs"). Hand-designed does not mean
hand-audited.

Amendment (2026-07-19, from docs/research/eforms-data-model.md): the checklist
is per SDK version and per profile — field sets differ between SDK releases
(fields get removed, xpaths move) and national profiles (eForms-DE) add their
own fields, so completeness is asserted against each (version, profile) pair
the archive actually contains, not one fields.json.
