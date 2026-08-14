# 139 — The 1,898 "XML with DTD detected" rows from ted/monthly/2010-03: a non-sibling population needing its own investigation

Status: RESOLVED-VERIFIED (2026-08-12) — 1,898 reclaimed into the tender layer, 7 relabeled unclaimed-content, ledger row outstanding 0 / reclaimed 1,898 on the live panel
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

**2026-08-12 mid-morning (orchestrator) — the reprocess with both fixes (145277a) RECLAIMED the
notices but STILL didn't stamp the ledger rows; third defect isolated to the addressing, loud
diagnostics added.** Job 612 (deployed rev 145277a): "1898 reclaimed, 7 still held
(unclaimed-content)". Notices verifiably flipped: the trailing fold picked up a change set, and
no new notice ids were inserted (REST id-tip check) — so the 1,898 went through the
notice-exists reclaim arm against notice rows that have existed since JULY. That rewrites the
issue's own history: the coverage grid's "1,898 held r208/2010" were notice rows all along —
the July monthly ingest already dispatched these members (the dispatch strip predates the
backfill), so the quarantine rows are PARSE-level (notice_id set), not profile-level as the
2008 analogy suggested. And 2010 is a text-era year (text/2010 = 389,496/391,397 ≈ 99.5%), so
the 1,898 r208 notices are largely language-twin content of text notices — the coverage panel
never showed a 2010 hole.

Yet the panel still reads outstanding 1,898 / by_reason unparsable-xml 8,246 (only the
flag-pass's 154-sibling sweep moved — see issue 190, filed): every quarantine UPDATE in the
reclaim (by notice_id AND by member address) matched zero rows, on both fetches, while
`flag_skipped_members`' member IN-list matched fine. Static analysis exhausted: each branch
that could have run implies a stamp that didn't happen. Rather than guess a fourth time, landed:
(a) `stamp_reclaimed` helper — both addresses, `eprintln` to the journal whenever a reclaimed
member resolves ZERO ledger rows (names notice id, fetch, member — the datum this hunt lacked);
(b) the already-parsed reclaim arm now resolves the member's ledger rows too (it used to stamp
nothing, so after job 612 NO re-run could ever converge the ledger); (c) same zero-stamp logging
on the fresh-record path. Next: after the fold completes, deploy, re-run the bucket, read the
journal line for one member, fix the real addressing mismatch, drain. (Row-level DB access
remains unavailable: /v1/sql token minting is classifier-blocked, no snapshot exists, and the
serving DB is never queried directly per prod-box policy.)

**2026-08-12 ~11:3x CEST (orchestrator) — CLOSED. There was no third defect: job 612's stamps
had landed all along, and the "unchanged" panel was a stale read snapshot.** The verification
re-run (job 619, rev 53d4b06) listed only fetch 217 in the bucket — fetch 195's 1,898 rows no
longer match (unparsable-xml, DTD, unreclaimed) — walked 593,010 sibling declines and found
0 reclaimable, 0 still held, with ZERO zero-stamp journal lines. The panel I had read at 09:37
served numbers matching the DB at exactly 08:55 (post-610-flags, pre-612): `measured_at` was
fresh while the data was pinned — the dashboard's reader held a read snapshot from the 08:55
service restart, the same pinned reader that kept the fold's WAL checkpoints `busy=true` for
two hours. Filed as issue 191. After today's restart the panel converged precisely:
unparsable-xml 8,400 → 6,341 (−1,898 reclaimed, −7 relabeled, −154 skipped/issue 190),
unclaimed-content 6,173 → 6,180, the 2010-03 ledger row outstanding 0 / reclaimed 1,898. The
fold (job 613) carried the reclaims into the tender layer: 7,927,854 tenders (+1,974 over the
epoch-3 baseline), 5,920 versions. Final accounting for the bucket: 1,898 real 2010 notices
reclaimed (language-twin content of a year text already covers at 99.5% — never a coverage
hole, now honest bookkeeping); 7 `.en` originals truthfully relabeled unclaimed-content (issue
183's population); 154 protected siblings swept to skipped-by-policy by the unguarded flag pass
(issue 190 owns the guard + repair). The observability (zero-stamp journal lines) and the
already-parsed-arm ledger stamping stay — they turn any future recurrence of this shape into a
one-journal-line diagnosis.

**2026-08-14 ~03:1x CEST (orchestrator) — four false alarms from job 654 diagnosed; guard
landed (rev 0a2acc7).** The COR drain fired 4× "[store] reclaim stamped NO ledger rows
(fresh record path)" over a fully consistent ledger. Shape: a correction file's records
mostly duplicate the earlier daily (job 654: 1,234 already-parsed), so the FIRST duplicate
resolves the whole-file rejection row via the already-parsed arm's opportunistic stamp;
the 4 genuinely-corrected records that follow are fresh identities whose stamps then find
nothing — zero-stamp, alarm. Verified on the box: all 22 not-utf8 ISO_COR rows terminal
with job-654's timestamp (2× EN reclaimed, 20× sibling-language skipped). Fix: the fresh
record path now applies the parsed arm's member_file_resolved benign-zero guard before
warning (store commit "benign-zero guard for the fresh-record reclaim stamp"); a true
stranded ledger still alarms. Regression test
corrected_records_after_a_resolved_file_row_are_a_benign_zero_stamp.
