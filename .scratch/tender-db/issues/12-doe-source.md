# 12 — DÖE source: fetcher, eForms-DE + sdk-0.1 profiles, cross-source merge

Status: ready-for-agent
Blocked by: 04

Goal: oeffentlichevergabe.de is a live second Source and German procedures
merge across Sources.

Scope:
- Fetcher: monthly + completed-day exports (eforms.zip), T+1 schedule,
  registry rows; archive under /data/archive/doe/.
- eForms-DE profile: SDK-DE deltas (+4 fields, national codelists,
  E-subtypes, DE→EU version fallback table per
  docs/research/eforms-de-profile.md); DEX satellite table.
- sdk-0.1 profile: the committed empirical path inventory as checklist
  (numeric + uuid channels); numeric-channel notices become single-notice
  Tenders; unknown new paths quarantine + inventory extension workflow.
- ADR-0003 merge: exact notice/procedure UUID equality merges DÖE+TED
  Tenders; per-field-class precedence; merge is a projection concern
  (re-projection can undo).
- Fixtures from the VPS sample months.

Acceptance: one DÖE day ingests; a known DÖE↔TED pair (e.g. 373130-2026)
resolves to ONE Tender with TED publication identity + DÖE national codes;
below-threshold islands appear as Tenders.
