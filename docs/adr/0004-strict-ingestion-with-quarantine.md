# Ingestion is strict: unmapped content quarantines the whole notice

The importer's parser is exhaustive — every element and attribute of a source
notice must be consumed by a mapping or matched by an explicit ignore rule.
Any unmapped content fails that notice hard: nothing from it enters the
canonical layer (no partial imports); its raw payload stays in the append-only
archive marked quarantined with the exact reason, and the pipeline continues
with other notices. After an importer fix, quarantined notices are reprocessed
from the archive. The dashboard exposes the quarantine count as the headline
data-quality metric — zero means the no-silent-omissions guarantee holds.

This is a deliberate deviation from lenient parsing (skip-what-you-don't-know
is the common default for XML importers): leniency converts completeness bugs
into silently missing data, which is fatal for a dataset whose selling point
is "all business terms, no omissions". The alternative of halting the whole
pipeline on any violation was rejected because one malformed notice (TED does
publish those) would freeze the live feed for all users.

Together with the fields.json completeness test (ADR-0002) this forms the
field-coverage guarantee: schema gaps are caught statically in CI, ingestion
gaps are caught at runtime and are visible, recoverable, and counted.

Amendment (2026-07-19, decided with Lennart): completeness is era-scoped. The
promise is "everything the source era publishes, nothing silently dropped" —
each mapping profile (text / ted-export-r208 / ted-export-r209 / eforms, plus
per-CustomizationID eForms profiles) carries its own mapped-or-ignored
checklist drawn from its own schema, quarantine is strict within that
profile's universe, and the dashboard reports coverage per profile. Declared
free-text blobs (text-era bodies) count as mapped text, never as unmapped
content. See docs/research/ted-legacy-mapping.md §8.

Amendment (2026-07-21, from the architecture review): the headline is no longer
the raw quarantine total. That total conflates three kinds of held member, and
at backfill scale it is dominated by two large suspected-parser-gap buckets
(`unknown-field-code` ~577k, ~entirely the legacy `OC` field 1995–98;
`unparsable-xml` ~628k, ~entirely DTD-bearing 2008 XML) whose years measure ~92%
held against TED ground truth — i.e. mostly duplicate representations of notices
already held via another member, not real loss. So `model::quarantine_class`
splits every reason three ways: **actionable** — a member identified as a notice
whose content we could not represent, confirmed coverage loss held whole;
**suspected-gap** — a large notice-shaped bucket pending investigate-then-fix
(filed as issues 35/36), neither counted as loss yet nor dismissed; and
**benign** — the reason itself proves the member was never a distinct notice (a
wrong XML root, no publication id, a corrupt archive entry). The dashboard
headline is now `quarantine_actionable`; the raw total and the per-reason
breakdown stay visible beneath it. The original guarantee is unchanged, it now
reads on the actionable class — zero actionable means the no-silent-omissions
promise holds. Nothing is called benign without the reason proving it; uncertain
reasons default to suspected-gap, never benign.
