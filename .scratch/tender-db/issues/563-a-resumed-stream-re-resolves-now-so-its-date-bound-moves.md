# 563 — a resumed SSE stream re-resolves the literal `now`, so its date bound moves and the resume is not exact

Status: ready-for-agent — built (uncommitted) 2026-10-16, decision B, review fixes applied the same day; NEXT: gate
(`ops/check.sh`), commit, ship in ONE deploy with 547 (547's Status names the coupling), then the Verify curl; close
as `DONE <date>` when it reads done. A `/v1/tenders` event stream refuses a date bound spelled as the literal `now`
with 400 (fresh connect and resume alike); pages keep `now`. Filed 2026-10-16 from issue 547's re-review (finding
"`now` re-resolved on resume", reproduced end to end over a real SSE socket).
Kind: correctness (SSE resume exactness under a moving date bound)
Relates to:
- 391 unit 1: taught `parse_instant` the literal `now` (the moving instant) on all four date bounds.
- 547: lets an unsorted date-bounded stream open, so `deadline_after=now` without `sort=id` became a stream a client
  could open at all (before, it was a 400 "sort does not apply to event streams"); this issue refuses it again, so
  the two ship in one deploy. The flaw predates 547: a `deadline_after=now&sort=id` stream had it already.
- 533: `status=open`/`closed` reads `Filter.now` the same way (the deadline window), with no absolute spelling.
- 164 / 287: the `(false, false)` arm and its over-delivery rules.

## Decision

**B (owner, 2026-10-16): refuse `now` on event streams.** On `Accept: text/event-stream`, a date bound
(`published_after`, `published_before`, `deadline_after`, `deadline_before`) spelled as the literal `now`, any case,
is a 400 whose message names the bound, says why (a resume would resolve `now` again, so the bound would move under
the state the client holds) and what to send instead (an absolute timestamp; for `deadline_after` alone also
`status=open` for tenders still open, with its clock caveat in the message itself). Pages accept `now` exactly as
before.

- **Why B over A and C.** Before 547 a date-bounded stream was refused unless it passed `sort=id`, so almost no client
  can depend on `now` on a stream; B makes the stream contract exact instead of documenting a gap (C). A (pin the
  resolved bound into the resume token) stays a possible later unit if a client needs a moving bound on a stream.
