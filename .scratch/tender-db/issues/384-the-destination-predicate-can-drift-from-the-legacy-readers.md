# 384 — `has_destination` can drift from what the legacy readers actually consume; a source-reading guard

Status: ready-for-agent (filed 2026-09-12 by the owner, from issue 368's per-profile probe)
Kind: defect (diagnostic honesty) — the "published and dropped" diagnostics answer from a
predicate that is maintained by hand beside the readers it describes
Blocked by: nothing

## What happened

`read_legacy_results` matches the literal `"TED-CONTRACT_AWARD_DATE"` and consumes it (issue 255).
`has_destination(…, Channel::Date)` knew the eForms result stems and the sdk-0.1 award date, not
this one. So the first honest run of the r208 probe listed **1,688 rows of a consumed field as
dropped**, ranked 43rd, wearing the same clothes as a real gap. Fixed for that one field in 368 unit
2 by naming it (`LEGACY_AWARD_DATE_FIELD`) and using the name in both places.

The shape is general. The legacy readers (`read_legacy_results`, the role/address synthesis, the
lot-number keys `TED-LOT_NO | TED-LOT_NUMBER | TED-ITEM`, `TED-VAL_TOTAL`/`TED-VALUE_COST`,
`TED-NO_AWARDED_CONTRACT`, `LEGACY_BID_COUNT_FIELDS`) match literals and slices in `match` arms; the
predicate is a separate hand-written union. Nothing holds the two together, and a literal added to a
reader without a predicate entry is invisible until a probe happens to list it. Section 13 could not
have shown it either — the corpus head is eForms.

## The guard

A source-reading test in `project.rs`, the shape this repo already uses for "the invariant is about
the code, so the code is what it reads" (`every_report_field_is_read_by_the_renderer`,
`the_report_sieve_is_per_channel`): walk the bodies of the legacy readers for
`("TED-[A-Z_.]+"` literals (and `|`-alternations) paired with a `NoticeValue::<Variant>` pattern,
map the variant to its channel, and assert `has_destination(literal, channel)` for every one. A
literal the predicate does not know fails the build with its name.

Complement: the DE-1.x alias test already asserts every alias target is read (`any_channel_reads`,
the one legitimate blind use); this is the same idea for the legacy side.

## Not in scope

Making `has_destination` derive from the readers (a registry the readers consult). Worth it if the
guard fires more than once; the guard first.
