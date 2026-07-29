# 72 — text-era "OC" field quarantines 577K notices (unknown-field-code)

Status: resolved as DUPLICATE of issue 35 — no code change; needs only the reprocess (team-lead ops step)
Kind: completeness / data-quality
Relates to: 35 (the OC/ON mapping — already implemented + deployed), 71 (SDK vendoring), ADR-0004, CONTEXT.md (text era = header-only, EN-only for now)

## Root cause (2026-07-29, proj-fix)

There is **no parser/inventory mismatch and no code fix to make** — the OC field
was already mapped and deployed by **issue 35 (commit 857fdb0, 2026-07-21)**,
which is an ancestor of the deployed prod rev `498b928`:

- `crates/ingest/src/text/rules.rs`: `("OC", PerLine(Type::Cpv))` and
  `("ON", PerLine(Type::Line(Some("EN"))))` — so `rules::rule("OC")` returns
  `Some(..)`; the parser does NOT hit the `unknown-field-code` branch for OC.
- `crates/ingest/sdk/text-inventory.json`: OC/ON present (1995-98 window).
- Fixture + test `oc_and_on_are_claimed_as_cpv_and_description` (crates/ingest/tests/text.rs)
  passes: OC → cpv `71101000`, ON → its EN description. Verified green today.
- Ledger entry (crates/app/data/quarantine-ledger.json): category "text OC/ON
  object CPV", `reason=unknown-field-code`, `detail_like="%: OC"` — the reclaim
  count lights up when the bucket reprocesses.

The snapshot's `576,753` quarantine count is **byte-identical** to issue 35's
pre-fix number: these are **stale quarantine rows** created before the 07-21
deploy and never reprocessed (the canonical build + issue-61 incident consumed
the window in between). The deployed parser already handles OC correctly; any
*new* OC ingestion succeeds.

## Remaining action (team-lead ops step — NOT a code task)

Reprocess the text-era quarantine bucket from the archive to recover the ~577K.
The reprocess mechanism this needed is now built (**issue 76**):

    POST /admin/jobs {"kind":"reprocess","reason":"unknown-field-code","detail_like":"%: OC"}

(OC is parse-level — a `notices` row exists in state `quarantined` — so a plain
`process` re-run is a no-op; issue 76's in-place re-parse is required.) This also
flips **issue 35** from `needs-verification` → done. No parser deploy needed
(the OC fix is already in prod); deploy issue 76 first, then run the above.

---

## Original finding (superseded by the root cause above)

## Finding (2026-07-29 snapshot analysis)

`unknown-field-code` = **576,753 quarantined**, and it is a monoculture:
- profile = `text` for ALL of them (the 1993–2010 TED tagged-text era).
- detail = `line NN: OC` for ALL of them — a single field code, **"OC"**, we don't map.

Per ADR-0004 a notice with any unmapped content is quarantined WHOLE, so one unmapped
text-era field code ("OC") is costing us **577K entire text-era notices**. The text era is
intentionally header-only/EN-only for now (CONTEXT.md), so SOME text-era quarantine is expected
— but "OC" appears to be common enough to be worth mapping (or explicitly ignoring if it carries
no header-level meaning).

## To do

1. Identify what the "OC" field is in the TED tagged-text format (crates/ingest/src/… text
   profile / the text-inventory.json checklist). Is it a header field we should map, or
   body/detail content that header-only scope can safely ignore?
2. Either add "OC" to the text-era field mapping, or add it to the ignore/known-but-skipped set
   so it stops quarantining the whole notice.
3. Reprocess the text-era quarantine from the archive → recover ~577K notices.
4. Validate: unknown-field-code count drops ~to zero; recovered text-era notices carry the
   expected header fields; no regression on already-ingested text-era notices.

## Note

This is the second-biggest single quarantine lever after issue 71 (SDK). Cheap if "OC" is
ignorable; a small mapping slice if it's a real header field.