- **Where.** `refuse_now_on_a_stream` in `crates/app/src/v1/mod.rs`, called from `collection()` on the stream branch
  before the org resolution (a refused stream reads nothing), over the bounds the collection honours. Date bounds
  are `/v1/tenders`-only (`honoured_params`); `/v1/lots` does not take them (they are ignored there, named in a
  page's `ignored_filters`), so `now` moves nothing on a lots stream and it still opens. `is_now` is the one
  spelling both `parse_instant` and the refusal read. On `/v1/tenders` a stream that also asks for `sort=published_at`
  / `deadline` or `order=desc` meets 547's sort refusal first (checked in `tenders()` before `collection()`). A
  malformed bound on such a stream (`deadline_after=today`) gets a 400 that offers the absolute forms only and says
  `now` is pages-only (`Params::filter_on` tells `parse_instant` which bounds refuse `now`); on `/v1/lots`, where
  `now` opens, the page wording with `now` stays.
- **`status` stays documented.** `status=open`/`closed` reads `Filter.now`, fixed when a stream connects, and has no
  absolute spelling. `/docs` and openapi say so: no event marks a deadline passing, so drop a Tender from a
  `status=open` state once its deadline has passed; after a resume a change that leaves such a Tender closed is not
  sent, while a version that reopens it arrives as `added` (within one connection it would still get `changed`,
  since the connect-time clock still calls it open). The owner's "`status=open` for tenders still open" is offered
  in place of `deadline_after=now` alone (it is no substitute for a publication bound, and `deadline_before=now`'s
  counterpart would be `status=closed`), and the 400 and the openapi `deadline_after` text carry its clock caveat
  inline, not only `/docs`.

## Verify

    curl -s --max-time 3 -H 'Accept: text/event-stream' 'https://tenders.zebreus.click/v1/tenders?deadline_after=now' | grep -c 'is refused on an event stream'

- **done**: 1 — the deployed server answers the `now` stream with this issue's 400 ("deadline_after=now is refused on
  an event stream: …").
- **open** (2026-10-16, before the 547+563 deploy; read from 547's open line, not re-run in this build): 0 — pre-547
  prod answers 400 "sort does not apply to event streams"; a 547-only deploy would open the stream (200, events until
  `--max-time` cuts it), which also prints 0.

Build evidence (the gate's, not the Verify): 2026-10-16, uncommitted, with the review fixes, the focused runs under
`/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b563/fix/` are green.
`a_resumed_now_stream_is_exact_or_refused` takes the "refused visibly" branch. On a fresh connect, a `/v1/tenders`
stream with any of the four bounds `=now` (any case, beside other filters or an absolute bound) answers 400
`application/json` naming the bound, "would resolve it again" and "pass an absolute timestamp instead", with
`status=open` (and its clock caveat) offered for `deadline_after` alone; a malformed bound on that stream gets a 400
that does not offer `now` (on `/v1/lots` it still does). A `Last-Event-ID` resume under `now` answers 400 naming the
bound; a `?cursor=` resume answers 400. The same stream under an absolute bound snapshots and resumes quietly; a
`/v1/lots` stream with `deadline_after=now` opens. `a_page_keeps_the_literal_now_that_a_stream_refuses`: pages
answer 200 as before. `the_docs_say_a_stream_refuses_a_now_bound`: the four openapi bounds, the two `/docs` rows and
the ordering section say `now` is pages-only, and only openapi's `deadline_after` offers `status=open`, with its
caveat; openapi's `listTenders` and the `/docs` Live feed intro name the refusal among a stream's 400s; both Live
feed texts state the refusal, why, the alternatives and `status`'s clock, and the retired caveats are gone.
After deploy, also: the same curl with `deadline_after=2026-01-01` opens the stream (200).

## What was found

`parse_instant` returns `store::now_unix()` for `now` at request time (crates/app/src/v1/mod.rs ~1535), and the
handlers build each request's `Filter` with `p.filter(store::now_unix())` (`tenders` ~1608, `collection` ~1322), so
`Filter.now` and every `now` bound are fixed when a connection opens. `sse::subscribe` keeps that one `Filter` for the
snapshot and every diff, which is consistent within one connection. A Last-Event-ID resume opens a NEW connection
and resolves `now` again: the resumed diff checks bound T1 against a client state built under T0 < T1.

`diff` only runs over change-log rows (`changes_since`), so a deadline passing produces no event. After the resume, a
change to a held Tender whose deadline lies in [T0, T1) probes `(false, false)` and is dropped
(crates/app/src/v1/sse.rs ~465): no `removed`, ever, unless the change is a log `removed` or an in-place `changed`.
The same holds for `deadline_before=now` / `published_*=now` in the other direction (a Tender that entered the moving
bound in between can be heard first as `changed`), and for `status=open`/`closed`.

Reproduced in the 547 re-review (2026-10-16; scratch logs under
`/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b547/reverify/now/`): the chain
CN with its deadline rewritten to now+5 s; two streams, `deadline_after=now` and a control `deadline_after=<T0
literal>`, both snapshot the Tender; after the deadline passed the page returns 0 rows and both open streams stay
quiet; then CHAIN[1]:
- deadline moved to 2026-01-27: the open `now` stream, the open literal stream and the RESUMED literal stream got
  `removed`; the `now` stream resumed from its live token got nothing (10 s);
- deadline unchanged: the three got `changed`; the resumed `now` stream got nothing.

So a client resuming a `now` stream keeps a Tender that matches neither bound; an unbroken connection does not.

## Units

1. **Decide (S).** Decided B, 2026-10-16 (`## Decision`). A: pin the resolved bound in the resume token — the event id carries the connect-time instant
   (e.g. `<generation>:<cursor>:<now>`), and a resume re-applies it instead of the clock; the snapshot of a fresh
   connection still resolves `now` afresh. Keeps `now` usable on streams and makes a resume exact; costs a token
   format change (a legacy two-part token keeps today's behaviour) and covers `status` through `Filter.now` too.
   B: refuse `now` on streams (400 naming the absolute form), leaving `status` documented. C: keep the documented
   gap.
2. **Build (S–M)** if A or B, with the api test the Verify names (the 547 re-review probe's shape), and retire the
   `/docs`, openapi and CHANGELOG caveats. **Built 2026-10-16 (B), uncommitted:** the refusal in
   `crates/app/src/v1/mod.rs`; tests in `crates/app/tests/api.rs` and `crates/app/src/v1/docs.rs`; `/docs` (the two
   date-bound rows, the ordering section's `now` sentence, the Live feed resume paragraph), openapi (the four bounds,
   `EventStream`), CHANGELOG (a 563 entry; 547's `now` bullet points to it). **Review fixes (2026-10-16):** a
   malformed bound's stream 400 no longer offers `now` (`Params::filter_on`); `status=open` is offered beside
   `deadline_after` alone and with its clock caveat in the 400 and openapi; the `/docs` Live feed intro and openapi's
   `listTenders` name the `now` refusal; the `status` resume caveat now says a change that leaves the Tender closed
   is not sent (a reopening version arrives as `added`) on all three surfaces; this file's Status and Verify follow
   the board rules; 547's record and Status name the retired caveat and the joint deploy.
