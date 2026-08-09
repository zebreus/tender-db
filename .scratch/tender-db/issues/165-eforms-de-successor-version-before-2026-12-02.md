# 165 — vendor the eForms-DE successor (2.2/3.0) before 2026-12-02

Status: open — DEADLINE-DRIVEN (external): eForms-DE 2.1 acceptance ends 2026-12-02
Role: run-driver

From the 2026-08-09 upstream-drift audit (docs/research/upstream-drift-2026-08.md).

DÖE's support table gives eForms-DE 2.1 / SDK 1.14 an acceptance window ending
**2026-12-02** (supported for validation to 2027-03-31). The successor is in
active development at projekte.kosit.org/eforms/eforms-de-specification —
version name still in flux (branches say both "3.0" and "revert-to-2.2"),
built on EU SDK 1.15 (codelist + schematron repos merged "apply changes from
eforms-sdk-1.15" on 2026-07-31). Known content deltas so far: BT-22 split into
BT-22 + BT-DEX-04, new rules BR-DE-37/38 (CVD/CPV), BT-DEX-05 (KMU suitability).

When it ships, notices will start declaring an eleventh CustomizationID our
`resolve()` does not know → they quarantine (the honest failure). To stay
ahead of that:

1. Watch the SDK-eforms-de repo (gitlab.opencode.de OC000008125155) for the
   release; vendor its fields.json (likely `eforms-de-2.2@eforms-sdk-1.15` or
   `eforms-de-3.0@...` keys, possibly two EU bases again).
2. Add the `resolve()` arm + completeness-test decisions; the DEX additions
   ride the ordinary kind-derived decision channel.
3. Also vendor EU `eforms-sdk-1.16` when final (~Sep 2026, per OP-TED roadmap
   — 1.16 is now the last 1.x, released in sync with SDK 2.0; the fields.json
   format break is deferred to SDK 3).

Trigger to act: first quarantined notice with an unknown eforms-de
customization, or the KoSIT release announcement — whichever comes first.
