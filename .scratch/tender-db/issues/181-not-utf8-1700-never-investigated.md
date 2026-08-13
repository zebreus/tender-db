# 181 — the 1,700 `not-utf8` rows were flagged suspected and never investigated

Status: RESOLVED-VERIFIED (2026-08-13) — CF delivery drained across three buckets: 1,417 new notices reclaimed, ~135k records deduped, 2,206 non-EN/meta files policy-skipped; not-utf8 terminal at 208 (early-1999 members + ~21 rows to name), unknown-root 0, unparsable-xml 4,441 non-CF residue
Kind: data-quality investigation (suspected-gap bucket)
Blocked by: —
Relates to: 30 (classified it SuspectedGap rather than assumed-benign), 137 (measured: 1,700 rows, 0 reclaimed)

## Why

`quarantine_class` deliberately flags `not-utf8` as `SuspectedGap` — "small uncertain reasons are
flagged too rather than assumed benign" (issue 30). That flag is a promise to investigate, and
nothing has: 1,700 rows, zero reclaimed, no issue, no diagnosis, since the reason first appeared.
It is the largest still-held population on the dashboard with no owner at all
(public `/api/dashboard`, 2026-08-10: still-held `not-utf8` = 1,700).

## What

Sample the bucket (bounded `/v1/sql` + archive bytes, the issue-141 method) and answer:

1. What encoding are they actually in? Legacy text-era files in Latin-1/CP1252 would be real
   notices recoverable with a transcode step — a parser fix and a reclaim.
2. Or are they binary/truncated members — benign-by-evidence, to be reclassified with the reason
   carrying the proof (issue 30's bar)?

Either way the outcome gets a ledger row; a bucket this old should not be answerable only by
re-deriving it.

## Comments

**2026-08-11 (orchestrator) — strong lead from the 189 sweep**: the whole 1993–2000 text era
ships as `*_ISO_ORG` zips — ISO-8859 encoded by naming convention, confirmed by inspection
(byte 0xE4-style umlauts in the SV file). The text profile evidently decodes these fine in bulk
(1999 ingests 100%), so the 1,700 `not-utf8` rows are the residue where decoding still fails —
plausibly odd single bytes or a different legacy codepage. Sampling needs row-level member paths
(/v1/sql token still pending), but the era context makes fix-and-reclaim (transcode) the likely
outcome rather than benign-by-evidence.

**2026-08-13 ~14:0x CEST (orchestrator) — DIAGNOSED via /v1/sql + archive read.** The bucket
splits: ~1,492 rows are `_CF1` members (1999–2007, every language, e.g.
`EN_20030124_017_ISO_CF1.ZIP`), 187 are early-1999 full-edition-named members, ~20 stragglers.
A CF member's content is the ORDINARY text-record format (TI/PD/ND tagged records) — REAL
notices, not metadata. Mechanism: `text_era_member` requires the 5th name part to be `ORG`, so
CF members fall past the text dispatcher into the XML path, whose UTF-8 check rejects their
ISO-8859 bytes wholesale → profile-level not-utf8. Spot check: CF record ND 12572-2003 is
already held and parsed via the SAME daily's ORG file (`…ISO_ORG#0`) — CF appears to republish
the ORG delivery, so the reclaim should be mostly already-parsed dedup with the rows resolving
as reclaimed/known-duplicate rather than suspected loss. Fix shape: accept the CF variant in
`text_era_member` (keeping the EN-only language policy and ISO/UTF8 variant selection),
dispatch through the text-record parser, reprocess the bucket; the 187 early-1999 rows need
the same look with the early naming. Next firing implements.

**2026-08-13 ~15:1x CEST (orchestrator) — scope tripled, fix landed.** The CF story covers not
just this bucket: CF-path rows are ALL 753 unknown-root rows (meta_cf members, XML-ish `part`
root → now the meta-variant policy skip) and 1,900 of unparsable-xml (UTF8 CF members: valid
UTF-8 plain text fed to roxmltree) — ~4,112 rows across three reasons from ONE dispatcher gap.
Fix: `text_era_member` accepts the `CF<n>` delivery class; `PackageContext` tracks the EN-UTF8
twin PER CLASS so an ISO companion is only superseded by a UTF8 companion; all text policies
(EN-only, meta skip) apply unchanged. Tests cover the name shapes and class-aware supersedence;
the real 2003 member dispatches into 1,008 records with zero quarantines. Drain sequence once
the queue frees: deploy → reprocess not-utf8 + unknown-root (reclaim_only) → reprocess
unparsable-xml with one trailing fold. Expect mostly already-parsed dedup (CF republishes ORG)
with any genuinely-new records reclaimed; non-EN CF rows go skipped-by-policy.

**2026-08-13 late (orchestrator) — VERIFIED terminal.** Post-fix re-walks (jobs 644/645, rev
6f86b6b): zero zero-stamp journal lines; the already-parsed arm resolved every CF file row via
the new ordinal-stripped address. Panel: not-utf8 1,700 → 208, unknown-root 753 → 0,
unparsable-xml 6,341 → 4,441 (non-CF residue); overall outstanding 22,288 → 10,440 since the
campaign started. The CF drain's totals: 1,417 genuinely-new notices reclaimed (records the
companion files carried beyond ORG), ~135k records confirmed already-held, 2,206 non-EN/meta
files skipped by policy. Residue here: 187 early-1999-named members (record-tally) vs 208 rows
— the ~21-row delta needs one naming query (next audit slot); the early-1999 members themselves
are the remaining sub-population (different name shape, same record format — a candidate
follow-up fix if their content proves recoverable the same way).
