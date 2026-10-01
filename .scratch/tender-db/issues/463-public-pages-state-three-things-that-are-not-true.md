# 463 — the public pages state three things that are not true: an SSE snapshot "read in a single consistent transaction", three drained quarantine classes "not resolved", and a source offer that points back at itself

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the two edits that need no decision (the SSE sentence on both surfaces, and dating the three ledger rows), and the source offer with its deploy guard follows; all three ship in one gate and one deploy.
Kind: public-surface correctness (served docs, dashboard, licence offer)
Relates to: 55 (the paged snapshot), 416 (noted the SSE sentence and left it), 370 (the retired-claim detector the
SSE fix extends), 433 (the reclaim that drained the three classes), 413 (the precedent: its row was dated at its
reclaim), 303 (the terminal tripwire, the pattern for the ledger signal), 05 (gap 1: `SOURCE_OFFER` left for deploy
time)

## What is wrong

All reads below were taken 2026-10-01 between 11:50 and 11:58 UTC against prod at rev `9b44528` (`/health`) and
against HEAD `7fedf34`.

### 1. The SSE snapshot is not "read in a single consistent transaction"

Served text:
- `/docs` (`crates/app/src/v1/docs.rs:381`): "one `added` event per row currently matching your filter, read in a
  single consistent transaction."
- `/v1/openapi.json` (`crates/app/data/openapi.json:1105`, `EventStream.description`, served by `include_str!` at
  `crates/app/src/v1/openapi.rs:18`): "(the snapshot, read in a single consistent transaction)".

Since issue 55 (2026-08-09), the code works differently. `crates/app/src/v1/sse.rs:7-13` says the snapshot is streamed
"in keyset pages — a pooled reader per page, never a transaction held across pages", and that it "is at-least-once
per entity under concurrent writes, and the diff stream is what makes the client's final state exact". Inside the
page loop, each page takes a fresh read, from the isolated pool (`isolated.read_page`, `sse.rs:202`) or from the main
pool (`readers.get()`, `:211-212`). So the snapshot is not a point-in-time
view, and an entity can arrive as `added` in the snapshot and again as a `change` after `live`. A client that trusts
the docs can rely on neither of the two properties the sentence promises. Issue 416 recorded this on 2026-09-18
(lines 59–61: "noted here, not fixed here"), and no issue has tracked it since.

### 2. Three drained quarantine classes are listed as "not resolved"

`crates/app/data/quarantine-ledger.json` has three `"fix": "issue 433"` rows with `"resolved": null` (lines 392, 400,
408). They are the only null rows of the 49. The first row's diagnosis ends "Resolved when the reprocess runs after
deploy." That reprocess ran on 2026-09-28 (433's Verify): job 1618 (`%not an integer%`) reclaimed 310, job 1619
(`%not a number%`) found nothing left, job 1620 (`BT-803(%`) reclaimed 13, and fold 1622 folded them. 433 closed
DONE 2026-09-28 without dating its rows. Issue 413 did take that step for its own row ("a one-line commit that rides
the next deploy", 413:119), and that row now reads `"resolved": "2026-09-27"` (ledger line 384).

Live, from `GET /api/dashboard` `.quarantine.resolved_categories`:

| row (`detail_like`) | `resolved` | reclaimed | still held |
|---|---|---|---|
| `%not an integer%` | null | 310 | 0 |
| `BT-803(%` | null | 13 | 0 |
| `%not a number%` | null | 0 | 0 |

The dashboard (`/`) renders every null row under "Known populations, not resolved" (`crates/app/src/ui.rs:1041-1066`).
That section's text says its rows are "identified and measured but NOT fixed", and it shows each of these three with
"Still held 0". `ResolvedCategory.resolved` documents `None` as "still OUTSTANDING"
(`crates/model/src/dashboard.rs:198-201`).

The cause is in the process. Dating a row is a manual step at the end of a quarantine fix, and no check reads it: 433's
Verify counted the quarantine, not the ledger. A row with `resolved: null` and nothing held contradicts itself, and the
dashboard snapshot already carries the numbers that show it. Nothing flags it.

### 3. The AGPL source offer points back at itself

`GET /_source` answers: "Request it, naming the revision above, from the operator at
https://tenders.zebreus.click/_source." That URL is the page itself. The page names no address, so nobody can act on
the offer. `GET /v1` serves the same URL as `source_offer`. Both come from `crates/app/src/v1/mod.rs:58`
(`const SOURCE_OFFER`), used at `:1945` and `:1987`. `/docs` (`docs.rs:804-806`) says "The running server offers the
source of its exact revision at /_source", which that page does not do.

The repository has been public since 2026-08-08. `api.github.com/repos/zebreus/tender-db` reads `"private": false`
(created 2026-08-08T17:50:13Z), and `https://github.com/zebreus/tender-db/tree/9b445289a97d4f97773ad7bdfb4173f43a9e30d6`,
the deployed rev, answers 200. Issue 05 (lines 99–104) left this for deploy time: "either publish the repo and point
`SOURCE_OFFER` … at it, or keep the offer and be reachable". The repo was published, but the constant was never
changed. `docs/operations.md:1018-1021` still says "A public GitHub repo was declined for now (no external resources,
2026-07-21), so the written offer stands", which contradicts the same file's line 814 ("once the repository was
published to GitHub (public)").

