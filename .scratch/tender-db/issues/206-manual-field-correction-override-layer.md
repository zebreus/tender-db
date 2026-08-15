# 206 — no way to record a manual one-off correction to a field of a specific notice

Status: open — design question, filed 2026-08-15 (owner, prompted by Lennart's question "do we have a
way to track corrections to wrong data, like a typo in a specific field of a specific notice we found
manually?"). Needs an owner decision on WHETHER we want this before any build.
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
