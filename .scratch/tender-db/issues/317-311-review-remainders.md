# 317 — The 311 campaign's unfinished halves: re-homing, escalations, the medium band

Status: Unit B DONE. Unit C's cheap half DONE (93d8704); its medium band is
still parked. Unit A's machinery, packet and REVIEW CAMPAIGN all DONE
(2026-08-30): 416 mentions re-homed, folded, and the packet reads EMPTY.
What remains of Unit A is 93 parked verdicts, issue 322's rename repair, and
issue 323's slow write
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


## UNIT A MEASURED (2026-08-30, prod job 1411)

`fusion-census` derives the fusion shape from the data instead of from the
campaign's prose:

    456 applied case rows, 456 with mentions;
    102 hold at least one mention naming somebody ELSE, over 585 such mentions

The signal reproduces the campaign's own cases without being told about
them. Org 9610149, "Bietergemeinschaft Dobler / Oberall", holds 20 of 21
judgeable mentions naming `Dobler GmbH & Co. KG Bauunternehmung` — the
Dobler case the pilot found by reading. Org 13011025 holds **28 of 29**
naming `Dipl.Ing. Wilhelm Sedlak Gesellschaft m.b.H.`.

**585 is an UPPER BOUND on the re-homing workload, not the workload.**
Reading the candidates, off-name mentions split two ways:

- **The fusion proper** — the member alone, repeatedly: `MIV GmbH` (14
  mentions) under a vehicle named for the full "Mecklenburgisches
  Ingenieurbüro für Verkehrsbau"; `HERMANNS HTI-Bau GmbH u. Co. KG` (15);
  `a+r Architekten GmbH` (10). These are the ones to re-home.
- **The vehicle under another spelling** — e.g. org 22559285's second group
  is `Bietergemeinschaft Hermanns HTI-Bau GmbH u. Co. KG / Birckha…`, which
  IS the vehicle, just longer than the head name, so N2 does not collapse
  it. Re-homing those would be a fresh error.

Separating them is exactly the per-case judgement the census refuses to
make, and it is why this stays a review campaign rather than a rule.

**A class the census surfaced that the campaign did not name.** At 28-of-29
and 20-of-21, some of these rows are not vehicles holding a member's
mentions — they are the MEMBER'S row wearing a consortium name. That is the
existing `member-row-mislabelled` verdict (58 cases), and for those the
repair is not re-homing at all: it is renaming the row to the member and
letting the vehicle be minted separately, if it is ever needed.

**Sizing**: 102 cases at the batch-v2 rate (~5.3k tokens/case) is about
540k tokens — the cheapest campaign on the board by an order of magnitude,
and the one that repairs records the API is currently serving wrong.

Still not built: the re-homing writer itself. It re-points mention rows
between orgs, which is dissolve-adjacent, so it needs a dry-first plan and
its own panel round — the bar every write path in this campaign has met.


## UNIT A MACHINERY LANDED (2026-08-30, 2ac5c1f) — read this before the campaign

`org_mention_rehoming` records one decision per mention, keyed
`(notice_id, section_id)` exactly as `organization_mentions` is, so a
mention carries one verdict and double-recording is impossible by
construction. `apply-rehoming` executes the safe subset: `rehome`, HIGH
confidence, and a `target_org_id` that EXISTS and is not the org the mention
already sits on.

**The design's centre, after a 13-finding panel round: ONLY THE MENTION
MOVES.** Party rows, bid-party rows and winner rows are all products of the
fold, and hand-moving them was wrong three separate ways — winners
accumulate across award rounds so a same-version party join collected other
mentions' awards; party facts are superseded per role so a carried-forward
winner had no party row to be found through; and a nested org section
aliases to its outer mention, so a party row's section id need not be its
mention's. Two of those were reproduced end to end against the real
projection. So the mention is corrected, the affected tenders are stamped
stale and their notices re-queued, and the fold rebuilds the rest.

**Consequences for whoever runs the campaign:**

1. **Run a `project` job after the wet run.** Until the fold runs, the
   derived layer still shows the old attribution. The job's `parties` and
   `bid_parties` counts are a BLAST RADIUS — what the refold will rebuild —
   not rows it moved.
2. **v1 never mints a destination.** A member with no standing org row is
   `missing_target`, counted and left alone. That count is the measurement
   that decides whether minting is worth building.
3. **The verdict schema needs the distinctions the census found**: the
   fusion proper (re-home) versus the vehicle under a spelling N2 does not
   collapse (keep), and the member-row-mislabelled shape at 28-of-29, where
   the repair is renaming the row rather than moving mentions off it.
4. **Issue 321** is the known residue: the name variant stays on the origin
   org, which keeps feeding the Stage-4 keys.


## UNIT A: THE REVIEW PACKET (2026-08-30) — the campaign's missing input

