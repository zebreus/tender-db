# 73 — unparsable-xml quarantines 628K notices (format/era our parser rejects)

Status: resolved as DUPLICATE of issues 36+41 — no code change. Reclaim QUEUED 2026-07-29 as reprocess job [5] (reason=unparsable-xml, detail_like='XML with DTD detected', reclaim_only) in the reclaim-all sequence (ADR-0009); folds in the final rebuild.
Kind: completeness / data-quality
Relates to: 36 (XXE-safe DTD strip), 41 (internal-ojs parser), 71, 72, ADR-0004, CONTEXT.md (three TED format eras)

## Root cause (2026-07-29, proj-fix) — NO dispatch gap, NO code change

The 2008-05 members are the **OPOCE INTERNAL_OJS export** — the exact era issues
36 (XXE-safe `strip_doctype`) + 41 (`internal-ojs` parser) already fixed and
deployed (both ancestors of prod rev `498b928`). Proven empirically:

- Extracted a real member from `/data/archive/ted/monthly/2008-05.tar` on the VPS:
  `20080502_2008085.tar.gz/114238/opoce-input/114238_2008.en`. Its head is
  `<?xml …?><!DOCTYPE INTERNAL_OJS PUBLIC "…R2.0.5//EN" "Internal_Ojs.dtd"
  [<!ENTITY % TYPE 'EEIG'>]><INTERNAL_OJS …>`.
- Its **sha256 is byte-identical** (`02d96e9…`) to the committed fixture
  `crates/ingest/tests/fixtures/internal_ojs/114238_2008.en`, whose test
  `internal_ojs_english_member_dispatches_as_a_notice` **passes**: current
  dispatch strips the DOCTYPE, parses the body, and routes EN → an `internal-ojs`
  notice (non-EN langs → documented duplicate skips).

So the deployed parser handles 2008-05 correctly; the `XML with DTD detected`
quarantine rows are **stale** (created before issue 36 deployed 2026-07-21, never
reprocessed — the canonical build + issue-61 incident consumed the window). Same
shape as issue 72 (OC).

### Recovery is ~28k notices, NOT ~620K

The 619,965 count is quarantine **members across ~22 languages** (~28k notices ×
~22 langs). A reprocess reclaims only the **~28k EN notices** (issue 41's "whole
2008 coverage gap"); the other ~591k are non-EN siblings that correctly become
duplicate-skips, not notices.

### Reclaim path (team-lead ops step)

These are **profile-level** quarantines (failed at XML-parse, so NO `notices`
row exists) — unlike OC. Two ways to reclaim:

- Preferred, now that the reprocess mechanism exists (**issue 76**) — it reclaims
  the notices AND flags the stale rows, so the ledger is correct:

      POST /admin/jobs {"kind":"reprocess","reason":"unparsable-xml","detail_like":"XML with DTD detected"}

- Or a bare `process` re-run over the affected periods (works because it's
  profile-level, but leaves the stale rows unflagged):

      POST /admin/jobs {"kind":"process","source":"ted","package_kind":"monthly","period":"2008-05"}
      POST /admin/jobs {"kind":"project","rebuild":false}

Either way the EN members insert as fresh `internal-ojs` notices and the project
folds them; the ~591k non-EN siblings correctly become duplicate-skips.

## To investigate (original — superseded by the root cause above)

## Finding (2026-07-29 snapshot analysis)

`unparsable-xml` = **628,204 quarantined**, with NO profile assigned (they fail at the
XML-parse stage, before profile dispatch). Sample details:
- `unknown token at 1:1` / `unknown token at 1:3` — the payload doesn't start as valid XML our
  parser accepts (wrong encoding? BOM? not XML at all? a different tagged format?).
- `XML with DTD detected` — XML carrying a DOCTYPE/DTD that the parser rejects (likely an
  XXE-safety refusal).

## SIZED (2026-07-29 join to fetches)

- **621,863 (99%) are `XML with DTD detected`**; only 6,341 are `unknown token` (likely genuinely non-XML/corrupt, leave quarantined).
- **619,965 (~99%) come from a SINGLE package: TED `monthly` 2008-05.** The rest is a thin tail across 2004–2010 monthly (hundreds each). So this is essentially one early-XML historical package (DTD-bearing) that our XML parser rejects — NOT scattered corruption.
- profile.rs ALREADY has `strip_doctype` + XXE-safety (test at profile.rs:524/561), so the fix is to route the 2008-05-era XML through that path (strip the DOCTYPE, resolve NO external entities, parse the body). Investigate why 2008-05 quarantines at "XML with DTD detected" instead of going through strip_doctype — likely a dispatch/detection gap for this era.
- Recovery: ~620K notices from correctly handling one package's DTD. High value, concentrated.

## To investigate (original)

1. Join a sample of these quarantine rows back to their `fetches` (source, package_kind,
   period) to identify the ERA/format: is this the older TED_EXPORT XML (R2.0.8/R2.0.9) that
   carries DTDs, the tagged-text era mis-routed to the XML parser, or genuinely corrupt files?
2. If "XML with DTD detected" is a large share: TED's older XML legitimately declares a DTD;
   we can strip/ignore the DOCTYPE safely (resolve no external entities) and parse the body —
   recovering that share. Decide the safe approach (disable external-entity resolution, keep
   the internal subset).
3. If "unknown token at 1:1" is non-XML text-era files mis-dispatched: fix the profile routing
   so they go to the text parser instead of the XML parser.
4. Anything genuinely corrupt/truncated stays quarantined (correct per ADR-0004).

## Note

628K is the largest single quarantine bucket after SDK (71). Recovery share depends on how much
is DTD-XML (recoverable) vs corrupt (out-of-scope) — the investigation in step 1 sizes it.
