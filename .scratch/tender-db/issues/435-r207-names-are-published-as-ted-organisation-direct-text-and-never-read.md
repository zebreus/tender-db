# 435 — R2.0.7 names are published as `TED-ORGANISATION` direct text and never read — ~1.06M nameless mentions

Status: ready-for-agent — DEPLOYED 2026-09-27 14:40 UTC (rev `9dedf49`, commit `94d2346`); reaches R2.0.7 mentions at the full fold via 434's refresh. Was: **BUILT 2026-09-27** (see the foot): `TED-ORGANISATION` is a party name. Uncommitted, not
deployed. The standing mentions need issue 434's refresh AND a fold over the r208 era (the queued re-parse chain's
1597, or a `refold` of `ted-export-r208` if 1597 has already run). Filed 2026-09-27.
Kind: coverage (projection — `ORG_NAME_FIELDS` / `ORG_NAME_FIELD_IDS` in `crates/ingest/src/project.rs`)
Relates to: 434 (without it, no mapping fix reaches a recorded mention), 393 (the r208/r209 re-parse queued as 1596
re-creates every section id, so these mentions survive it), 234 (a nameless mention mints a nameless provisional of
its own — the ~1.05M rows), 307 (the labelled-variant backfill reads `ORG_NAME_FIELD_IDS`), 259 (nested-org repair,
the other nameless-provisional shape)

## Observed (2026-09-27, prod)

~1.06M organization mentions on notices of profile `ted-export-r208` that declare `R2.0.7.*` (published 2010-03 →
2011-09) carry an empty name, and ~1.05M nameless provisional organizations stand behind them (issue 234 never merges
nameless mentions, so each minted its own). Every such notice has the name in its parse layer, under
`TED-ORGANISATION`.

**The 2026-09-15 fan-out filed this under "Refuted — do not re-file these"**
(`.scratch/tender-db/api-dq-review-2026-09-15.md:257`, "R2.0.7 (2011) notices: every organisation mention nameless
because the name is published as TED-ORGANISATION direct text", reason "not recorded"). That was wrong: the claim is
correct as stated, and the fixture below reproduces it.

## Mechanism

R2.0.7 publishes the name as the `ORGANISATION` element's direct text: `<ORGANISATION>Translink</ORGANISATION>`.
R2.0.8+ wraps it: `<ORGANISATION><OFFICIALNAME>…</OFFICIALNAME><NATIONALID>…`. The r209 walker claims both through
one `Rule::TextGroup` (`crates/ingest/src/r209/rules.rs:678`, `r209/parse.rs` `Rule::TextGroup`): non-empty direct
text is emitted under the element's own id, `TED-ORGANISATION`, and the children are walked. The fold's name list
was `ORG_NAME_FIELDS = ["TED-OFFICIALNAME", "TXT-AU"]`, so `mentions()` never read the R2.0.7 value.

The same direct-text shape also names R2.0.8 `OTH_NOT`'s `ADDRESS_NOT_STRUCT` blocks (fixture
`r208/oth-not-000030-2014.xml`, 24 of them) and the 2008 INTERNAL_OJS buyers (`internal_ojs/115165_2008.en`); both
populations were nameless for the same reason and are fixed by the same mapping.

## What to build

1. `TED-ORGANISATION` joins `ORG_NAME_FIELDS` (read by `mentions()`, `has_destination`, the fold-decision list) and
   `ORG_NAME_FIELD_IDS` (the 307 variant backfill's probe list).
2. Check first that no later grammar can publish a competing value in the same section, since a mention's name head is
   first-seen: where R2.0.8+ wraps the name, the `TextGroup`'s direct text is blank, so nothing is emitted under
   `TED-ORGANISATION` there.
3. A fold test over the committed R2.0.7 fixture `r208/f06-r207-070248-2010.xml` (18 inline `ORGANISATION` blocks:
   Translink and 17 winners): no empty mention name.

## Verify

After the r208 era is folded with 434 + 435 deployed, a bounded read of one R2.0.7 window through `/v1/sql` (pick a
notice-id band inside 2010-03 → 2011-09; the corpus-wide count is over the 10 s cap):

    SELECT COUNT(*) FROM organization_mentions m JOIN notices n ON n.id = m.notice_id
     WHERE m.notice_id BETWEEN <lo> AND <hi> AND n.declared_version LIKE 'R2.0.7%' AND m.name = ''

- **done**: `0`, and the fold's job line reads `… recorded mention(s) refreshed` in the ~1M range
- **open**: every R2.0.7 mention in the window (2026-09-27)

## Built (2026-09-27)

Uncommitted, not deployed, not run on prod. Gate (`ops/check.sh`) not run; the focused suites below are green.

**The mapping.** `"TED-ORGANISATION"` added to `ORG_NAME_FIELDS` and `ORG_NAME_FIELD_IDS`
(`crates/ingest/src/project.rs`), with the reason on the constant. No precedence rule was added between it and
`TED-OFFICIALNAME`: the sweep below found no section carrying both. Note the verification is the sweep, not the
grammar alone: "R2.0.8+ wraps it" is true of the structured address blocks, but R2.0.8 `OTH_NOT` still publishes
`ADDRESS_NOT_STRUCT/ORGANISATION` as direct text — with no `OFFICIALNAME` beside it, so the two ids never compete in one
section there either. Side effects checked: `buyer_key` now sees R2.0.7 buyer names, but it only feeds the
placeholder-key election refusal (`key_shaped`, eForms keys), which legacy notices never reach; the text parser's note
at `text/parse.rs:1605` ("`ORG_NAME_FIELDS` … reads only `TED-OFFICIALNAME`") is now one id short. That file is under
another agent's edit, so it was left alone.

**Tests** (`crates/ingest/tests/project.rs`):
- `the_r207_fixture_folds_every_organisation_block_with_its_name`: the F06 folds 18 mentions, none nameless, no
  nameless organization, Translink / Amtrain Midlands Ltd / Interfleet Technology Ltd each once under GB, and a party
  row names Translink.
- `a_refold_names_the_standing_r207_mentions_and_moves_their_parties` (with 434): prod's stock, reproduced by folding
  the fixture without its `TED-ORGANISATION` rows (18 nameless mentions on 18 nameless provisionals, party rows on
  them), then re-parsed with them and refolded: 18 refreshed, 18 re-bound, no nameless mention or nameless party left.
  The 18 nameless provisionals are left mention-less; see 434's follow-up.
- `no_legacy_org_section_publishes_two_name_ids`: every committed r208 / r209 / internal-OJS fixture parsed, no section
  carries both `TED-ORGANISATION` and `TED-OFFICIALNAME`, and the sweep saw both shapes.

**Red first** (the mapping line reverted, together with 434's early return, then restored byte-identical):
`the_r207_fixture_folds_every_organisation_block_with_its_name` fails with 18 nameless mentions, and
`a_refold_names_the_standing_r207_mentions_and_moves_their_parties` refreshes 0 of 18. The sweep passes on both, as a
property of the parse layer should. Green: all three, and the full ingest `project` suite (72/72).
