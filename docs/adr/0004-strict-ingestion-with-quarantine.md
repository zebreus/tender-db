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
