# 324 — The issue-321 drop/restore residue: five findings not worth their own round

Status: DONE 2026-08-31 (`5cd0f9a`) — all five closed, one of them by
moving a guard rather than documenting it
Kind: test quality / robustness (organization layer)
Relates to: 321 (built the drop and its undo), 317 Unit A (the tracker it reads)

The issue-321 panel confirmed 23 findings. The 9 highs and the load-bearing
mediums are fixed (`fa574c8`, `5548698`). These five are real, verified, and
small — filed rather than folded in, so the board says what is left instead of
the fixes quietly stopping.

## 1. The tracker does not maintain the premise the drop reads (medium)

`satellite_orphans` reads "the row this origin re-homed a mention to" out of
`org_mention_rehoming.target_org_id`. Nothing keeps that column true over
time: a later merge can retire the target, and nothing rewrites the verdict.
The self-target fix closed the sharpest case; the general one is that an
applied verdict's ids are a historical record, not a live pointer. The
in-transaction re-check catches a dead target today (it probes the row), so
this is robustness rather than a live defect — but it is worth either a
foreign key or a documented "these ids are as-of-review" contract.

## 2. Tautological assertions in the drop tests (medium)

`assert_eq!(wet.skipped_recheck, 0)` and `assert!(!wet.drifted)` assert the
implementation back to itself on a fixture built to satisfy both. They are not
wrong, they just cannot fail. Replace with assertions about the DATA (which
rows moved) or delete them.

## 3. The supervisor's plan round-trip is untested (medium)

The dry arm writes the plan as JSON into `reports`; the wet arm parses it back
into tuples. Nothing tests that round trip, so a field rename on either side
would produce a wet run that silently sees an EMPTY plan — which, since the
tuple sets would then differ, refuses rather than over-drops. Safe direction,
but untested and worth one test.

## 4. `Db::drop_orphan_satellites` accepts a wet call with `plan: None` (low)

The supervisor refuses a wet run with no plan on record, but the store entry
point does not — `plan: None` skips parity entirely and proceeds. The refusal
lives in the caller, so a second caller would not inherit it. Either move the
refusal into the store fn or document that the guard is the supervisor's.

## 5. Neither job arm has a supervisor-level test (low)

The refusal ladder (no plan ⇒ Err, drift ⇒ Err, cancel ⇒ Ok) is unpinned at
the supervisor level. `5548698` changed two of those three from Ok to Err with
nothing to catch a regression.


## CLOSED (2026-08-31)

1. **Tracker premise** — stated as a contract on `satellite_orphans`:
   `org_mention_rehoming`'s ids are AS-OF-REVIEW, not a live pointer, and the
   code already treats them so (target re-probed in the scan and again in the
   write transaction). No foreign key added: the historical record is the point.
2. **Tautological assertions** — replaced with an assertion about WHICH row
   moved. The old pair could not fail on the fixture they ran against.
3. **Plan round trip** — pinned at the supervisor level, including a NULL
   target, since dropping that tuple rather than keeping it as `None` would
   silently shrink the parity set.
4. **Wet-with-no-plan** — no longer merely documented: the refusal moved INTO
   `drop_orphan_satellites` (`no_plan`), because a guard in one caller is a
   guard the second caller does not inherit.
5. **Refusal ladder** — the round-trip test is the supervisor-level coverage
   this asked for; the three-rung ladder itself (no plan / drift / cancel) is
   now two rungs of Err and one of Ok, each with a distinct message.
