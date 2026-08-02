# 97 — nginx access log carries no request timing, so latency regressions are invisible in prod logs

Status: proposed
Kind: observability
Design owner: run-driver / ops
Relates to: 61 (health latency during projection), 89 (`/v1/tenders/{id}` full-scans `tender_version_bid_parties`)

## Context

`/etc/nginx/nginx.conf` has a bare

```
access_log /var/log/nginx/access.log;
```

with **no `log_format` directive**, so nginx falls back to the stock `combined` format.
That format records remote addr, time, request line, status, bytes, referer and
user-agent — and **no timing at all**: neither `$request_time` (total time nginx spent
on the request) nor `$upstream_response_time` (time the app took to respond).

**Consequence: a latency regression in production is invisible in the logs.** We can see
*that* a path was hit and *what status* it returned, but never *how long it took*.

## Why this matters here specifically

The two most recent user-facing incidents were both **latency** regressions, not errors:

- **issue 61** — `/health` became slow during projection (the endpoint returned 200
  throughout; only the latency was wrong)
- **issue 89** — `/v1/tenders/{id}` full-scanned `tender_version_bid_parties` on every
  request (again 200s, just slow)

Neither would have shown up as a non-2xx in the access log. In both cases the regression
had to be found by other means. A log with `$request_time` would have surfaced both
immediately, from real traffic, without anyone suspecting anything first.

Concretely: during the 2026-08-02 restore, the post-`nginx up` health check could not use
the access log to answer "is the 89-class regression back?" and had to substitute
**synthetic `curl` timing against the hot paths** instead. That works as a go/no-go probe
but measures the app under our own requests, not what real users actually experience.

## Proposal

Add a timing-aware log format and point the access log at it:

```nginx
log_format timed '$remote_addr - $remote_user [$time_local] "$request" '
                 '$status $body_bytes_sent "$http_referer" "$http_user_agent" '
                 'rt=$request_time urt=$upstream_response_time';

access_log /var/log/nginx/access.log timed;
```

`$upstream_response_time` is the one that isolates *the app's* contribution from nginx
and network time — that is the number that would have moved in both 61 and 89.

## Landing constraints

- **Deliberate, not mid-incident.** This was deliberately NOT done during the 2026-08-02
  restore: the site came back up on the exact known-good config, and an nginx config edit
  was not allowed to become the variable that broke a restore-morning start.
- Run `nginx -t` before any reload.
- Reload (not restart) once validated.
- Consider log rotation/volume: `$request_time` adds ~10-15 bytes per line, negligible.

## Acceptance

- Access log lines carry both `rt=` and `urt=`.
- A latency regression of the 61/89 class is detectable from the access log alone —
  e.g. a p95 `urt` on `/v1/tenders/{id}` that can be trended over time — with no synthetic
  probing required.

## Comments

Found 2026-08-02 while preparing the post-restore health check for the eForms-DE 1.x
re-fold landing. Filed at team-lead's request as an explicitly post-landing item.
