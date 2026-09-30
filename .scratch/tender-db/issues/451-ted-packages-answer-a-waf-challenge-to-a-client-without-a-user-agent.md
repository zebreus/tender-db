# 451 — TED's `/packages/*` answers an AWS WAF challenge to a client without a User-Agent, so the daily TED probe fails

Status: DONE — DEPLOYED 2026-09-30 ~08:1x UTC (`f733846`) and VERIFIED: the daily run enqueued right after (`{"kind":"daily"}`, jobs 1707–1715) probed TED ok, "probed 3 issue(s), 1 new", and processed and projected 3,512 TED notices. The startup-catch-up predicate fix below rides the next deploy.
Was status: ready-for-agent — BUILT 2026-09-30 ~08:0x UTC (fix + test, gated before deploy). Deploys at once: every hour
without it is a TED day that is not ingested.
Kind: operations (ingestion outage)
Relates to: 280 (the fetch client's timeouts), 402 (the last TED hole, backfilled), 450 (FTS fetch control)

## What happened

The 2026-09-30 07:35 UTC daily tick failed its TED probe (job 1692) with `unexpected status: 202 Accepted`. The two
catch-ups failed the same way (1701 at 07:40, 1702 at 07:45). TED issue 2026-00189 onward was not fetched. DÖE and
FTS were unaffected.

Read on the box and from a second network:

    $ curl -sD - https://ted.europa.eu/packages/daily/202600188 -o /dev/null
    HTTP/2 202
    content-length: 0
    x-amzn-waf-action: challenge
    x-cache: Error from cloudfront

- TED's CloudFront now runs an AWS WAF **challenge** in front of `/packages/*` (and `/en/notice/…/xml`). It answers 202
  with an empty body. It does so even for yesterday's package (00188), which had been fetched fine, and from both
  networks. So it is a site rule, not an IP ban.
- `api.ted.europa.eu/v3/notices/search` is unaffected (200).
- The rule keys on the User-Agent:

| User-Agent | answer |
|---|---|
| none (reqwest's default) | 202 challenge |
| `curl/8.5.0`, `python-requests/2.32` | 202 challenge |
| `tender-db/1.0 (+https://tenders.zebreus.click)` | 202 challenge |
| `Mozilla/5.0 (compatible; tender-db/1.0; +https://tenders.zebreus.click)` | **200**, 18.8 MB |
| a browser's | 200 |

reqwest sends no User-Agent at all unless told to.

## Fix (built)

- `ingest::fetch::USER_AGENT` = `Mozilla/5.0 (compatible; tender-db/<crate version>; +https://tenders.zebreus.click)`.
  This is the crawler convention (Googlebot's shape): it names the tool and where to find it. It does not pose as a
  browser, and TED can still tell these requests apart and refuse them. The supervisor's `fetch_client()` and the
  `fetch` CLI send it.
- FTS, DÖE, the ECB zip and TED monthly all accept it (checked from the container with this exact string).
- A WAF answer is now a named error. `Error::Challenged { status, action }` is raised at both request sites (the
  package download and the FTS page GET) whenever `x-amzn-waf-action` comes with anything but 200/206. The job reads
  "refused by the source's bot challenge (202 Accepted, x-amzn-waf-action: challenge): the request's User-Agent was
  not accepted (issue 451)", not a bare 202.
- Test (`crates/ingest/tests/fetch.rs`): `a_waf_challenge_is_named_and_the_crawler_user_agent_passes_it`. A mock with
  TED's rule challenges a UA-less client (named error, nothing registered) and serves `USER_AGENT`.

## If TED refuses this User-Agent too

Do NOT move to a browser User-Agent. That would be disguise, getting past a bot check TED chose to put there. The
honest fallbacks are the search API, which is not WAF'd (`api.ted.europa.eu`, docs/research/ted-access-channels.md §3),
or asking TED's helpdesk for an allow-listing.

## Verify

    /root/aj.sh "/admin/jobs?limit=20" | grep -c "unexpected status: 202"

- **done**: after the deploy, a `probe` job for `ted daily` ends `ok` and fetches 2026-00189 onward, and the next
  process/project fold the new TED notices in.
- **open**: a TED probe after the deploy still errors (with `Challenged` now named).

## Also fixed: the startup catch-up counted the wrong probe as serving the tick

A deploy inside the morning window restarts the process, which kills issue 222's in-process re-probe loop.
Issue 245's startup catch-up is meant to cover that. It counted a tick as served by ANY successful `probe` after
it. But the tick pushes three probes (TED, DÖE, FTS), and on 2026-09-30 DÖE's and FTS's succeeded while TED's
failed. So a restart then would have left TED's day unfetched until the next tick; this morning's deploy was
followed by a manual `daily` to cover it.

`tick_needs_catch_up` now takes `weekday`. On a weekday only a TED probe (`ted daily (probe)` or
`(catch-up)`) serves the tick, and only a queued TED probe suppresses the catch-up. A weekend tick pushes no TED
probe, so any probe serves it. The test `a_tick_that_passed_unserved_is_caught_up_on_startup` gains the
2026-09-30 morning.
