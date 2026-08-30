# 316 — Stoplisted generic names must not corroborate an R3 merge

Status: DONE 2026-08-30 (76d39e6) — the r3-cadence blocker is cleared
Kind: correctness (organization merge)
Relates to: 300 (§4.1, Stage 3 R3, Stage 4 stoplist)

## The obligation

Design §4.1: a name key shared by more organizations than the stoplist cap
is a generic name, and such keys are **disqualified as R3 corroboration
unless the identifier hard-checksum-passes**. R3's merge arm currently
corroborates on exact N2 name agreement with no stoplist consultation, so
a "Gymnázium"-class name can still corroborate a NULL-country rescue merge.

The scan now MEASURES that class every week — 55,312 n2 keys and 6,622 n3
keys over the cap, with the top-20 sample hand-reviewed on 2026-08-30
(tribunal administratif de X, european commission, platform vendors like
"avenue web systèmes" at 62,084 orgs). The wall exists; R3 just does not
consult it.

## Why it is not urgent but must not be lost

R3's shipped slices are anchored: a merge needs a checksum-passing
identifier anchor AND exact N2 corroboration AND the whole denial stack, so
a generic name alone cannot merge anything today. The exposure opens when
r3 joins the weekly cadence — a standing automatic merge path consulting a
corroboration rule the design already called insufficient.

**Therefore: the Stage-3 pending decision to fold `rule=r3` into the Sunday
tick stays BLOCKED on this issue.** That ordering is recorded in the
Stage-4 plan; this file is the issue it should always have been.

## Shape of the fix

The stoplist is derivable from `org_match_keys` with one indexed count per
key, so R3 can ask "is this corroboration key over the cap?" without a new
table. Gate: if over cap, require the anchor's checksum to be a HARD-scheme
pass (idgate::hard_scheme + Checksum::Pass), else deny and count it in the
report's denial ladder like every other wall.


## DONE 2026-08-30 (76d39e6): the wall is consulted

`match_org_null_country_r3` now asks `org_match_keys` whether the
corroborating N2 key is held by more than the cap — the SAME cap the scan
stoplists on, passed as one constant to both, so the two walls cannot drift
apart into disagreeing about what "generic" means.

- Over the cap, soft anchor: **denied**, on its own rung
  (`denied_generic_name`) in the denial ladder. Folding it into
  `uncorroborated` would have hidden the class the design named.
- Over the cap, HARD-checksumming anchor: allowed — the design's one
  exemption — and COUNTED (`generic_name_hard_anchor`), so the cadence
  reports how often the exemption carries a merge instead of leaving it
  assumed-zero.
- The probe is bounded: the range scan stops at cap+1, so the 62,084-org
  key costs what a 2-org key costs. It counts DISTINCT orgs, not rows.

**The lenient edge, stated because it is a real limit.** A key the
satellite has never seen reads NOT generic. That is deliberate — the
satellite is built wholesale (issue 315), and a stale one must not silently
deny every rescue merge — but it means the wall is only as complete as the
last build. The corollary shipped with it: a WET r3 run now REFUSES when
`org_match_keys` is empty or a build is mid-walk (non-zero watermark, no
covering index). A dry run still plans, because a person reviews its plan
before anything merges.

Not yet done: the actual fold of `rule=r3` into the Sunday tick. The
blocker this issue existed for is gone, but the fold deserves its own
decision with the wall's first prod numbers in hand — run r3 dry once after
deploy and read `denied_generic_name` before making a merge arm automatic.
