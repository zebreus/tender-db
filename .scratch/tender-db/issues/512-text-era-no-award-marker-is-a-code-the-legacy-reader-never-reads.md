# 512 — the text era's "all tenders rejected" marker is stored as a Code, and the legacy reader only reads an Integer

Status: done — 2026-10-11, deployed 6d237a7 (fix 4791168). Sizing job 2188: 9 text notices carry the Code marker. Drain: job 2201 `refold-fields` (`expect` 9: re-queued 9, stamped 8 Tenders) → 2202 fold: **2 Tenders corrected** — the dated rejections that read NULL now read `clos-nw`; the other 7 had no award date and already read `clos-nw` by default.
Was: ready-for-agent — filed 2026-10-10 from issue 510's adversarial build review.
Kind: data quality (results; text era)
Relates to: 257 (the "silence is not closure" rule), 244 slice 9 (the text award skeleton), 510 (unit1-decision §3's
text-era note, corrected by this finding)

## What is wrong

`text::parse::claim_award_skeleton` (crates/ingest/src/text/parse.rs, the `if rejected` arm) pushes
`TED-NO_AWARDED_CONTRACT` on its bare `RES-1` as `NoticeValue::Code { code: "1" }`. The projection's
`read_legacy_results` reads the marker only as `(LEGACY_NO_AWARD_MARKER, NoticeValue::Integer(_))`, and
`has_destination` lists it on `Channel::Integer` only (the XML eras' `Rule::Marker` stores `Integer(1)`, which is
what the constant's doc describes). So for the text era the marker never takes effect:

- an award body that says every tender was rejected AND states an award date mints `RES-1` with the date and the
  Code marker; the fold reads no decision, no winner, no value and a date, so `decision` stays NULL instead of the
  `clos-nw` the parser's doc promises ("rejection phrase → `TED-NO_AWARDED_CONTRACT` here → decision `clos-nw`");
- without a date it reads `clos-nw` only because that is the default for a result with no evidence.

No test sends a text rejection through the fold: the parse test checks the field id is present, not its channel.

## Proposed fix

1. **Measure.** Count the text notices carrying `TED-NO_AWARDED_CONTRACT` in `notice_codes`, and how many of
   those also carry `TED-CONTRACT_AWARD_DATE` on the same section (the NULL ones). A bounded read by
   `field_id` equality through `/v1/sql`, or a `refold-fields` dry count.
2. **Build.** Accept the Code spelling in `read_legacy_results` (`(LEGACY_NO_AWARD_MARKER, NoticeValue::Code { .. })`
   → `clos-nw`) and add it to `has_destination` under `Channel::Code`. A refold is enough; emitting `Integer(1)`
   from the parser would need a text reparse instead. A projection test with a text award body that has a
   rejection phrase and a date (expect `clos-nw`).
3. **Drain.** `refold-fields` over the field, `expect` from unit 1.

## Verify

The unit-1 count of dated rejected text results with `decision IS NULL` reads 0 after the drain.
