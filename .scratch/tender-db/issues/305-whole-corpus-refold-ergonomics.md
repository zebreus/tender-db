# 305 — whole-corpus refold ergonomics: mislabeled identity pass + predictable fallback

Status: OPEN (filed 2026-08-27 from watching job 402 — THE epoch refold's paired
projection)
Kind: operability (progress honesty) + one avoidable 98-minute pass
Relates to: 58 v2 (the closure cap + fallback, working as designed), 65 (phase
records exist to prevent exactly this misreading), 291 (the refold this bit).

Observed on job 402 (the 23-profile epoch refold's `project rebuild=false`):

1. **The incremental identity pass reports itself as `planning`** — the phase
   record showed `planning: 13,900,000/14,314,613 "notices planned"` for 98
   minutes while the job was actually in `incremental stage pass-1 identity`
   (journal: "1181955 new keyed keys, 11003672 legacy notices … 5886.2s").
   When the 58-v2 fallback then fired, `build_plan` restarted the same-named
   counter from zero — reading like a crash-restart. The identity pass needs
   its own phase name ("identity"), so the record never lies about which stage
   is running. Small fix, high operator value.

2. **A whole-corpus refold's fallback is predictable**: unmarking every
   profile guarantees the legacy closure exceeds the 500k cap, so the 98-min
   identity pass only discovers what the enqueuer already knew. Either the
   refold admin path should pass a hint ("expect full") when the profile set
   covers the legacy eras, or the closure-cap check should run BEFORE the
   identity pass on a cheap upper bound (count of un-projected legacy
   notices). Not urgent — whole-corpus refolds are rare by design — but the
   next one shouldn't pay it.

The fallback itself worked exactly as issue 58 v2 designed (11,003,672 > cap →
full re-projection); WAL stayed bounded (~650 MB, truncating each chunk).
