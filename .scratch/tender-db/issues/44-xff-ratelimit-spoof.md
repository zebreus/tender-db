# 44 — Rate-limit / SSE-cap key trusts spoofable X-Forwarded-For (MEDIUM)

Status: needs-verification (fix landed)
Severity: MEDIUM (defeats the only DoS control on the unauthenticated surface)

Found by the security review, code + deployment verified by the owner
(2026-07-21). `client_key()` (crates/app/src/v1/mod.rs) took the
LEFTMOST `X-Forwarded-For` value as the per-IP bucket for both the
governor rate limiter (10rps/50burst) and the SSE per-IP stream cap (~5).

Confirmed live-exploitable: the box's nginx sets
`X-Forwarded-For $proxy_add_x_forwarded_for` (preserves the client's
value and appends the real peer) — so the leftmost is attacker-supplied.
A caller rotating `X-Forwarded-For: <random>` per request gets unlimited
rate + unlimited SSE streams.

## Fix (landed)
nginx also sets `X-Real-IP $remote_addr` (the true peer, single trusted
value) — verified on the box. `client_key()` now prefers `X-Real-IP`;
falls back to the RIGHTMOST XFF entry (the one our proxy appended), never
the leftmost; then a shared "direct" bucket. Tests:
`prefers_trusted_real_ip_over_spoofable_forwarded_for`,
`without_real_ip_uses_rightmost_forwarded_for_not_the_client_value`.

Acceptance: spoofed X-Forwarded-For no longer changes the bucket
(same real peer → same limiter/cap); legitimate per-IP limiting intact.
