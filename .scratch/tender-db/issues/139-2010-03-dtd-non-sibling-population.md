# 139 — The 1,898 "XML with DTD detected" rows from ted/monthly/2010-03: a non-sibling population needing its own investigation

Status: FIX LANDED (2026-08-12) — deep-parse DTD strip + reclaim dual-addressing, tests green; awaiting deploy + reprocess to drain
Filed: 2026-08-05 (team-lead, from proj-fix's 9fe744c + sdk-vendor's d294901/137)
Blocked by: nothing (independent of #29's execute — excluded from it by construction)

## What these are

1,898 quarantine rows, `reason = unparsable-xml`, `detail LIKE 'XML with DTD detected%'`,
**all from a single fetch: ted / monthly / 2010-03**. Surfaced by the #29 remainder
attribution (harness `6268b798`, results `9fe744c`, two independent runs agreeing):
they are the entire non-English remainder between the outstanding DTD measure
(594,915, issue 137) and the 2008-scoped marker population (593,010).

## Why they are NOT part of #29's duplicate-skip resolution

- **Different packaging shape, not merely a different month.** Their member paths
  end `.xml` with **no language code** — the suffix extraction returns `xml` for
  all 1,898, where every 2008-corpus row yields a 2-letter language. So there is
  no sibling structure in the paths.
- The duplicate-sibling evidence justifying the 592,856 marking was established
  **over the 2008 TED monthly corpus only**. It does not extend here — and the
  sibling question may not even be *coherent* for these files.
- The #29 marker's scope requires a 2008 TED monthly fetch, so these 1,898 are
  excluded **by construction** and cannot be mis-marked. They stay outstanding.

## How to investigate (start here, not with a sibling test)

**First establish what these files ARE** — read a few members from the 2010-03
archive (fetch located via `v_fetches`) and characterize the packaging: why no
language code? A different TED export format? A partial/transitional archive?
Do NOT start by re-running the sibling test: a null result there would read as
"no duplicates" when the true reading is "the question doesn't apply".

Per [[verification-input-discipline]]: a predicate verified over corpus X is
silent about everything outside X, and that silence reads exactly like absence —
this population was invisible to every earlier read precisely because every
prior query was scoped to 2008.

## Where they are surfaced meanwhile

Ledger (f803475): listed under "Known populations, not resolved" — named,
distinct, no resolution date (deliberately `Option`; a date would have filed
unresolved work under "Resolved").

## Size

~1,898 rows / one fetch period. Small, bounded, not urgent. The right shape is
one bounded characterization read (archive members, immutable source) when
someone has context for it.

## Comments

**2026-08-11 evening (orchestrator) — DIAGNOSED from the archive (no DB needed).** The 1,898
`.xml` members all live in ONE daily: `03/20100310_2010048.tar.gz` (2010-03-10, OJS edition
048/2010) — the only daily in the month NOT shipped as per-language text zips. It contains
exactly 1,898 per-notice XML files (`00070853_2010.xml`, publication-id names). Each is a genuine
`TED_EXPORT` document (DOC_ID/EDITION attributes, `<COMMENTS>From Convertor</COMMENTS>` — TED
converted this day to XML) carrying `<!DOCTYPE TED_EXPORT SYSTEM "TED_EXPORT.dtd">`, which
roxmltree refuses wholesale — the same defect class as 2008/issue 36. These are REAL notices held
whole; the era's r208 walker should read them once the DTD line is stripped. Fix: extend the
issue-36 DTD strip to the ted-export dispatch path (verify why job [5]'s reclaim didn't already
recover them — the strip may be scoped to internal-ojs), add a fixture from this edition, then
reprocess the bucket: +1,898 notices for 2010. Open question, one bounded DB read when a token
exists: the coverage grid shows exactly 1,898 held r208/2010 notices — same number; establish
whether that population is this edition already recovered via another path (then the quarantine
rows are stale bookkeeping, the 2008 pattern) or a coincidence.

**2026-08-12 (orchestrator) — why job 610 didn't drain the bucket: TWO defects, both fixed.**
Job 610 (`reprocess unparsable-xml / detail "XML with DTD detected"`) reported 1,905 still held
(unparsable-xml 1,898 + unclaimed-content 7) and changed nothing visible. Root causes:

1. **The DTD strip was dispatch-only.** `profile::dispatch` strips the DOCTYPE to find the root,
   so the members dispatch as notices — but `r209::parse_payload` fed the RAW bytes to the deep
   parse, which re-quarantined every one with the identical reason/detail. (internal_ojs already
   stripped; r209 didn't.) Fixed: same XXE-safe `strip_doctype` before `parse()` in
   `crates/ingest/src/r209/mod.rs`. Verified against the real archived member
   `20100310_48/00070853_2010.xml` via the diag139 example: now PARSED, 7 sections / 159 values.
2. **The failed attempt split the member's state, and later reclaims addressed the wrong half.**
   Job 610's no-notice branch inserted notice rows (state quarantined) while the original
   profile-level quarantine rows kept `notice_id` NULL (the parse-level insert hits
   UNIQUE(fetch_id, member_path, content_hash) — same raw-bytes hash — and is ignored). Every
   subsequent reclaim then takes the notice-exists branch, whose `WHERE notice_id = ?` matches
   zero rows: still-held stamps went into the void, and even a SUCCESSFUL re-parse would have
   left the ledger rows outstanding forever. Fixed: both the success path and `stamp_still_held`
   now also address `(fetch_id, member_path, notice_id IS NULL)` — disjoint from the notice_id
   address by construction, so attempts are never double-counted. Regression test
   `reclaim_reaches_profile_level_rows_after_a_failed_attempt_created_the_notice` walks the
   exact three-pass history (profile-quarantine → failed reclaim → failed reclaim → success).

The 7 `unclaimed-content` rows are the same story: their current failure was stamped nowhere.
After deploy + reprocess they either reclaim (strip fixes them too) or relabel truthfully.
Remaining: deploy, re-run the DTD-bucket reprocess, verify the panel (1,898 → reclaimed;
the 154 sibling rows stay correctly outstanding per issue 84's parsed-original guard unless
their originals now parse), then close.
