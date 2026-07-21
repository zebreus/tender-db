# 39 — Record today's standing decisions as ADRs

Status: resolved

Resolution (2026-07-21): ADR-0004 amended (three-class quarantine headline);
ADR-0005 amended (reader-pool topology + connection-budget note); new ADR-0007
(durable supervisor job queue, incl. the Spec serde compat contract); new
ADR-0008 (background dashboard refresher, with the rejected TTL-cache
alternative); ADR-0003 precedence wording reconciled to the code and owner
sign-off recorded; CONTEXT.md ambiguity cleared. No code changes.

The 2026-07-21 architecture review found four load-bearing decisions
that outran the ADRs, plus one wording reconciliation:

1. Amend ADR-0004: the quarantine headline metric is now the
   three-class split (actionable / suspected-gap / benign-by-evidence,
   model::quarantine_class) — the raw total stays visible but the
   headline is quarantine_actionable.
2. New ADR: durable supervisor job queue — restart-durability
   guarantee, at-least-once + idempotent re-walk semantics, resume
   cursor, and the on-disk serialized Spec forward/back-compat contract
   (unknown variants dropped, regenerable).
3. New ADR: background dashboard refresher — the request path can never
   scan; rejected alternative: request-path TTL cache (superseded today
   for cause: cold windows + no single-flight under write load).
4. Amend ADR-0005: reader-pool topology — one writer + four
   independently-sized reader pools over one file (store READ_POOL=8,
   app READERS=8, SQL_READERS, webhooks 2), the "reads never queue
   behind the writer" guarantee, and a note that no single owner tracks
   the process connection budget (accepted for now).
5. ADR-0003 wording: OWNER SIGN-OFF RECORDED (team lead, 2026-07-21,
   under transferred product authority): shared-field conflicts fold by
   PUBLICATION date with dispatch date as ordering fallback and
   source_rank as equal-instant tiebreak — i.e. what the projection
   implements. Update the ADR text to match the code and clear the
   "pending sign-off" flag in CONTEXT.md's Flagged ambiguities.

Acceptance: ADR files follow the existing docs/adr/ style (context /
decision / consequences, dated); CONTEXT.md ambiguity cleared; no code
changes.
