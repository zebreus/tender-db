# 296 — tender document/PDF attachments (metadata-only today)

Status: BACKLOG (filed 2026-08-26; CONTEXT.md "Metadata only: the PDF/document
attachments of tenders are out of scope for now"; spec non-goal)
Kind: capability (out-of-scope-by-decision; revisit trigger = user demand)

The corpus references procurement documents (specs, contract drafts) by URL; we
store no attachment content. Any future support is a storage + fetch-policy
question first (the raw archive already measured multilingual TEXT at ~86% of
size; documents would dwarf it), and a legal one second (third-party hosted
content, takedown surface — see ADR-0012). Do not start without a measured demand
signal; when started, begin with link-liveness metadata (which URLs still
resolve), not content mirroring.