The machinery could RECORD a verdict and EXECUTE one; nothing produced the
input a verdict needs. `fusion-census` publishes names and counts, and a
verdict needs an **address** (`notice_id`, `section_id` — how both
`organization_mentions` and `org_mention_rehoming` are keyed) and a
**destination** (`target_org_id`, a standing row). Neither is derivable from
a count, so the campaign could not start.

`rehoming-packet` (read-only, stoppable, report kind `rehoming-packet`) is
that input. Per open case: the vehicle's own row, its judgeable/off-name
ratio, the off-name mentions ADDRESSED, and per name group the standing rows
it could move to — found through the `org_match_keys` N2 satellite, which is
the only indexed path from a name to the rows carrying it.

**It is resumable by construction.** A mention carrying a decision is
excluded, so the packet is what is LEFT and the campaign runs in batches
without re-reading itself.

### What the adversarial panel changed (19 of 25 findings confirmed)

Five reviewers, one verifier per finding, each writing and running scratch
tests against the real code. The first cut of this job was wrong in ways
that would have wrecked the campaign rather than merely annoyed it:

- **The destination probe was an unordered `LIMIT 12`.** For any name carried
  by more than twelve rows it returned the twelve LOWEST ORG IDS — not the
  established row, not a sample. A verifier measured a key with **62,084**
  carriers. Replaced with the genericness wall FIRST (a bounded distinct
  count at `SCAN_STOPLIST_CAP`, threaded from the app so this wall and the E3
  scan's cannot drift): over the wall the group is published as a **shared
  literal with NO destinations**, because five arbitrary rows that look like
  a shortlist are worse than none.
- **A verdict the apply job can never execute vanished from the packet** —
  below the confidence bar, no target, or a target that no longer stands. The
  campaign would have read "no open work" while nothing moved. Those now come
  back as `parked`, with the reason. The panel also proved the naive fix
  wrong: a `keep` is never stamped by `apply_rehoming` (three consecutive wet
  applies, `applied_at` still NULL), so testing `applied_at` alone parks every
  keep forever.
- **Country was carried and never used.** A same-named foreign row outranked
  the domestic one and was then truncated away.
- **The case cap kept the SMALLEST cases** — the first `cases_cap` in org-id
  order — and then presented them sorted by size, so an arbitrary subset read
  as the worst. Now two passes: pass 1 sizes every case holding three numbers
  each, pass 2 probes only the ranked head.
- **An org reviewed under two cohorts was walked twice**, doubling its
  mentions in every total and listing the same address twice.
  `org_case_reviews` is keyed `(case_org_id, cohort)`. Fixed here **and in
  `fusion_candidates`**, which had it too — the census and the packet must
  agree on the same data.
- **No `org_match_keys` preconditions.** An index-less, mid-walk or
  wrong-epoch satellite does not make the packet visibly wrong; it makes
  every group read "no destination anywhere", which is the one answer a
  reviewer cannot tell from a real finding. The job now refuses like
  `scan-org-match-keys` does, and stamps the build's provenance
  (`keys_built_at`, `keys_rows`, `keys_epoch`) into the report — because
  preconditions cannot catch STALENESS, which is the same silence arriving
  later.
- **`groups_total` was a post-cap listed count published as a total.** Counts
  are now scoped by name: `groups_total` over the whole workload,
  `probed_groups` / `probed_groups_with_target` / `groups_generic` over what
  was probed, `groups_elided` per packet and per case.
- Smaller, all confirmed: the group's displayed name was the first spelling
  seen rather than the modal one; alias-matched destinations did not say so;
  the mention's own published identifier was dropped although the row was
  already being read; the mid-case stop checkpoint and every truncation path
  were untested.

### What the packet now hands a reviewer, and why each piece is there

The census left the hard judgement unsupported: the fusion proper (re-home)
versus the vehicle under a spelling N2 does not collapse (keep). Three pieces
of free evidence separate them, all from rows already being read:

1. **`notice_orgs`** — how many organizations the mention's own notice names.
   A notice naming ONE organization, under the member's name, is the fusion.
   A notice naming seven is a consortium listing.
2. **`identifier_match`** — the destination's identifier shares its digit
   body with one the mention publishes (the Unit C peer comparison, reused).
   It outranks a bigger namesake.
3. **the `mentions`/`off_name` ratio**, counted over EVERY mention on the row
   so it does not move as the campaign runs — 28-of-29 is issue 322's
   mislabelled-row shape, where the repair is a rename, not a move.

### Still open

The campaign itself. Run `rehoming-packet`, review, POST `/admin/rehoming`,
`apply-rehoming` dry, review the plan, wet, **then run a `project` job** —
until the fold runs the derived layer still shows the old attribution.


## UNIT A CAMPAIGN RUN (2026-08-30) — 416 mentions moved

Prod `rehoming-packet` (job 1413, under a keys satellite built 21 hours
earlier: 13,034,812 rows, epoch `n2v1+n3v1`):

    456 applied case orgs, 102 still hold an UNDECIDED off-name mention,
    over 585 such mentions in 181 name groups;
    115 groups have a standing destination, 3 are a shared literal over the wall

Thirteen reviewers took eight cases each; thirteen AUDITORS then re-checked
every verdict, with instructions biased at the appliable ones. **585 verdicts,
585 audited, 4 overturned — 0.7%.**

### The verdicts

| shape | n |
|---|---|
| `rehome` high, with a target — **the appliable set** | 416 |
| `rehome` medium, with a target (parked: reading right, row uncertain) | 29 |
| `rehome` medium, name only (`missing_target`) | 60 |
| `rehome` high, name only (`missing_target`) | 4 |
| `keep` high — the vehicle under a spelling N2 does not collapse | 56 |
| `keep` medium | 20 |

**39 of the 76 keeps are `member-row-mislabelled`** (issue 322) — the row IS
the member's, wearing a consortium name, and the repair is a rename. That is
the class the packet was designed to surface rather than bury, and it is a
third of everything kept.

**64 mentions name a company with no standing row at all.** That is the
measurement `apply-rehoming` v1 defers minting in order to take: about 11% of
the workload would need a row minted, which is not nothing and not most.

### The audit's best catch

A reviewer sent `]init[ AG` to org 3525816 `init AG` (34 mentions) at HIGH
confidence, applying the "same company duplicated → take the largest" rule.
The auditor stopped it: `match_norm` drops the brackets, so `]init[ AG` (the
Berlin digital agency, whose HRB 73218 B the mention publishes) and `init AG`
(init innovation in traffic systems SE, Karlsruhe — a separate listed company)
land in ONE key. Positive evidence they are different: the two candidate rows
that carry identifiers carry DIFFERENT ones. Corrected to the largest
bracketed row and downgraded to medium, so it parks instead of writing an
award onto the wrong listed company.

That is the exact failure mode the duplicate-consolidation rule creates, and
one pass of adversarial re-reading caught it. The other three overturns were
two consortium names wrongly kept and one target dropped in favour of
`missing_target`.

### The run

`apply-rehoming` dry (job 1414) → plan reviewed → wet (job 1415):

    416 mentions moved (1326 party and 2955 bid-party rows follow them
    across 258 tenders, whose 1710 notices are re-queued so the fold
    re-derives the winners); 0 no-ops, 4 missing_target

Plan review before the wet run checked three things the counts cannot show:
every target id was one the packet actually offered for that case (585/585,
no invention, no self-targets); no self-moves; and the seven moves whose
DESTINATION is itself a `Bietergemeinschaft` row — all seven correct, and a
shape worth naming: **a different consortium fused onto this row through the
shared lead partner's identifier**, moved to its own exact-named standing row
(Pfeiffer/Lange/Braumann/Echterhoff off the Pfeiffer/KASSECKER row;
Tecton/Mohr off Tecton/IBBS; MN Glasfaser/MUENET off Muenet/epcan).

Then a `project` job to re-derive the winners. Until it finishes the derived
layer still shows the old attribution.

**The wet run took 1,185 seconds for 416 row moves** — the dry pass computes
the identical plan in under one second, so the whole of it is the re-queue
write. Filed as issue 323; it holds the single writer for twenty minutes.

### The repair, measured after the fold

`project` closed the window in 32 seconds: **1,710 notices → 258 tenders
(2 islands), 1,710 versions; 258 tenders written, 0 verified unchanged** —
every touched tender's winner set re-derived from the corrected mentions.

Re-running both jobs against the repaired layer (jobs 507/508):

    fusion-census:    102 → 45 rows hold an off-name mention, 585 → 169 mentions
    rehoming-packet:  0 open, 492 decided, 93 PARKED

And the four numbers reconcile exactly, which is the acceptance test:
416 moved + 76 keeps = 492 decided; 76 keeps + 93 parked = the 169 off-name
mentions the census still sees; 492 + 93 = 585. **The packet is empty.**

The 45 rows the census still reports are not a residue of this repair — they
are the decisions: rows a reviewer read and left alone (the vehicle under
another spelling; the mislabelled member rows of issue 322) plus the 93
verdicts nobody can apply yet, which is what `parked` exists to keep visible.

### What is left on Unit A

- **93 parked verdicts.** 64 name a company with no standing row
  (`missing_target` — the minting question, now sized); 29 name a row but at
  medium confidence, where the reviewer was sure of the reading and not of
  the row. Both need a second pass with better evidence, not a lower bar.
- **39 mislabelled rows** → issue 322 (the rename repair, unbuilt).
- **Issue 321** — the name variant left on the origin org — is now live for
  416 moves, and it feeds the Stage-4 keys.
- **Issue 323** — the re-queue write is 20 minutes for 416 row moves.
