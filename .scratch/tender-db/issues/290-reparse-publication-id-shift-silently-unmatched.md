# 290 — a parser change that shifts `publication_id` derivation makes `reparse_notice` silently no-op (counted as benign `unmatched`)

Status: **RESOLVED 2026-09-16** — the content-hash fallback, the key adoption, the loud summary and the runbook all landed; see the foot. Taken off ANALYSIS because issue 394 unit 1 requires it settled first, and because "LOW confidence it ever bites" was wrong: 394's DÖE re-key is exactly the bite. Was: ANALYSIS (2026-08-26, owner — adversarial reclaim review; partly inherent to identity-keyed reparse; LOW confidence it ever bites)
Kind: correctness hazard (a future parser change could pass a re-parse as "complete" while a cohort keeps its old fold)
Severity: LOW (requires an uncommon kind of parser change; nothing current triggers it)
Relates to: 100 (the re-parse mechanism), 272 (reparse resume/dry-run operator traps)
Found by: the 2026-08-26 adversarial reclaim/quarantine review.

## The hazard

`reparse_notice` (lib.rs ~1693) re-keys its target by the NEW parse's
`(source, publication_id, content_hash)`. `content_hash` is stable (same
bytes), but `publication_id` is parser-EXTRACTED — if the very parser change
being re-folded also changes how `publication_id` is derived, the lookup misses
the existing notice row, `reparse_notice` returns false, and the record is
counted as `unmatched` (process.rs ~585), which the `ReparseReport` doc treats
as benign. The stale parsed layer under the OLD publication_id is never
replaced.

## Why only ANALYSIS

This is partly intrinsic to identity-keyed in-place reparse, and no current or
planned parser change moves publication_id derivation. The risk is the
REPORTING: `unmatched` is a non-event, so such a shift would not be noticed.

## Guard direction (cheap)

When a re-parse job finishes with `unmatched > 0`, surface the count
prominently in the job summary (it may already be there — verify) and document
in the reparse runbook: "an unexpected unmatched spike on a re-parse means the
parser changed identity derivation — stop and investigate, do not treat the run
as complete." Optionally: fall back to a `(source, content_hash)`-only lookup
when the exact triple misses and the hash is unique — the same fallback shape
issue 193 gave reclaim-attempt stamping.

## RESOLVED 2026-09-16 — the fallback AND the reporting, both

Status: **RESOLVED** (gate green, `GATE-EXIT=0`). Taken off ANALYSIS because issue 394 unit 1 needs
it settled first: its `## Done when` says "**290 is settled before that runs, not after**", since
re-keying 7,158 DÖE notices off the `00000000-1900` placeholder is precisely a derivation shift, and
so precisely the silent `unmatched` no-op this issue describes.

The "Guard direction (cheap)" listed three things and called the third optional. All three are done,
and the optional one turned out to be the substantive fix rather than a nicety.

### 1. The `(source, content_hash)` fallback — and the adoption that makes it terminate

`reparse_target` (`crates/store/src/lib.rs`) tries the full triple, then falls back to
`(source, content_hash)` when — and only when — that hash names **exactly one** notice of the source.
On the fallback, `reparse_notice` also **adopts the new `publication_id`** in the same transaction.

The adoption is not an extra: without it the row keeps a key the parser has stopped producing, every
later re-parse takes the fallback again, and the served `publication_id` stays wrong. With it, a
derivation shift is a one-time bridge — which is exactly the mechanism 394 unit 1 needs, delivered by
an ordinary re-parse rather than by a bespoke re-key job.

Safe by construction rather than by care: the identity is `UNIQUE(source, publication_id,
content_hash)`, so a second row already holding the NEW triple would share this row's source and hash
and would have made the fallback's count 2, which refuses. Two notices sharing bytes are left
unmatched and loud, never merged.

Not shared with the reclaim path, deliberately. A reclaim that finds no exact identity is looking at
a record the corpus does not carry under that key; minting is right there, and adopting a hash-twin
would merge two notices on the strength of duplicate bytes.

### 2. `unmatched` was already in the summary — as one number among four, which was the problem

