# 116 — tender detail reports more lots than it ships

RESOLVED-VERIFIED on prod (2026-08-10, orchestrator, deployed in 6ed1b0f):
GET /v1/tenders/7161565 now serves lots=2604 with 2,604 unique lot_details
(was 1,000), 2.0MB in 153ms warm. The completeness cap and cursor behavior
are gate-tested; prod confirms at the tender that exposed the bug.

Status: fix landed on main (2026-08-09, orchestrator) — awaiting deploy + the prod verification
below. Exactly the post-115 shrunken form this issue prescribed: `lots_of` now passes
`TENDER_LOTS_CAP` (20,000 — 549f8f5's constant and doc reasoning) instead of `MAX_PAGE`; the
in-memory retain/truncate from 549f8f5 was NOT taken (dead weight post-115 — the containment
shape serves `l.id > ?` from the Tender's own slice in SQL). Test
`a_tender_scoped_lots_read_is_complete_and_still_honours_the_cursor` red-checked against the
unfixed code (failed 1000 vs 1200 at the truncation assertion), completeness asserted through
the real `tender_detail` path. Store suite 66 green, app suites green.
Remaining: on the next deploy, run the prod verification (`GET /v1/tenders/7161565` →
len(lot_details) == lots == 2604, cursor equivalence, warm timing).
Kind: correctness (API honesty)
Blocked by: — (115 landed on main `3485e3d`, deployed b56ce13 2026-08-09)
Blocks: —

## Observation

`GET /v1/tenders/{id}` answers with a body that contradicts itself. For tender 7161565:

```
"lots": 2604            <- the Tender's real lot count (from the tender row)
"lot_details": [ … ]    <- 1000 entries
```

Nothing in the response says the array was cut. A client that reads `lot_details` believes it has the
Tender's lots; a client that reads `lots` is told a different number. There is no `next`/cursor on the
detail response either — `lot_details` is not a page, it is presented as the set — so the missing 1,604
lots are simply unreachable through that endpoint.

Scale: 16 Tenders of 4.26M carry more than 1,000 lots (max 2,604); 424 carry more than 500. So the
defect is confined to a tiny tail, but for those Tenders the detail response is wrong, silently.

## Mechanism

`read::lots_of` (read.rs) asks for a Tender's lots by reusing the *paginated list* scope:

```rust
lots(conn, &filter, Scope::Page { after: 0, limit: MAX_PAGE }).await
```

`MAX_PAGE` is 1,000 — the hard ceiling on a page of the global `/v1/lots` stream. It is the right
bound for a cursor-paginated list and the wrong bound for a containment question ("which Lots belong
to this Tender?"). The count beside it comes from a different source, so the two disagree.

Same impedance mismatch as the latency bug fixed in `1830d50`: "all lots of ONE Tender" was answered by
reusing the machinery for "the next page of a global list". That commit fixed the *cost* of the
mismatch; this issue is the *semantic* half, still open.

## Intended fix

Commit `549f8f5` on `lots-tender-scoped-read` ("a tender's lots are a bounded set, not a page of a
stream") is the fix, already written and NOT shipped:

- a tender-scoped read fetches the Tender's complete lot set under `TENDER_LOTS_CAP` (a sanity bound
  well above the corpus maximum, not a page size), so `lot_details` matches `lots`;
- `/v1/lots?tender=&after=` keeps exact cursor semantics by applying `after`/`limit` to the complete
  set in memory;
- the unfiltered `/v1/lots` stream keeps the plain cursor and `MAX_PAGE`, untouched.

## Why it is blocked (do not ship 549f8f5 yet)

**Corrected 2026-08-03.** This issue was first filed saying the 1,000-row cap was load-bearing as a cost
bound, and that lifting it to 2,604 would cost ~2.6× more. run-driver measured that on the box and it is
**false**: sweeping the page size over a faithful 2,604-lot fixture gives 16.25s / 16.19s / 16.37s /
16.05s at LIMIT 125 / 250 / 500 / 1000 — flat across an 8× change. The top-level sorter consumes the
whole result set before `LIMIT` applies, so every request already pays for all 2,604 lots. The cap
truncates the *answer* without bounding the *work*.

So the truncation is not buying anything. It is pure loss: the response is wrong AND slow.

The sequencing still holds, for the remaining reason: shipping `549f8f5` on the unfixed read would
serve 2,604 lots from a read that takes tens of seconds — turning a wrong-but-slow response into a
right-but-slow one, and leaving the resource-exhaustion surface untouched. Land 115 first and the
honest answer costs milliseconds.

Sequence: land 115 (drive the containment read from `tender_version_lots`, batch the satellites)
→ ship the bounded-set change → this issue closes.

### 115 makes this fix SMALLER than `549f8f5`

`549f8f5` was written against the old query, where any `l.id > ?` term forced the 13.2M-row walk. It
had to work around that: drop the cursor from SQL, fetch the whole set under `TENDER_LOTS_CAP`, then
re-apply `after`/`limit` **in memory** to keep pagination exact. That machinery exists only to avoid
emitting a predicate that used to be catastrophic.

After 115 (`2751ce3`) the tender-scoped read drives from `tender_version_lots` and `l.id > ?` is just a
cheap filter on an index-driven set — measured linear, 0.0169s at 2,400 lots. So the in-memory cursor
is dead weight. What is left of this issue is the actual defect: **`lots_of` passes `MAX_PAGE`**, a
global-stream page size, to a containment question. Replace that one argument with a sanity bound above
the corpus maximum (`549f8f5`'s `TENDER_LOTS_CAP`, and keep its reasoning for why that is a guard
rather than a page size) and the response stops contradicting itself.

Take `549f8f5`'s doc comments and its cap constant; **drop its in-memory `retain`/`truncate`** — that
part solves a problem 115 removed.

## Rejected alternatives

- **Lower the cap / cap the reported count to match.** Makes the numbers agree by making both wrong; the
  data is still unreachable.
- **Paginate the detail response's lots.** Adds a cursor to a nested field of a detail document to work
  around a cost that will not exist after 115. A Tender's lot count is a bounded real-world quantity
  (max 2,604), not a growth curve — it does not need a stream.
- **Ship `549f8f5` now and accept the latency.** A request that pins a worker for tens of seconds is a
  resource-exhaustion surface, not a trade — and 115's fix is days of work away, not months.
- **Keep the cap because it bounds cost.** Measured false (above): it bounds the answer, not the work.

## Verification when it lands

- `GET /v1/tenders/7161565` → `len(lot_details) == lots == 2604`.
- `/v1/lots?tender=7161565&after=N` returns the same rows the pre-fix cursor did, for N at 0, mid-set
  and past the end (the in-memory cursor must be exact, not approximately equivalent).
- Time it warm, and time it — do not read the plan: turso's EQP text misreports the `lots` access path
  (issue 112).

## The cost of the honest answer, measured (proj-fix, after 115's fix)

Post-115, on a fixture rebuilt to prod's measured slice profile for tender 7161565:

| LIMIT | rows served | time |
|-------|-------------|------|
| 125   | 125   | 0.0146s |
| 1000  | 1000  | 0.0152s |
| 3000  | **2604** | **0.0158s** |

**Serving the Tender's whole set costs 0.4% more than serving the truncated page.** That is the
measurement this issue rests on: after 115 there is nothing to trade off, so the only remaining
argument for the cap is that nobody has removed it yet.

Asserted, not just observed — `lot_prod_profile_probe` fails if the whole-set read ever costs more
than twice the smallest page, so the premise cannot rot silently.
