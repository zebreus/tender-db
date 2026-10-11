# 514 — spellings of one register number (label text, linked-charity suffixes) split one registry identity

Status: ready-for-agent — triaged 2026-10-11 with issue 469 U2/U3's evidence. NEXT: unit 1, measure the standing
spelling pairs per scheme (two orgs holding two spellings of one register number).
Was: needs-triage — filed 2026-10-11 from issue 469's unit-2 decision (decision 4: spelling repair stays out of the 469 arm).
Kind: data quality (organization identity)
Relates to: 469, 470 (the GB O/0 company-number lookalike repair, the precedent)

## What is wrong

FTS register literals carry label pollution (`GBUKPRNUKPRN10007798`, `GBCHCCHARITYNUMBER216250`) and glued
linked-charity suffixes (`216250-1`), so one register number can mint several identities. 469's arm refuses them
(its shape gate) rather than repairing them.

## Proposed unit

Measure the spellings per scheme on the 469 dry plan's literal shapes (U0), then decide a per-scheme canonical form
behind the name gate, as 470 did for GB company numbers.

## 469 U2/U3 evidence (2026-10-11)

The spellings are real. U3's shape gate keeps them apart, never folding them:
- **MPR.** The FCA's own number for a 2014-Act society is bare (`7639`, Ongo Homes). Companies House mirrors it as
  `RS007639`, and FTS publishers use both (`GB-MPR-RS007639` in the U2 sample). Two identities, one society.
  - Bare and `R`-suffixed are NOT spellings of each other: `7522` is Thirteen Housing Group, `7522R` an allotment
    society (FCA list). A canonical form may map `RS0NNNNN` to bare `NNNN`, and nothing else.
  - `IP16243R` is Companies House's spelling of FCA `16243R`. The sample's `ip6243r` is a typo of it and stays
    refused.
- **NIC.** `NIC 100095`, `NI 100770` and a bare `100095` are three identities for CCNI numbers (U3 accepts all
  three as published).
- **UKPRN / CHC.** Label pollution (`UKPRN10007798`, `CHARITYNUMBER216250`) and glued linked suffixes (`-1`) are
  refused, never stripped.

A unit-2 canonical form must ship with a 345-style re-key repair (`publishes()` compares raw strings), behind the
name gate as 470 did.
