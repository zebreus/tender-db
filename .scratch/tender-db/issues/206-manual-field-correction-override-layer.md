# 206 — no way to record a manual one-off correction to a field of a specific notice

Status: DECIDED 2026-08-21 (owner) — DEFERRED-BY-POLICY, no build now. Decision recorded at the
bottom: faithful-to-source + fix-in-code stays the deliberate stance; the override layer is built
the FIRST time a real single-notice correction is actually needed, in the vendored-as-data,
code-review-gated shape sketched here, correcting the projected value AND recording the source
erred (both halves of open decision 2), with the stale-override surfacing rule as a hard
requirement. Until that first real need, this issue is the design's home, not a work item.
Kind: architecture / data-quality
Blocked by: —
Relates to: 40 (quarantine ledger — the pattern to mirror), 173 (takedown/redaction — the same
"alter one notice in an append-only store" hole), ADR-0001 (deterministic projection), ADR-0004
(faithful-to-source ingestion).

## The gap

There is no mechanism to record a manual, one-off correction to a wrong value in a specific field of a
specific notice — e.g. a human spots a typo in one notice's title/value/date and wants it fixed and
tracked. Confirmed by reading the code:

- Notices are faithful-to-source (ADR-0004); the raw member is archived immutably. A source typo is
  preserved by design.
- Tenders/lots/orgs are DETERMINISTICALLY projected from notices (ADR-0001), so a hand-edit to a
  projected field is overwritten by the next rebuild/reprojection. `project.rs` states there is "no
  fact-level override here."
- The only "correction" concept in the code is `is_correction` — a correction NOTICE the *source*
  republishes under its BT-701 logical id, superseded automatically. Nothing for corrections *we* make.
- The quarantine ledger (`crates/app/data/quarantine-ledger.json`) is the closest audit-trail pattern,
  but it tracks parse-quality *categories* and corrects no field values.
- Issue 173 already records the sibling hole: no mechanism to alter/remove one notice from the
  append-only versioned store + tar archive + change log.

Today's actual "fix wrong data" path is class-level: fix the parser/projection in code → reprocess/
rebuild → correct values re-derive from source. That cannot express "this one notice, this one field."

## If we want it — the shape (not yet decided)

Mirror the ledger's philosophy (corrections are reviewable DATA, keyed to what they describe):

- A durable **override layer**: rows keyed by `(source, publication_id | notice_id, field path, lot
  key?)` holding `{ old_value, new_value, reason, author, at }`. Vendored-as-data (like the ledger, so
  a correction is a reviewable table edit) or a real table with an admin endpoint — decide by whether
  corrections are code-review-gated or operator-live.
- The projection **applies** overrides AFTER deriving from source, so they survive every rebuild and
  are the last word. An override whose `old_value` no longer matches the source (the source changed
  under it) should surface, not silently apply — otherwise a stale override lies.
- Emitted like any version change so `/v1/changes` and subscribers see it; auditable by construction.

## Open decisions for the owner

1. Do we even want manual overrides, or is faithful-to-source + fix-in-code the deliberate stance
   (a manual override is, by definition, the DB disagreeing with the archived source — a real cost)?
2. If yes: correction of the *projected* value only, or also a note on the *notice* that the source is
   wrong? The two answer different questions ("show correct data" vs "record that the source erred").
3. Governance: who may write one, and is it code-review-gated (vendored data) or operator-live (admin
   endpoint)? The quarantine-ledger precedent is code-review-gated.

No build until (1) is answered.

---

## Owner decision (2026-08-21)

**(1) Not yet — and not never.** Three facts decide it:

- **Demand is zero so far.** In two-plus months of operating this corpus — quarantine drains,
  re-parses, campaign sweeps, dashboards read daily — not one concrete "this one field of this one
  notice is wrong and must be hand-fixed" case has surfaced. Every wrong value found had a CLASS
  cause (parser gap, mapping alias, inventory mislabel) and the class fix corrected it everywhere.
  Building override machinery ahead of the first real case would be speculation with a permanent
  correctness cost attached.
- **The cost is not hypothetical.** An override is the DB disagreeing with its own archived source.
  It weakens the strongest property this system has (byte-faithful derivation, ADR-0001/0004) for
  every future migration, refold, and verification gate — the golden test, the epoch discipline,
  and issue 99's byte-identity all assume derive-from-source is total.
- **The gap has a cheap interim.** A source typo is FINDINGS material: it can be recorded on the
  issue board (as issues 131/132 recorded source-published negative amounts) without touching the
  layer. If a consumer-facing wrong value ever matters enough, that pressure IS the trigger below.

**Trigger to build:** the first genuine case where a specific notice's specific field is wrong, the
source will not republish, and the wrongness has a real consumer cost. When it fires, build:

- **Shape (decisions 2+3 pre-answered):** vendored-as-data, code-review-gated (the
  quarantine-ledger precedent) — corrections are reviewable table edits, not a live admin surface.
  Apply AFTER projection so overrides survive every rebuild. Correct the projected value AND mark
  the notice "source states X, corrected to Y, reason, author, date" — both what-to-show and
  that-the-source-erred, because consumers need the first and audits the second.
- **Hard requirement:** an override whose recorded `old_value` no longer matches the source
  surfaces loudly instead of applying — a stale override that silently applies is worse than the
  typo it fixed.
- Emit through `/v1/changes` like any version change.
