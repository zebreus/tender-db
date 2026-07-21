
## 2026-07-21 16:35 — PRODUCTION ACCEPTANCE: cheap resume on b978c96 (run-driver)

First restart with a resume cursor already recorded (job 1 had been writing its
cursor since 15:10). Deploy b978c96 restarted the service 16:34:01 UTC; job 1
recovered and resumed **directly at package 2012-03** — NOT from 1993:

- current: job 1 `process ted monthly (all)`, package **2012-03**,
  `packages_done/total = 0/171` (total dropped from 401 → 171: the ~230
  already-processed packages are skipped entirely, not even re-walked),
  notices already climbing (7625 → 11465 within ~1 min of boot).

Contrast: the b0a5cdb restart (no cursor on job 1's row) paid a ~90-min full
re-walk from 1993. This one resumed real parsing in ~1 min. **Resume-cursor
production acceptance MET** — restarts are now cheap; no re-walk, no wasted IO.
