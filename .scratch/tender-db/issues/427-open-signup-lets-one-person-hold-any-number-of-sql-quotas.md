# 427 — open signup lets one person hold any number of `/v1/sql` quotas

Status: ready-for-agent — **BUILT 2026-09-26 21:xx UTC** (the owner's decision with Lennart: a global daily
cap, not per IP); gate, deploy and the live Verify follow.
Kind: abuse resistance
Relates to: 425, 426 (the other two gaps in the same review), 06 (accounts)

## What

Anyone can register (username + password, no verification), and every account gets its own `/v1/sql`
quota — 2 concurrent queries, 300 per hour. The per-account limits therefore bound nothing for someone
willing to make accounts: a handful of them can hold all 4 SQL reader connections and deny the endpoint to
everyone for minutes (the REST API and ingest are unaffected — see 17/417).

## Decision (2026-09-26)

Per-IP limits were considered and rejected: an address is a weak identity behind NAT (many real users on
one address) and trivially rotated by anyone abusive. Instead, **at most 5 new accounts per rolling day,
for the whole service** (`accounts::SIGNUPS_PER_DAY`, `SIGNUP_WINDOW_SECS`). A legitimate user refused by
it waits a day; the traffic today does not come near it.

## Built

- `store::Db::create_user_capped` — the insert carries the cap in the same statement
  (`INSERT … SELECT … WHERE (SELECT COUNT(*) FROM users WHERE created_at > ?) < ?`), so two simultaneous
  signups cannot both take the last slot; a refusal is told apart as `Taken` (the name exists) or `AtCap`.
- `accounts::register` reads the count BEFORE the argon2 hash, so at the cap a signup costs one COUNT, not
  a deliberately expensive hash; the error is `AuthError::SignupsClosed` ("new accounts are limited to 5 per
  day across the whole service …").
- Test `crates/app/tests/accounts.rs::signups_are_capped_per_day_across_the_service`: yesterday's accounts
  do not count, five of today's pass, the sixth is refused and nothing is created, and the store's capped
  insert refuses on its own at the cap. **Mutation-checked**: loosening the insert's guard lets the racer
  in and fails the test.

## Verify

    grep -c 'SIGNUPS_PER_DAY' crates/app/src/accounts.rs

- **done**: 3 or more, and the deployed rev carries it
- **open**: 0 (read 2026-09-26, before the build)
