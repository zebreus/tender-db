# 436 — text-era awarding-authority names keep TED's line wrap

Status: RESOLVED-VERIFIED 2026-09-28 — verified after the text re-parse (1598/1599), 434's refresh and the full fold (1616). Was: DEPLOYED 2026-09-27 14:40 UTC (rev `9dedf49`, commit `cef344b`); text re-parse queued as job 1598 (`reclaim_only`), folded by the one full fold. Was: **BUILT 2026-09-27** (see the foot): `AU` and `TW` are names now, space-joined like issue
397's titles. Uncommitted, not deployed. The standing rows need the gated `text` re-parse and fold, and then issue
434's mention refresh to reach `organizations.name`. Filed 2026-09-27.
Kind: defect (ingest — the text-era `AU` rule in `crates/ingest/src/text/rules.rs` and the prose newline join in
`crates/ingest/src/text/parse.rs` `flush`)
Relates to: 397 (the SAME wrapper and the SAME join, fixed for `TI` only; this issue reuses its unwrap), 330 (decided
line breaks in org names are publisher-side, on a comparison that could not see this one; see below), 434 (the
mention refresh the standing org names need after the re-parse), 199 (the wrapper's flush-left tails, already
classed as layout), 232 (`AU` became an Organization), 393 (fixture 108345-1997)

## Verify

    curl -s --max-time 20 https://tenders.zebreus.click/v1/tenders/2247398 | python3 -c "import sys,json; d=json.load(sys.stdin); print([p['organization_name'] for p in d['parties'] if p['role']=='buyer'])"

- **done**: `['ARISTOTELEIO PANEPISTIMIO THESSALONIKIS (APTH), GRAFEIO PROMITHEION, KTIRIO DIOIKISIS']`: one line.
  Needs the `text` re-parse, the fold, and 434's mention refresh.
- **open**: the same name with `\nKTIRIO DIOIKISIS`. This is the expected output, not one I read: this session had
  no prod access. The parse-layer value before the fix is pinned red below.

