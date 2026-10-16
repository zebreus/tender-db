# 563 — a resumed SSE stream re-resolves the literal `now`, so its date bound moves and the resume is not exact

Status: needs-triage — filed 2026-10-16 from issue 547's re-review (finding "`now` re-resolved on resume", reproduced
end to end over a real SSE socket). Documented on `/docs` (Live feed, the resume paragraph), in openapi's
`EventStream` and in CHANGELOG 547; not fixed.
Kind: correctness (SSE resume exactness under a moving date bound)
Relates to:
- 391 unit 1: taught `parse_instant` the literal `now` (the moving instant) on all four date bounds.
- 547: lets an unsorted date-bounded stream open, so `deadline_after=now` without `sort=id` is now a stream a client
  can open at all (before, it was a 400 "sort does not apply to event streams"). The flaw predates it: a
  `deadline_after=now&sort=id` stream had it already.
- 533: `status=open`/`closed` reads `Filter.now` the same way (the deadline window), with no absolute spelling.
- 164 / 287: the `(false, false)` arm and its over-delivery rules.

## Verify

    grep -c 'fn a_resumed_now_stream_is_exact_or_refused' crates/app/tests/api.rs

- **done**: 1 (the test passes: a `deadline_after=now` stream resumed after a held Tender's deadline passed either
  hears that Tender's `removed` on its next change, or is refused visibly), OR a `## Decision` below keeps the gap
  documented with its reason.
- **open (2026-10-16)**: 0. `/docs`, openapi and CHANGELOG 547 say a `now`-bounded resume is not exact and to pass
  an absolute timestamp for an exact resume.

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

1. **Decide (S).** A: pin the resolved bound in the resume token — the event id carries the connect-time instant
   (e.g. `<generation>:<cursor>:<now>`), and a resume re-applies it instead of the clock; the snapshot of a fresh
   connection still resolves `now` afresh. Keeps `now` usable on streams and makes a resume exact; costs a token
   format change (a legacy two-part token keeps today's behaviour) and covers `status` through `Filter.now` too.
   B: refuse `now` on streams (400 naming the absolute form), leaving `status` documented. C: keep the documented
   gap.
2. **Build (S–M)** if A or B, with the api test the Verify names (the 547 re-review probe's shape), and retire the
   `/docs`, openapi and CHANGELOG caveats.
