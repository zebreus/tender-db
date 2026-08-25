# Takedown tombstones: a dormant design, deliberately not built

**Status:** accepted 2026-08-25 (project owner, under the standing mandate) as a
DORMANT design note — no machinery exists and none is to be built until an actual
obligation arrives. This closes issue 173's remaining half.

## Context

The architecture assumes sources never retract published notices: the canonical
layer is append-only and versioned (ADR-0001), the fetch archive keeps every
package byte-exact forever, and the change log is promised forever (`changes` is
never trimmed — issue 70's `oldest_cursor` fix leans on exactly that invariant).

Two dated boundary facts frame this note:

* 2026-08-23 (Lennart, direct): the lawyer confirmed all stored information is
  public; data-protection law is not a concern for this system and is CLOSED as
  a topic. The standing rule — never re-introduce privacy-driven design — stands.
* This note is therefore NOT about privacy. It is insurance against a different
  event class: a source-side redaction or a court order that obliges removing
  one published notice. Probability low; cost of having no plan when it lands
  with a deadline: high.

## Decision

Build nothing now. If a takedown obligation ever arrives, implement the design
below rather than improvising ad-hoc deletion against three stores that were
never meant to forget.

## The design (what would be built, in order)

The unit of removal is a notice (equivalently: an archive member). Obligations
arrive as "this publication must go", never as "this Tender".

1. **`tombstones` table — the order of record.** One row per taken-down notice:
   `(notice_id, content_hash, member_path, package, authority_reference,
   stamped_at, completed_at)`. The `content_hash` is load-bearing: it is what
   prevents resurrection (step 6) and what D4 consults (step 5). Rows are never
   deleted; `completed_at` is set when every step below has run.

2. **Parsed layer.** Delete the notice's parsed rows and satellites, and stamp a
   quarantine-style ledger entry (reason `tombstoned`, detail = the authority
   reference). Completeness surfaces then show an explained hole, not silent
   loss — the same honesty discipline as ADR-0004's quarantine.

3. **Canonical layer.** Re-fold only the affected Tender (the issue-99
   projection-epoch machinery already forces a targeted rewrite); its versions
   re-sequence — ADR-0001 already declares the change log, not `seq`, the stable
   spine. A Tender whose only notice is tombstoned retires through the existing
   removal path (issue 164), visible to SSE subscribers like any retirement.

4. **Change log — append, never redact.** `changes` rows carry
   `(entity_kind, id, seq, op, at)` and NO content (verified in
   `canonical.rs`), so history needs no rewriting: the re-fold appends ordinary
   `removed` rows and the log stays append-only. Should a future change-row
   shape ever carry content, THAT feature must add a trim path first — and the
   documented `oldest_cursor` coupling (issue 70 F1) is the one place that then
   needs a real watermark.

5. **Archive.** A tar.gz member cannot be deleted in place; the package is
   repacked once — stream-copy every member except the tombstoned one — and the
   tombstone row records both hashes (original and repacked). **Standing
   coupling:** D4 (`rehash-probe`) must treat a hash mismatch on a package with
   a tombstone row as EXPECTED, not DRIFT. Today that check is vacuously true
   (no table); it becomes real in the same change that creates the table.

6. **Re-arrival guard.** The daily walk-forward may re-download the original
   package (sources do not reliably redact their archives). Ingest dispatch
   drops a member whose content hash matches a tombstone row, counting it in
   the ledger (the issue-180 policy-skip pattern) — without this, step 2 undoes
   itself on the next fetch.

7. **Snapshots and backups.** Reflink snapshots and any off-box copies made
   before the takedown still hold the content; the tombstone row's completion
   checklist includes regenerating or deleting them. `completed_at` is set only
   after this.

8. **Verification.** Acceptance is positive absence: the content hash appears
   nowhere in DB, archive, or snapshots; the ledger documents the action; D4's
   next pass over the repacked package reports expected-mismatch, not drift.

## Consequences

* Per-takedown cost is bounded and small: one package repack, one Tender
  re-fold, one snapshot regeneration. Standing cost is zero — no code runs
  until an obligation exists.
* Two couplings must stay true when the design is ever built, and are recorded
  where they live: D4's drift classifier (step 5, noted in issue 173) and
  `oldest_cursor`'s trim coupling (step 4, already documented in issue 70).
* Nothing in this note licenses privacy-driven removal; the trigger is an
  external legal obligation with an authority reference, recorded verbatim in
  the tombstone row.