One constraint shapes the fix. `deploy.sh:129` pushes only to the box (`git push vps "$REV:refs/heads/main"`), so
nothing guarantees that the deployed rev exists on GitHub. The box has run commits that `origin/main` did not have
(CLAUDE.md, 2026-09-04).

## Proposed fix

1. **SSE sentence, on both surfaces.** Replace the sentence with what `sse.rs` says: the snapshot is read in pages,
   one pooled read per page, with no transaction across pages. Under concurrent writes an entity can arrive in the
   snapshot and again as a `change` after `live`, so a client treats `added` as an upsert by id, and the diff stream
   makes the final state exact. Pin it in `docs.rs`'s tests in two ways. Add `"single consistent transaction"` to
   `RETIRED` in `no_served_surface_repeats_a_retired_claim`, which walks `/docs` and `/v1/openapi.json`. Add the
   positive half as `the_docs_say_the_sse_snapshot_is_at_least_once`, which asserts that both surfaces say
   "at-least-once".
2. **Ledger.** Set `"resolved": "2026-09-28"` on the three issue-433 rows, the date jobs 1618–1620 ran. Then make the
   next miss visible. `model::dashboard::quarantine_ledger_open_but_drained(&[ResolvedCategory])` names every entry
   with `resolved: None` and `outstanding == 0`. `/metrics` serves its length as
   `tender_db_quarantine_ledger_open_but_drained` beside `tender_db_quarantine_terminal_exceeded`, following issue
   303's pattern (`crates/app/src/v1/metrics.rs:435-446`). The gauge reads 3 today and 0 after the edit. A real open
   population is held by definition, so 0 is the steady state. Pin it with `an_open_ledger_row_with_nothing_held_trips`,
   beside `the_terminal_tripwire_trips_on_growth_and_unknowns_only` in `dashboard.rs`.
3. **Source offer.** Replace the constant with a function of `rev()`. For a full sha it returns
   `https://github.com/zebreus/tender-db/tree/<rev>`, and for a `dev` build the repository root. `/_source` names
   that URL as the place where this revision's source is, `/v1`'s `source_offer` serves it, and the `/docs` sentence
   becomes true. AGPL §13 asks for "access to the Corresponding Source from a network server", which this link gives
   directly. Pin it with `the_source_offer_links_the_running_revision_on_the_public_repo` in `mod.rs`'s tests: the
   offer contains `rev()` under the GitHub prefix and never contains `/_source`. `deploy.sh` has to keep the link
   true. Before the build, it fetches `origin` and refuses a `REV` that no `origin` ref contains, and it names the
   push to run (`git push origin HEAD:main`). Its existing `origin/main` guard at the top already refuses in the
   same way (exit 1, naming the command to run; it is skipped when `origin/main` does not resolve). Correct
   `docs/operations.md:1018-1021` in the same commit.

Gate with `ops/check.sh`. The `v1` and `ledger` tests compile only under `--features server`, which the gate enables.

## Verify

    curl -s https://tenders.zebreus.click/docs https://tenders.zebreus.click/v1/openapi.json https://tenders.zebreus.click/_source https://tenders.zebreus.click/api/dashboard | grep -o -e 'single consistent transaction' -e 'from the operator at https://tenders.zebreus.click/_source' -e '"fix":"issue 433","resolved":null' | sort | uniq -c

It prints one line for each false statement that is still served, with its count across the four public surfaces.
- **open** (2026-10-01 11:58 UTC, rev `9b44528`): three lines, `3 "fix":"issue 433","resolved":null` (the three
  ledger rows), `1 from the operator at https://tenders.zebreus.click/_source`, and `2 single consistent transaction`
  (`/docs` and the OpenAPI document).
- **done**: no output. When a unit is fixed, its line disappears. At done, the first line of `/_source` names the
  revision, and the page links `https://github.com/zebreus/tender-db/tree/<that revision>`, which answers 200.
