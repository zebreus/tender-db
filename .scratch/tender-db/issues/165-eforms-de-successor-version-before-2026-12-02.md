# 165 — vendor the eForms-DE successor (2.2/3.0) before 2026-12-02

Status: WATCH AUTOMATED 2026-08-21 (owner) — neither trigger has fired, and the "watch the repo"
half is now a machine's job: `tender-db-driftwatch`, a daily detection-only timer beside the
disk/job watches (ops/watchdogs/), probes the public SDK-eforms-de release feed and goes loud
(journal WARN + `systemctl --failed`) the day a release lands beyond the vendored 1.14.x line.
Installed and green on the box ("ok drift: newest SDK-eforms-de release is 1.14.4"). The
2026-08-21 state of the successor is recorded at the bottom; the vendor-and-resolve work (steps
1–3) starts when the watch fires. Was: open — DEADLINE-DRIVEN (external): eForms-DE 2.1
acceptance ends 2026-12-02
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

---

## Upstream state as of 2026-08-21 (checked via the public GitLab APIs)

- **SDK-eforms-de (gitlab.opencode.de OC000008125155, the artifact we vendor):** newest release
  **SDK-DE 1.14.4 (2026-07-22)** — still the eForms-DE 2.1 / EU SDK 1.14 line. Active but
  unreleased: `prepare/direct-flat` branch (2026-08-11).
- **Spec repo (projekte.kosit.org):** no tag beyond v2.1.0 (2024-12-19). Successor development is
  live on branches — `revert-to-eforms-de-2.2` (2026-07-11; the naming fight appears settled
  toward **2.2**), BT-DEX-05/KMU (2026-06-10), master last touched 2026-08-12 — but NOT released.
- **Schematron:** newest releases still serve the 2.0.0/2.1.0 lines (v0.9.7 / v1.0.5, mid-2025).
- **Codelist:** newest release v2024-12-20; the "apply changes from eforms-sdk-1.15" work of
  2026-07-31 sits unreleased on branches.
- **Prod trigger:** quarantine's `unknown-customization` newest arrival is 2026-07-29 (weekly DQ,
  section 5) — no unknown eForms-DE CustomizationID has arrived.

Read together: the successor is late-stage but unshipped, and with the 2.1 acceptance window
closing 2026-12-02 the release should land within weeks-to-months. The driftwatch turns that
uncertainty into a same-day signal; when it fires, bump `TENDER_DRIFT_KNOWN_PREFIX` as part of the
vendoring commit so the watch re-arms for the line after.
