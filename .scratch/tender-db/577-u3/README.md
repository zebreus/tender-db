# 577 U3 job files (sliced, 2026-10-24)

Never post `UNSLICED-do-not-post.json`. 3.3M changed notices in one job push the paired `project` past the
incremental fold's 500k closure caps (`LEGACY_CLOSURE_CAP`, `LINK_CLOSURE_CAP`, crates/ingest/src/project.rs), so it
falls back to a whole-corpus re-projection (~5 h, never cancelled; issue 244's cliff). Source: 551
`b6/fast-plan-critique.md` finding 1.

Slices, each a reparse + its paired `project`, each its own class-Z segment (segment.py open, POST, close):

1. `slice1-de1.json` — eforms-de-1.0/1.1/1.2 whole. Under the cap (539 run b re-folded exactly these in ~55 min).
   W7's live test: the bar is 0 corrections outside the NOTE classes.
2. `slice2-de2.json` — eforms-de-2.0/2.1 whole (539 run c, ~55 min).
3. `slice3-sdk-probe.json` — the 14 sdk-1.x profiles, first 20 packages. Its summary gives notices per package and
   `next_after`.
4. sdk-1.x rounds: `{"kind":"reparse","profiles":[<the 14 sdk>],"after":<previous next_after>,"packages":N}`, N sized
   so a round re-parses at most ~250k notices (526's rounds ran 117k–255k). `reparse_packages` counts only packages
   that hold parsed notices of these profiles, so N is per-profile-set. Always carry `after`: a fresh enqueue without
   it re-walks the prefix (issue 272).

Fallback alarm: a slice whose project logs "INCREMENTAL → FULL fallback" is a 5 h corpus job; stop slicing that way.
Each slice's K1: now + upper bound + 15 min ≤ the next tick (02:10Z Sunday weekly, 08:35Z daily).