Verified: `supervisor.rs` already printed `{unmatched} unmatched`. That is not surfacing it, because
`0 unmatched` and `4,812 unmatched` read the same at a glance and the `ReparseReport` doc calls the
counter benign. Now `reparse_notice` returns `Reparsed::{Replaced, Rekeyed, Unmatched}`,
`ReparseReport` carries `rekeyed`, and the summary appends a clause when either is nonzero:

- re-keyed only → `— NOTE: N re-keyed by content hash, so this run CHANGED publication_id derivation
  (issue 290). Intended for a re-key run; a regression otherwise`
- unmatched only → `— CHECK: N unmatched. On a run that expected few, that is the issue-290 shape …
  those notices KEPT THEIR OLD PARSE. Do not read this run as complete until the count is explained`
- both → names both and says which half recovered

At zero, both stay ordinary numbers in the line. The operator does not have to know to look.

### 3. The runbook

`docs/operations.md` § "Sizing a `reparse`" gains the mechanism, a two-row table of what each counter
means and what to do, and the narrowness of the fallback.

### Tests

In `crates/store/src/lib.rs`'s suite, extending the existing re-parse test rather than adding a
parallel one, so the ordinary and the shifted paths are read side by side:

- the `stranger` case now uses DIFFERENT BYTES as well as a different key, because with the same
  bytes it is the new `Rekeyed` case — the split is the point;
- same member, same bytes, new key → `Rekeyed`, the stored `publication_id` is the NEW one, and
  `COUNT(*) FROM notices` is still 1 (adopted, not duplicated);
- re-parsing again → `Replaced`, pinning that the fallback is a one-time bridge and not a permanent
  second lookup.

### What this unblocks

394 unit 1 can now change the DÖE `publication_id` election and re-parse the 7,158 carriers: they
have 7,158 DISTINCT content hashes (394's own measurement), so every one satisfies the fallback's
uniqueness condition, the re-parse re-keys them, and the job summary reports the count so it can be
checked against 7,158 rather than assumed.

## PROVEN LIVE 2026-09-16 — job 1388, the first real derivation shift

The fallback and the loud summary were exercised end to end by issue 394 unit 1's re-key, on rev
`6612b2b`, three DÖE packages:

    re-parsed 37145 notices across 3 packages (64549 members walked, 0 unmatched, 701 re-keyed,
    5 now failing and left untouched); stamped 246265 tender(s) epoch-stale; 85 package(s) held
    back by the cap — continue with {"after": 403} — NOTE: 701 re-keyed by content hash, so this
    run CHANGED publication_id derivation (issue 290). Intended for a re-key run; a regression
    otherwise

- **701 re-keyed** — 701 notices whose identity the new parser derives differently were found by
  their bytes, re-parsed, and had the new `publication_id` adopted.
- **0 unmatched** — nothing was lost to the shift, which is the whole point. Before this issue, all
  701 would have landed in `unmatched` and the run would have reported a clean re-parse over a cohort
  that kept its old parse.
- The NOTE clause fired and says the right thing for a re-key run.

Arithmetically confirmed against the stored rows, not just the counter: the placeholder cohort went
**7,177 → 6,476**, a drop of exactly **701**. The counter and the corpus agree.

### One thing this run exposed that is NOT covered, recorded here rather than assumed away

`now_failing` counts a PARSE-level quarantine (`reparse_package` matches `Record::Notice` and then
`Parse::Parsed`). A **dispatch**-level quarantine — the record never becoming a `Record::Notice` at
all, which is exactly what a botched identity guard would cause — is neither counted nor reported: it
is silently skipped by the walk. So the reporting this issue fixed covers the case where identity
derivation MOVES, and not the case where it DISAPPEARS.

That gap is real but it is not this issue's; the cheap guard for it is the same one 394 already
relies on — the cohort count must reach **0**, not "some small number", and a residue is the signal.
Filed here as the note a future reader needs; if a run ever shows a cohort that stops shrinking, this
is where to look first.

(The `5 now failing` in this run are parse-level and pre-existing — the guard changes only the
publication-id election, which is a dispatch-stage decision and cannot reach the eForms parser.)
