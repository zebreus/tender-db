# 578 — repair-placeholder-orgs (issue 440) re-resolves a dissolved org's mentions on the raw country with a bare LIMIT 1, not the resolver's key

Status: needs-triage — filed 2026-10-20 from issue 530 Build 2's review round 2 (the OQ4 follow-up its build plan
proposed and never filed). Next: unit 1, decide whether 440's own lookup (`DissolveTarget::Legacy440`) switches to the
resolver arm (`DissolveTarget::Resolver`, as issue 530's wet run uses), and measure what a re-run of a past 440 cohort
would land differently.
Kind: data quality (organization identity): a dissolve and a rebuild disagree on where a mention binds; low severity —
440 runs only on reviewed condemned classes
Relates to:
- 440 (RESOLVED-VERIFIED 2026-09-28): `repair-placeholder-orgs`, whose per-org body issue 530 Build 2 factored into
  `Db::dissolve_org_mentions`.
- 530: Build 2's wet run re-resolves through `DissolveTarget::Resolver` — the issue-234 resolver arm's key
  `(org_name_norm(name), register_jurisdiction(country))`, `ORDER BY id LIMIT 1`, one mint per key — so its mentions
  land where a fresh fold would put them. 440 kept its own lookup unchanged (Build 2's plan, "440 stays byte-identical").
- 234 / 432: the resolver's reuse key.

## Verify

    grep -n "DissolveTarget::Legacy440)\|&mut DissolveTarget::Legacy440" crates/store/src/canonical.rs

- **done**: it prints nothing (440's dissolve calls `dissolve_org_mentions` with the resolver arm, or
  this issue records why it keeps its own lookup and the doc on `DissolveTarget::Legacy440` says so).
- **open (2026-10-20)**: it prints `dissolve_condemned`'s `&mut DissolveTarget::Legacy440` call site
  and the `None if matches!(key, DissolveTarget::Legacy440) => 0` winner arm.

## What was found

- `Db::dissolve_org_mentions` under `DissolveTarget::Legacy440` (the `else if !name_norm.is_empty() && country.is_some()`
  branch) looks a moved mention's target up as `name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1` with the
  mention's RAW country and no `ORDER BY`; a mention with no name or no country gets a fresh provisional row each.
- The resolver keys the same lookup on `register_jurisdiction(country)` with `ORDER BY id LIMIT 1` and reuses one row
  per key. So an RE/GP/MQ mention, or a key with two identifier-NULL rows, can land on a different row under 440 than a
  fresh fold or a from-archive rebuild would bind (issue 530's A11 shape, which Build 2 fixed for its own path only).
- Two comments claimed the parity before review round 2: `repair_placeholder_orgs_batch`' "re-resolves through the
  post-234 provisional path ((name_norm, country) reuse)" and "the resolver's reuse key (issue 432), so a re-resolved
  mention lands where a fresh fold would put it" above all three `DissolveTarget` branches. Round 2 rewrote both to name
  the difference and point here.
- Not affected: issue 530's wet run and restore (`Resolver`, `Fixed`).

## Proposed units

1. Decide: switch `dissolve_condemned` to `DissolveTarget::Resolver` (its whole-row residue refusal and its "winner with
   no target" refusal come with it), or keep `Legacy440` and say why in its doc. Measure first: a dry 440 plan over the
   placeholder classes it ran on, comparing the two lookups' targets per mention.
2. If switched: the `placeholder_dissolve` tests pinned to the resolver's target (register country, lowest id).
