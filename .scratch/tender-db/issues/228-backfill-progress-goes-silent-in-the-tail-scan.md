# 228 — a chunked backfill's progress counter goes silent for its whole tail scan, and reads as a hang

Status: FIXED on main 2026-08-17 (owner), awaiting deploy — and the fix found a latent
CORRECTNESS bug beside the observability one, see "Resolution" at the bottom. Filed while
operating the issue-58-v2 step-2 backfill; it cost 40 minutes of operator doubt on its first
real run.
Kind: operational rough edge / job observability
Blocked by: —
Relates to: 58 (the legacy-adjacency backfill this surfaced on), 53 (`/metrics` — the trend surface
that would have answered it), 226 (freshness clock only counts ingest kinds), 224 (job watchdogs)

## What happened

`backfill-legacy-adjacency` (job 1, enqueued 08:39 UTC) drove `members_done` to **10,950,000** by
09:22 and then did not move it again for over 40 minutes, with the queue idle and nothing in the
journal. From the operator surfaces alone — `tender-admin jobs`, `journalctl` — a completed-but-not-
recorded job, a deadlocked job, and a job doing exactly what it was written to do are
**indistinguishable**.

What it was actually doing is correct and knowable only from the code
(`ingest::project::backfill_legacy_adjacency` + `store::Db::legacy_parsed_chunk`):

- the chunk query pre-filters `AND (profile = 'text' OR profile = 'internal-ojs' OR profile LIKE
  'ted-export%')`, and `LIMIT` counts only **matching** rows;
- so once the cursor passes the last legacy notice, the query cannot return early — it scans every
  remaining row to prove no legacy row is left. On prod that tail is ~3.3M eForms notices;
- the sweep loop calls `progress(swept)` **once per returned chunk**, and that final query returns
  exactly one chunk (empty), which breaks the loop. So the counter's last update is the last legacy
  chunk, and the entire tail scan is silent.

It took an out-of-band `/proc/<pid>/io` sample to establish liveness: 1,246 MB read in 45 s with
**zero** bytes written — a pure read sweep finding nothing to insert, which is the tail scan's exact
signature. That is the right conclusion, but an operator should not have to reach for `/proc` to
distinguish "working" from "wedged", and the next person (or the next firing) will not.

## Why it matters beyond this job

The result was never in doubt — the watermark is established from `max_parsed_notice_id()` on
success, so a completed run is self-verifying. The cost is entirely operational, and it is the shape
that gets a real hang missed: once "flat counter for 40 minutes" is known to be normal for this job,
the same reading during an actual deadlock earns the same shrug. Compare issue 226's finding, which
is the same lesson from the other direction: a signal that cannot distinguish two states is not a
signal.

## Fix (pick one; they are not exclusive)

1. **Report the cursor, not just the count.** `progress` currently carries `swept` (legacy notices
   found). Carrying the sweep's `cursor` — or reporting both — makes the tail scan visible: the
   number keeps climbing through 11M → 14.2M even when no legacy row is found. Cheapest, and it
   fixes the class rather than this instance. Needs a supervisor progress field that is not
   `members_done` (or an honest re-reading of that field's meaning).
2. **Bound the sweep.** Take `MAX(id)` over legacy notices once, up front, and stop the loop there —
   the tail scan then never runs at all, saving ~3.3M rows of I/O per backfill as well as the
   silence. Note the correctness caveat: the bound must be re-read (or deliberately fixed) if the
   backfill is ever run concurrently with ingestion of new legacy rows, which today it is not.
3. **Log a heartbeat per chunk.** A journal line per chunk (cursor, rows, elapsed) makes any long
   job legible without touching the progress protocol. Weakest of the three — it only helps someone
   already tailing the journal — but it is three lines.

Recommend **2 + 1**: the bound removes the wasted scan, the cursor report removes the ambiguity for
every future chunked job of this shape.

## Acceptance

A long-running chunked backfill shows *movement* on `tender-admin jobs` throughout, including any
phase in which it finds nothing to write; and an operator can tell a working job from a wedged one
without reading `/proc`.

## Resolution (2026-08-17, owner)

Fixed by **windowing the id range**, which is option 2 done properly — not by precomputing
`MAX(id)` over legacy notices (that would have paid the same full scan up front, merely moving the
silence to the start), and not by option 1: carrying the cursor in `members_done`/`members_total`
would put an id where every other job puts a count, and mixing the two in one field pair is the
kind of dishonest signal this issue is complaining about.

Each query is now bounded on **both** axes: at most `ADJACENCY_BACKFILL_CHUNK` matching rows
(memory, unchanged from the issue-42 shape) and at most `ADJACENCY_BACKFILL_ID_WINDOW` = 500k ids
scanned (I/O). Termination becomes "the cursor reached the target captured up front" instead of "a
chunk came back empty", because an empty chunk now means only *no legacy notices in this window* —
the normal state of every eForms-era window. Progress therefore ticks through the tail, and the
wasted tail I/O is gone as a side effect.

**The latent correctness bug the fix exposed.** Terminating on an empty chunk was not merely quiet.
Had the pre-filter ever returned an empty chunk mid-corpus, the sweep would have stopped early and
then called `establish_legacy_adjacency(target)` anyway — publishing a watermark that attests
coverage up to the target while rows above the stopping point were never written. Step 3's closure
walk trusts that watermark to decide it may scope a legacy fold, so the failure would not have
surfaced as a missing row; it would have surfaced as a silently under-scoped projection. Today's
corpus cannot hit it (the legacy eras sit in a dense low-id range and the eForms tail is contiguous
above them), which is why the run in flight is trustworthy — but "the data happens not to trigger
it" is not a property anyone should have to re-establish.

The new test is shaped for exactly that: `legacy → three eForms → legacy`, so the second legacy
notice is found only if the walk crosses the gap. Red-checked against the old loop — it fails on
the progress assertion (one tick where the corpus spans five windows) and passes with the fix. The
window is a parameter so the test can force one window per id over a 5-row corpus; production
always takes the default.

Deploy note: the run in flight (job 1, enqueued 08:39 UTC) is on the OLD code and is being left
alone — restarting it to pick up the fix would discard ~1.5 h of sweep and re-establish nothing.
Its result is trustworthy for the reason above. The fix lands with the next deploy, and the
verification that matters is the NEXT backfill of this shape showing movement throughout.
