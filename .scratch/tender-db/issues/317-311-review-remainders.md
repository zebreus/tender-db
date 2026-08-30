# 317 — The 311 campaign's unfinished halves: re-homing, escalations, the medium band

Status: Unit B DONE + Unit C's cheap half DONE (93d8704, 2026-08-30);
Unit A still open
Kind: data quality (organization layer)
Relates to: 311 (produced them), 312, 300

Filed 2026-08-30 because these lived only as a "REMAINING:" paragraph
inside 311's log. The campaign is done; these are what it left standing.

## Unit A — solo-mention re-homing (the fusion repair)

The campaign's flagship finding: a consortium vehicle carrying a lead
member's identifier ALSO captured that member's standalone mentions through
the shared literal (R&K Ingenieure: 7 of 8 mentions are the member alone;
Dobler: 7 solo mentions). Stripping the identifier stopped the fusion from
growing; it did not un-fuse what was already bound. Those mentions still
point at the vehicle org.

Every re-homing candidate is already NAMED: the 823 verdicts in
`311-batch-verdicts.json` carry a `handling` string that says which mentions
belong to which member. This is dissolve-adjacent machinery (re-point a
mention to a different org, mint the member row if absent) and it writes
entity references, so it needs its own adversarial panel round and a
dry-first plan like every other write path in this campaign.

## Unit B — the escalations queue

17 verdicts came back `unclear-escalate` (plus 16 wrong-identifier/low).
Nothing consumes them: they sit in `org_case_reviews`, permanently outside
the apply job's safe subset, with no surface that lists them. Minimum
viable: a report kind (`case-escalations`) listing them with their evidence
so they are reviewable at all, and a policy for what closes one.

## Unit C — the medium-confidence band

102 verdicts are `consortium-vehicle-wrong-identifier` at MEDIUM confidence
— the reviewer's honest "best explanation, single-sourced". They are
correctly not auto-applied. But "parked forever" is not a policy either.

The measured shape (both audit rounds) is that mediums cluster on
register-format impossibility: an FN/HRB/HRA number on a GbR/GesbR-shaped
consortium name, where the reading is decisive about the CLASS but not about
WHICH member holds the number. Two ways to convert them, both worth a
measurement first:

1. **Evidence enrichment** — a register lookup (Firmenbuch/Handelsregister)
   turns "some member's number" into "this member's number" and promotes the
   verdict to high with a citation. External dependency; check licensing.
2. **Structural corroboration** — if the same identifier stands on a
   standalone member row elsewhere in the corpus, that IS the missing
   evidence, and it is a bounded local query. Cheaper; do this one first.

## What NOT to do

Do not lower the apply job's confidence bar to sweep the medium band in.
The bar is why 83 audited cases produced zero false strips, and issue 312
is what happens when an apply action outruns its evidence.


## UNIT B DONE, UNIT C's CHEAP HALF DONE (2026-08-30, 93d8704)

`case-review-backlog` (report kind `case-escalations`) lists every parked
verdict with the evidence needed to close one:

- the case org's CURRENT state, including `gone` when the row was merged
  away since the review — which settles that verdict on its own, and is a
  class nobody had thought to look for;
- counts of every unapplied verdict/confidence pair, so a listed slice is
  never mistaken for the whole backlog (`truncated` says when a list clips);
- and, per case, how many OTHER standing org rows carry the same
  `(identifier_kind, identifier)`, plus one example.

That last count IS Unit C's option 2. A consortium vehicle publishing a
member's register number is the shape the campaign kept finding, so a peer
row holding the same number is the corroboration a medium verdict was
missing — "some member's number" becomes "this member's number", from a
bounded local query, with no external register dependency. The job reports
how many mediums have one; it suggests no merges and writes nothing.

Read-only and stoppable: cancel returns an EMPTY report rather than a
partial backlog, and the STOPPABLE_KINDS contract test carries the new kind.

**Still open here: Unit A** (solo-mention re-homing — the fusion repair).
That one writes entity references, so it needs its own dry-first plan and
its own adversarial panel round, exactly like every other write path in
this campaign. Unit C's option 1 (external register lookup) stays unbuilt
and unneeded until the peer-row measurement says how much of the medium
band it would actually convert — which the backlog job now measures.


## UNIT C MEASURED (2026-08-30, prod jobs 1404/1406)

The first prod run reported **0 of 60** mediums with a peer row — and was
wrong. It compared `(identifier_kind, identifier)` exactly, and the very
first two rows it printed refuted it: org 12524925 `national D1633830016`
beside org 12524926 `vat DE1633830016`. Matching on the DIGIT BODY (8-digit
floor) is the fix, and the re-run reads:

    873 verdicts, 456 applied, 417 parked;
    20 escalations and 111 medium-band cases listed (not truncated);
    7 of the mediums have a peer row carrying the same identifier

Read the 7 rather than the number, because they split two ways:

- **Five name a MEMBER**, which is exactly the evidence a medium verdict was
  missing — the reading goes from "some member's number" to a named company:
  `USTIDDE308082288` → `DE:EUROPEAN DYNAMICS Deutschland GmbH [vat
  DE308082288]`; `USTIDDE814503707` → `DE:regineering GmbH`;
  `USTIDNRDE225141937` → `DE:Eiffage Infra-Nordwest GmbH`; `358568041` →
  `DE:Wenzel Architekt + beratender Ingenieur Part mbB`; and the best of
  them, `vat HRGNUMMER39060355NIEDERLNDISCHESHANDELSREGISTER` — a German
  SENTENCE ("HR-Nummer 39060355, niederländisches Handelsregister") in the
  identifier slot — → `NL:HKV Lijn in Water B.V. [national 39060355]`.
- **One pair is the same vehicle twice** (12524925/12524926, peers of each
  other): evidence of a DUPLICATE VEHICLE ROW, not of a member. Counting it
  as corroboration would be a category error.

**So Unit C option 2 converts 5 of 111** — about 5%, not the band. That is
the honest sizing: the cheap structural probe does not clear the medium
band, and the remaining ~106 need either the register lookup (option 1) or
a per-case re-review with the enriched evidence. Neither is started.

A third class fell out for free: identifiers carrying German LABEL TEXT
(`USTIDDE…`, `USTIDNRDE…`, `HRGNUMMER…`). That is rule-shaped — a prefix
strip in the crosswalk would canonicalize them — and it is measured here at
4 rows in this band alone. Worth a corpus census before deciding.

The escalations list (20, none with a missing org row) is now surfaced
whole; nothing consumes it yet, which stays Unit B's remainder.