Tender 2247398 is 108345-1997 (issue 393's table), the committed fixture
`crates/ingest/tests/fixtures/text/1997-can-greek-iso-8859-7.txt`.

## Observed (2026-09-27, prod, the lead session's read)

Text-era organization names carry TED's ~72-column wrap as a literal `\n`:

```
…SOUS-DIRECTION DE LA\nCOMMUNICATION…
RECHNER- UND \nNETZWERKTECHNOLOGIE
```

About **1,000 of every 6–7k organization rows** in the text era's org-id band carry a line break.

## Mechanism

- `rules.rs` gave `AU` the `Prose(None)` rule, and `flush` joins a prose field's head and continuation lines with
  `"\n"`. A wrapped name therefore kept the wrapper's break, and `Emit::authority` put that string into the
  `ORG-1` section the projection mints the buyer from.
- The code comment on that path was wrong. It said `AU` is "one clean line in every vintage the fixtures cover".
  The committed 1997 fixture wraps it (`…GRAFEIO PROMITHEION,` / `KTIRIO DIOIKISIS`), and `rules.rs`'s own
  header says "measured: `AU` wraps its single name".
- Issue 397 found the same wrapper breaking `TI`. It built `Rule::Heading` (space-join and `flatten`) and moved
  only `TI` onto it.

### Why issue 330 did not see it

Issue 330 counted 356,749 line-broken `organizations.name` values. It found **0** "derived-only", because in every
case some mention carried the break too, and it concluded "NOT a parser defect". The test compared the org name
with its mention names. For the text era both come from the same `TXT-AU` value, produced by the same parser join.
A break the parser adds is in the mention as well, so that comparison cannot see it. The 330 verdict still holds
for the eForms/r209 names it sampled (`Vergabekammer Rheinland-Pfalz\nStiftsstraße 9…` is a publisher string). It
does not cover the text era.

## Built (2026-09-27)

Uncommitted, not deployed. `ops/check.sh` was not run. The focused suites below are green.

**Rule.** `Rule::Name` is new in `crates/ingest/src/text/rules.rs`: head plus continuation lines make ONE name,
joined with a space and passed through `parse.rs`'s `flatten`. That is the `Heading` unwrap without a title's
annotation vocabulary, and a name carries no language tag. `AU` and `TW` (the authority's town, also one name)
moved from `Prose(None)` to `Name`. In `flush`, the `Name` arm hands `AU` to `Emit::authority` as before, and every
other `Name` field becomes a text row. The wrong comment is gone. The `Name` doc records the measured shapes.

**The hyphen stays as published.** `RECHNER- UND` / `NETZWERKTECHNOLOGIE` becomes
`RECHNER- UND NETZWERKTECHNOLOGIE`. `Rechner- und Netzwerktechnologie` is a German suspended compound, so gluing
the hyphen to the next line would invent a word. Internal whitespace runs collapse to one space, as they do in
titles.

**Deliberately left as `Prose`:**

- `AB`, `TX`, `OT`: bodies. Their line structure is the document's own.
- `CO`, the contractor list. 154-2005 publishes one supplier per line, so there the break separates values.
  108345-1997's `CO` is a wrapped sentence, but the two cannot be told apart by shape.
- `IA`, a URL or e-mail. A wrap inside an address needs no separator at all, which neither join gives, and no
  fixture wraps one. Measure it before changing it.

**Seen, not fixed.** `CT` (a `PerLine` label) splits a wrapped label into two values. 108345-1997 publishes one
`CC: 3446` and then `CT: Electrical counters, measuring, testing, regulating and control` / `instruments`, which is
two `TXT-CT` rows for one code. A per-line list cannot tell a wrap from a second value without counting its sibling
codes. This is a separate issue if it matters. `TXT-CT` feeds no fold fact.

**Tests.** Red first, then green:

- `tests/text.rs` `a_wrapped_awarding_authority_name_is_one_line` runs on the committed 1997 fixture. It was red
  with `left: "…GRAFEIO PROMITHEION,\nKTIRIO DIOIKISIS"`.
- `text::parse::tests::a_wrapped_name_rejoins_on_one_line` uses inline records. It was red with
  `left: "UNIVERSITAET BEISPIELSTADT, ZENTRUM FUER\nRECHNER- UND \nNETZWERKTECHNOLOGIE"`, which is exactly prod's
  `UND \n` shape. It covers the hyphen break, the `SOUS-DIRECTION DE LA` / `COMMUNICATION` break with a doubled
  space collapsed, a wrapped `TW`, and `CO` keeping its lines.

Also green: issue 397's `a_wrapped_heading_rejoins_before_its_annotation_is_read`,
`the_nature_atoms_leave_the_title_only_when_the_record_carries_the_code` and `a_1993_record_maps_its_coded_header`;
`every_inventory_code_has_a_rule_and_vice_versa`; all of `cargo test -p ingest --lib` (275), `--test text` (13),
`--test fts` (4), `--test project` (66) and `--test data_quality` (9).

**What reaching the standing rows takes:**

1. A **re-parse of the `text` profile** and its fold. This is the same gated operation issue 397 ran, queued in
   the 2026-09-27 re-parse chain. If this change deploys before that chain's text leg runs, the chain carries it
   for free.
2. **Issue 434's mention refresh** (another agent is building it). The fold re-derives mentions, but an
   organization row that already stands keeps the name it was minted with. The corrected mention name reaches
   `organizations.name` only through 434.
3. Then the Verify above.

**Order note.** 432 re-keys `name_norm` (trim and collapse whitespace), so a newline-bearing `name_norm` minted
under the old parse and the flattened one land on the same key once 432 is live. Without 432, the new parse's
mention for an identifier-less authority keys `'…de la communication'` against a stock row `'…de la\ncommunication'`
and mints a twin. Run 432's repair after the text re-parse, which is the order 432 already records.

## Verify read 2026-09-28

`['ARISTOTELEIO PANEPISTIMIO THESSALONIKIS (APTH), GRAFEIO PROMITHEION, KTIRIO DIOIKISIS']` on tender 2247398 (read 2026-09-28 16:0x UTC): one line. **done.**
