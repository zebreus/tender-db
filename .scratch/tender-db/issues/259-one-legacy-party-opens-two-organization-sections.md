# 259 — one legacy party opens TWO Organization sections, so the winner has no name

Status: HALF-LANDED (acceptance checked 2026-08-27 ~23:0x against THE epoch
refold): the WINNER DEDUP half landed — exemplar 362996-2018 now has ONE
winner row (was two; satellites are rewritten per version on refold). The
NAME half did NOT land: the winner still points at the nameless provisional
org (12000087), because `resolve_mentions`' idempotency map keeps an
already-recorded (notice, section) on its Organization on re-projection BY
DESIGN — the refold never re-routes stale mention bindings, and the old
inner-section (ORG-3) mention rows persist. The "Landing: needs a refold, not
a re-parse" analysis below was wrong about the mention layer. Corpus check:
1,432,504 nameless provisional organizations still stand.
NEXT (BUILT 2026-08-27 ~23:3x, pending deploy + run): `repair-nested-orgs`
admin job (`Db::repair_nested_org_mentions_batch`) — walks the 1.43M nameless
provisionals; where one is the single-mention outer wrapper of a nested party
pair (ancestor-chain membership like `nested_org_aliases`, so any depth,
non-party intermediates, and sdk-0.1's ContractingParty/WinningParty kinds all
match) whose descendants name exactly ONE distinct org, it repoints every
reference onto that named org via the merge machinery (winner dedup included),
deletes the empty row, and emits removed/changed/tender-changed events.
`dry_run` defaults TRUE (preview first, on prod too). Skips are counted, so
the residue is measurable. Adversarially reviewed (5 findings fixed: rollback
discipline, chain-walk coverage, sdk-0.1 kinds, shared repoint helper with the
234 merge, dry-run). RUN ORDER on prod: dry-run preview → read counts →
confirmed run → re-check the exemplar (362996-2018 winner must read "Opal
Publicidade, S. A.") and the nameless count. Original status: DIAGNOSED +
FIXED 2026-08-20 (owner), found under 234's tail check.
Kind: canonical identity defect (winner names lost, winner counts inflated)
Blocked by: —
Relates to: 234 (whose nameless population this creates — read 234's decision note first), 100/257
(the same "award names nobody" symptom, different mechanisms), 04 (provisional organizations),
251 step 4 (the r208/r209 re-parse this should ride with)

## What

The legacy TED vocabulary declares BOTH the party wrapper and the address block inside it as
`Rule::Org` (`crates/ingest/src/r209/rules.rs`): the Org list holds `ADDRESS_WINNER` **and**
`WINNER`. So an F13 prize block opens two Organization sections for one company:

```text
<RESULTS>                       -> LotResult    RES-1
  <AWARDED_PRIZE><WINNERS>
    <WINNER>                    -> Organization ORG-2   (carries no values of its own)
      <ADDRESS_WINNER>          -> Organization ORG-3   (OFFICIALNAME lives here)
        <OFFICIALNAME>Opal Publicidade, S. A.</OFFICIALNAME>
```

`mentions()` collects a section's values "keyed by the enclosing Organization" via `enclosing()`,
which returns the NEAREST one — so the name binds to the inner ORG-3. The result block's winner
reference names ORG-2. Consequences, both live in production:

1. **The winner has no name.** ORG-2 mints a provisional Organization with an empty name, and the
   award points at it. The real party sits on ORG-3, a sibling row nothing references.
2. **The winner count is doubled.** The results reader picks up both Organization sections under the
   block, so a single-winner award reports TWO winners. Caught by the red test at
   `left: 2, right: 1` before the fix.

## Evidence

Verbatim from the archive, `ted/monthly/2018-08.tar`, member `20180818_158/362996_2018.xml`
(publication `362996-2018`, notice 19966469) — now committed as
`crates/ingest/tests/fixtures/r209/f13-prize-winner-362996-2018.xml`:

    …<RESULTS><AWARDED_PRIZE><DATE_DECISION_JURY>2018-06-27</DATE_DECISION_JURY>
    <PARTICIPANTS><NB_PARTICIPANTS>1</NB_PARTICIPANTS></PARTICIPANTS>
    <WINNERS><WINNER><ADDRESS_WINNER><OFFICIALNAME>Opal Publicidade, S. A.</OFFICIALNAME>…

The stored parse layer for that notice, which shows the shape rather than argues for it:

    section_id  kind          parent
    RES-1       LotResult     PROCEDURE
    ORG-2       Organization  RES-1        <- referenced as the winner, zero values
    ORG-3       Organization  ORG-2        <- TED-OFFICIALNAME = "Opal Publicidade, S. A."

Not a one-off. The same topology on 19966470 (`Cryseia, Animação Turística…`, PT) and 19966471
(`Groupement Atelier d'architecture et d'urbanisme M. Boudry & P. Boudry`, FR).

**Scale**, from the windowed counts taken for issue 234: nameless provisional Organizations run
1,953 / 398 / 2,051 / 5,077 per 200k-row window across the legacy id space (and 0 in the newest
windows, which are eForms and text-era). Their party roles in one sampled window:

    DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR   2,411
    winner                                         153
    PURCHASING_ON_BEHALF_YES / … / PARTY_NAME_ADDRESS   8

So these are **awarded contractors**, not harmless empty rows — which is why this outranked the merge
question it was found under.

## Why no test caught it

No fixture in the corpus contained a `WINNER`/`ADDRESS_WINNER` pair. The committed r209/r208 award
fixtures use `CONTRACTOR` > `ADDRESS_CONTRACTOR`, and `CONTRACTOR` is a *transparent container*, not
`Rule::Org` — so it does not nest and the defect is invisible on them. `WINNER` is the only
non-`ADDRESS_*` entry in the whole Org list, which is exactly why it is the one that nests.

## The fix

Nesting is the signal that two sections are one party — an Organization is not a container for other
Organizations in any era's vocabulary. `nested_org_aliases()` maps every Organization-kind section
that sits inside another one to the **outermost** Organization above it, and:

- `mentions()` builds a mention only for outermost sections, and routes a value's owner through the
  alias, so `Opal Publicidade, S. A.` reaches the party the award references;
- `bind_organizations()` extends `by_section` with the aliases, so a reference naming EITHER end of a
  nest binds to the one Organization — and the existing `sort/dedup` collapses an award that
  references both ends into one winner instead of two.

eForms is untouched by construction: `efac:Organization` sections are siblings under
`efac:Organizations`, which is not itself an Organization, so the alias map comes back empty. Pinned
by an assertion in the unit test rather than left as a claim.

## Gates

- `a_legacy_prize_winner_resolves_to_the_party_that_has_the_name` (ingest/tests/project.rs) — the real
  payload end to end: one lot_result, ONE winner, named `Opal Publicidade, S. A.`, and zero
  empty-named Organizations. Verified red before the fix (`left: 2, right: 1` on the winner count).
- `a_nested_organization_aliases_to_the_outermost_one` (ingest/src/project.rs) — the two things a
  fixture cannot reach: a THREE-level nest must land the innermost on the OUTERMOST (an alias pointing
  at another alias would resolve to a section that mints no mention), a sibling party under the same
  result is untouched, and the eForms shape produces an empty map.
- `every_r209_fixture_is_consumed_exhaustively` now covers the new fixture, so ADR-0004 holds on it.

## Landing

Needs an **r208/r209 refold** — the parse layer is unchanged (the sections and values were always
there; the projection read the wrong one), so this is a refold, not a re-parse. Issue 251 step 4
already owes that era a re-parse for the VAT basis; if that runs first this rides on it for free.

Expected effect, stated in advance so it can be checked rather than admired: the nameless provisional
Organizations in the legacy id ranges should approach zero, `organizations` should shrink by roughly
one row per nested party, and legacy award winner counts should FALL (the doubling goes away) while
the share of winners carrying a name RISES. If the winner count does not fall, the dedup is not
firing and I should find out why before believing the name numbers.

## Not done here

Whether the ~200k already-minted nameless Organizations are deleted or simply orphaned by the refold
is not decided. Issue 103 already records that a shrinking rewrite orphans entity rows rather than
removing them, and its tripwire was "the first removed or narrowed mapping" — **this is that change**,
so 103 is now reachable and should be read alongside this.
