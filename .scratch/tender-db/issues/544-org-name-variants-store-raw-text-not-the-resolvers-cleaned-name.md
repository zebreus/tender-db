# 544 — `organization_names` written by the 307 walk and the 542 re-derivation carries raw text, not the resolver's cleaned name

Status: ready-for-agent — filed 2026-10-15 from issue 542 unit 2's second review (finding confirmed by an
independent verifier). Next: unit 1, size it.
Kind: data quality (organization name variants)
Relates to:
- 307 (the backfill walk);
- 542 (the dissolve's re-derivation, walk-equivalent by design);
- 527 (`(gegunde partij)` mark);
- 528 (reviewed Dutch award prose);
- 520 unit 4 (void-summary winner);
- 300 (the cross-language matcher reads the satellite).

## Verify

    echo "SELECT org_id, lang, name FROM organization_names WHERE name LIKE '%(gegunde partij)%' LIMIT 20" | ssh root@zebreus.click /root/sq.sh

- **done**: 0 rows.
- **open (2026-10-15)**: not read yet. This is unit 1. Job 2491's whole-corpus walk ran at 01:46 UTC, so any marked
  spelling a fold had cleaned was written back raw then.

## What was found

The resolver rewrites a party's name before it records the name as a variant (`ingest/src/project.rs`,
`mentions_and_dropped`):
- issue 527 strips TenderNed's `(gegunde partij)` mark from BT-500;
- issue 528 maps reviewed Dutch award prose to the awardee;
- issue 520 unit 4 replaces a void summary with its winner;
- names of nested Organization sections reach the outer mention through `enclosing` and `nested_org_aliases`.

`Db::backfill_org_name_variants_batch` (the issue-307 walk) and `Db::rederive_org_name_variants` (issue 542) both
write `notice_texts.value` raw and match the exact section only. So after a walk, or after a dissolve re-derives an
org:
- the head can be clean while the satellite holds the raw spelling, e.g. `… (gegunde partij)` or Dutch prose;
- a nested-org variant can be missing.

The readers are the cross-language matcher and the resolver's name-agreement evidence. Both read the union of an
org's names, so a raw spelling adds one key that only another raw spelling matches. It is a noise key, not a wrong
merge.

## Also from that review

The 542 re-derivation reads a reused target's whole mention history, inside the write transaction. That is fine for
the targets seen so far (job 2489 reused Hiscox, 2123088). A dry run reports only `variant_orgs`. It could also size
the read, as the sum of mention counts over the reused targets, so that a dissolve landing on a 100k-mention
provisional is visible before going wet.

## Units

1. **Size it.** Bounded reads: `organization_names` rows containing `(gegunde partij)`, and the 528 reviewed prose
   strings (the reviewed table names them). If both read 0 or a handful, record the reading and close.
2. **If it is real:** add one shared rewrite, e.g. `pub fn variant_name(field, value) -> String` in ingest. It holds
   527's strip and 528's map, and `mentions_and_dropped` calls it too. Thread it into both store paths as a
   `clean: fn(&str, &str) -> String` beside `normalize`, select `field_id` in both reads, and refresh the affected
   orgs with a scoped re-derivation.
3. **Dry-run sizing** for the dissolve's re-derivation: the summed mention counts of the reused targets.
