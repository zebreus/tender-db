# Changelog — the public /v1 API

Behavior changes a client could observe, newest first. Additive fields and new
endpoints land without an entry unless they change how an existing request
answers; this file exists for the rare case where one does.

## Unreleased (issue 526) — 1993–2003 winner lists keyed by lot are read one winner per lot

The oldest award notices list their winners lot by lot — `Lot 1) Blin. Lot 2) Prodirest.`, `Lot 1: Spie
Trindel; lot 2: Philips`, `lots 7, 8, 9 and 10: Carabelli`, `10) et 11) Maquet`. These lists were read as one
organization per list or per comma (`Lot 1) Blin. Lot 2) Prodirest. …`, `lots 7`, `et 11) Maquet`), so a company
was split across one organization per lot-number spelling. After the re-parse of the 1992–2003 notices:
- each lot's winner is served under the company's own name (`Blin`, `Prodirest`), merging with the same name
  elsewhere;
- a lot reference is never part of a name, and a slot that names only a lot (`Lot 3`), a pointer (`Lot 2) Idem`),
  a status (`en cours`, `were not awarded`), a price or a lot's lower-case description (`Lot 3) papier, Sopaco`)
  is no organization;
- a keyed award sentence names its awardee (`lot 3) la société Storz a été retenue` → `Storz`).

On a 912-notice sample, names carrying a lot reference fell from 4,423 to 208. Lettered and Roman lot labels
(`Lot A)`, `Lot III:`, about 250 organizations) are kept as published. The prefixed organizations are removed
(`organization removed`), and the affected Tenders publish `tender changed`.

## Unreleased (issue 528) — Dutch award prose in a winner slot is read as what it says

About 130 organizations were named after Dutch award prose sitting in a winner slot rather than after a company.
Each of these reviewed rows is now read as what it says:
- one awardee named in prose is served under the awardee's name (`De opdracht is gegund aan: Tebodin BV` →
  `Tebodin BV`), and a company name carrying an award note serves without it (`De Beijer Bladel BV (6 percelen
  gegund)`). Most of these old notices carry no country, so the awardee is a new organization rather than an
  existing one of the same name;
- a pointer, a count, or an award still to come is no organization (`Zie bij nadere inlichtingen, 3 aannemers zijn
  gegund`, `Dit perceel wordt later gegund.`), and the result keeps its decision;
- a statement that nothing was awarded is no organization, and the result reads `decision: clos-nw`.

Lists of several awardees are kept as published. The prose organizations are removed (`organization removed`), and
the affected Tenders publish `tender changed`.

## Unreleased (issue 527) — a tenderer the notice marks "niet gegunde partij" is not served as a winner

When TenderNed re-publishes an award with each tenderer marked after its name — `(gegunde partij)` for the one
awarded, `(niet gegunde partij)` for the others — the marked losers are no longer served as that notice's
result's winners, even where its contract lists their tenders. They keep their bids (`?bidder=`), and both marks
are dropped from the names served for those mentions. One tender carried the marks when this shipped (Gemeente
Maastricht, 00798764-2024). Its earlier, unmarked award (00792573-2024) still lists every tenderer as a winner,
and an organization first seen under a marked name keeps that name as its served name, until later work.

## Unreleased (issue 520) — an award summary that names one winner binds to that winner

Issue 510 kept, as published, a winner "name" that states a void lot but also seemed to name an award. Three
kinds of them have changed. Void prose that names no award after all (`Le lot 3 : … a été déclaré infructueux`)
is now treated like any other "not awarded": its organization is removed and its result reads `decision: clos-nw`
(about 170 organizations). A summary that names exactly one awardee now binds its winner mention to that awardee's
organization, which takes the winner role. The awardee is named either in its award clause (`Lot déclaré
infructueux … puis attribué à la Société Arrow Génériques suite à procédure négociée` → `Arrow Génériques`) or
before a void tail (`BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les
parts non couvertes des besoins` → `BGE Guyane`; `GRPT GCC (mandataire) et Andre BTP, suite à procédure négociée
après AO infructueux` → the consortium). That is an existing organization of the same name and country where one
exists, otherwise a new one; the result stays `selec-w`, and the summary organization is removed. Summaries that
list several winners, and any summary whose reading is not certain, are still kept as published.

## Unreleased (issues 513, 519) — an NHS body is one organization under its ODS code, its PPON and its sub-codes

An NHS trust, ICB or other body that Find a Tender publishes under its letter-only ODS code (`GB-NHS-QWU`) in some
notices and under its PPON in others, or under several spellings of its name, stood as several organizations, and the
code itself found none of them. Each allowlisted body on the frozen ODS list is consolidated once a reviewed verdict
for it is applied: its rows fold into one survivor (its PPON organization where it has one), every folded row
publishes `organization removed` and answers `308` to the survivor, `?identifier=` with the code answers the survivor,
and later mentions under any of the reviewed spellings bind to it — by name, never by the code: a coded mention under
a spelling no review read stands as its own organization until the next review (issue 513; 171 bodies so far). Since
issue 519, a sub-ICB location or trust site that publishes its own child code while naming itself as its parent body
(`NHS Coventry and Warwickshire ICB - B2M3M`) folds into the parent the same way: its child code also answers the
parent, and a later mention under that code binds to the parent only when its name passes the same strict check the
review used — any other name stays on its own organization, which `?identifier=` then returns beside the parent.
Nothing folds until each body is reviewed, so the change reaches `/v1` as the review runs.

## Unreleased (issue 469) — a UK college, charity or society published under its PPON is merged into its register entry

A UK body that Find a Tender publishes under its Central Digital Platform PPON in some notices and under its register
number in others — a college's UKPRN, a charity's Charity Commission, OSCR or CCNI number, a housing association's
FCA Mutuals number — stood as two organizations. Pairs that a notice states together for one party, that the register
confirms or a reviewer reads, are now merged into the register-numbered organization: the PPON organization publishes
`organization removed` and answers `308` to the survivor, and later PPON-only mentions bind to the survivor. NHS codes
are not part of this yet. Nothing merges until each pair is reviewed, so the change reaches `/v1` as the review runs.

## Unreleased (issue 491) — text-era award notices serve each contract's own figure

Award notices of the 2004–2010 text era now serve each contract's own figure (its "total final value of the
contract") as that award's `awarded` value, where the notice heads its awards and the figure belongs to exactly one
named winner. Before, that figure was read only as the notice's total, and only when the notice awarded a single
contract and stated no overall total. The contract figures are now partners of the existing scale rules, so a notice
total of €1 bn or more that is exactly 10ᵏ× one of its contracts' figures (×100 included) can be refused: a €35 bn
total whose one contract states €35 m is no longer the Tender's `value`, which falls to its next accepted figure or to
none (refused is not corrected). A total that is close to the sum of its contracts is kept, as a framework total over
lots is. No other Tender `value` moves, and `amounts` gains no rows; the contract figures appear only on the results.
The affected Tenders are re-folded and announced as `tender changed`.

## Unreleased (issue 517) — `/v1/changes` refuses a `cursor` parameter

The change feed resumes from `since`. A request that sent `cursor` instead (the list routes' parameter) was answered
from the very start of the log, with `ignored_filters: []`, as if the client had asked for everything. It is now a
`400` naming `since`. `since` itself is unchanged, including its deliberate lenience: a value that does not parse
still reads from the start.

## Unreleased (issue 516) — a supplier that spells out "Public Limited Company" is no longer split from its plc

The Companies House ↔ PPON merge arm (issue 448) read `PUBLIC LIMITED COMPANY` and `LIMITED LIABILITY PARTNERSHIP`
as "Limited", so a supplier published as `X PUBLIC LIMITED COMPANY` under one identifier and `X plc` under the other
was refused as a form conflict and stayed two organizations. The spelled-out forms now read as `plc` / `LLP`; such
pairs merge once reviewed, and the merged organization publishes `organization removed` with a 308 to the survivor.

## Unreleased (issue 512) — a text-era award body that says every tender was rejected reads "closed, no award"

Award notices of the 1993–2010 text era that state an award date and say every tender was rejected now serve
`decision: clos-nw` on their result, as the same statement already did in the XML eras. They served no decision
(NULL) before: the rejection was recorded but never read.

## Unreleased (issue 510) — "the lot was not awarded" is no longer served as a winner

A winner slot that says the lot was not awarded (`Infructueux`, `Sans suite`, `Desierto`, `Lotto deserto`,
`Niet gegund`, `Not awarded`, …) is no longer an organization. Such a party is not served as a winner, buyer or
any other party, and an older-format award whose only "winner" it was now reads `decision: clos-nw` instead of
`selec-w`. About 17,000 organizations made of these phrases (one alone, `infructueux`, held over 3,000 winner
mentions), in every language including TED's own multilingual "not awarded", have been removed; each published
`organization removed`, and every Tender that served one published `tender changed` (with `lot changed` for its
affected lots). eForms results keep the publisher's own decision code. A "name" that states a void lot but also
names an award — a per-lot summary such as `Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré
infructueux`, or `infructueux, relancé et attribué à …` — is kept as it was.

## Unreleased (issue 506) — an award notice's framework values per lot are kept, and refuse scale slips

eForms award notices state a framework maximum and a re-estimated value per lot result (BT-709, BT-660). They are
now served in `amounts` at their lot under two new `field` values, `result_framework_maximum` and
`result_framework_reestimate`; they are never elected as a Tender's or a lot's `value`. A figure of €1 bn or more
that is exactly 100× (or 10ᵏ×) one of them can now be refused as a scale slip under the existing rules (their
corroboration and framework-total exemptions are unchanged). Measured before the release, three Tenders move: two
whose value was a framework maximum typed 100× too large now serve their estimate, and one whose €1.6 bn estimate is
exactly 1000× its own award notice's framework maximum now serves no value (refused is not corrected). `?currency=` matches a Tender whose only amount
in that currency is one of these rows. eForms-DE 1.x notices that state a notice-level framework maximum at the
root of the notice (`DE1-FrameworkMaximumAmount`, about 100 notices) now serve it as the Tender's
`framework_maximum`, as EU notices with the same element already did; their `value` can rise to it.

## Unreleased (issue 508) — text-era award notices of 2009-12 to 2010 serve their winners and values

Award notices published from 2009-12-02 to 2010-12-31, the last thirteen months of the tagged-text era, now serve
the winners they name. Those published from March 2010 on also serve their total value. Before, about 155,000 of these
Tenders served no winner, and about 96,000 of those from March 2010 on served no value, although the notice text
states both. One value is no longer served: where a 2004–2010 notice awards several lots under `LOT NO` headings and
states no total, one lot's figure is no longer served as the Tender's `value`.

## Unreleased (issue 507) — a carried prior-information part no longer outranks the contract notice's figures

When a prior information notice's parts carry into a later contract notice that has lots, a part's figure is no longer
elected as the Tender's `value` while the notice states any other figure the election accepts. It is still served on
the part's own row, and it is still the value when nothing else is accepted.

## Unreleased (issue 505) — a lot figure that is a scaled residual of its procedure total is refused

A lot's figure of €1 bn or more is no longer served, nor elected as the Tender's `value`, when it is exactly 100× to
1,000,000× what the notice's accepted procedure total leaves after the notice's other lots that state a figure in the
same field and currency, and that remainder is at least a hundredth of the total. Where the slip had also kept that
procedure total from being accepted as a framework total (issue 492), the total is now the Tender's `value`. Example: a £60 bn lot beside lots of £150 m and £250 m under a £1 bn procedure total. The figure stays in
`amounts`; the affected Tenders are re-folded and announced as corrections.

## Unreleased (issue 492) — a x100 scale slip is refused unless it is a framework total

A Tender's `value` (and what `min_value` / `max_value` compare) no longer elects a figure of €1 bn or more that is
exactly 100× another figure of the same Tender in the same currency, unless it is a framework total: a
procedure-level figure over two or more lots, whose smaller partner is a lot figure, and whose lots add up to
between a tenth of it and all of it. Each lot's served value follows the same rule, except that a lot figure is
kept only when its smaller partner only ever appears as a figure of other lots of the Tender (never procedure-level,
never this lot's own, never a part's or a lots group's) and the notice states an accepted procedure total at least as
large. A part's own figure is never kept this way.
×1,000 and up was already refused (issue 471). The refused figure stays in `amounts`. Affected Tenders are
re-folded and announced on the change feed as corrections.

## Unreleased (issue 495 unit 5) — a rate correction is announced

A corrected exchange rate re-derives stored EUR values in place (`eur_cents` on amounts, lot results, bids and
contracts). That used to be silent on the change feed. Now each Tender whose EUR values moved gets one
`tender` `changed` with `version: null`. Each lot whose own values moved also gets a `lot` `changed`, and when
the Tender's current version moved, so does every lot of that version. The values elected from them (the
Tender's and the lots' headline EUR value) follow at the next fold, which announces them the same way.

## Unreleased (issue 495 unit 4) — a re-derivation announces corrections, not a history replay

When a fold re-derives a Tender, for example after a fix to how notices are read, the change feed (`/v1/changes`,
SSE, webhooks) used to re-emit every version of that Tender under new cursors. A large refold wrote tens of
millions of change rows that changed nothing.

The fold now compares the re-derived Tender with what is stored and writes only what differs. A Tender whose
served reading moved gets two kinds of row, both with `version: null`:
- one `tender` `changed`;
- one `lot` `changed` for each lot whose reading may have moved. When the Tender's current version moved, that is
  every lot of the new version; when versions were inserted or removed mid-chain, every lot of the Tender.
A Tender that did not move gets no row at all.

Treat a `version: null` `changed` the way the docs already say: re-read the entity by id and upsert it. A client
that ignored such rows also missed every correction from now on, and no longer receives the replay that used to
carry them. Notices arriving as usual are announced exactly as before.

## Unreleased (issue 457 unit 3) — `/v1/sql` refuses recursion and EXCLUDE frames

`/v1/sql` now answers `400` to three constructs, naming the reason:

- `WITH RECURSIVE`;
- any CTE whose body reads its own name or the name of a CTE declared after it;
- an `EXCLUDE` window frame.

The engine already refused `WITH RECURSIVE` and `EXCLUDE`. The change a client can see is the
self-reference without the keyword: `WITH tenders AS (SELECT … FROM tenders) …` used to read the
base table and now answers `400`. Name the CTE something else. The next turso release executes such
a CTE recursively, so its meaning was about to change silently. These constructs are refused
because their work grows with the data's fan-out, which the time limit bounds but memory does not.

## Unreleased (issue 484 unit 3) — `?winner=` no longer counts a buyer as its own winner

`?winner=<org>` on `/v1/tenders`, `/v1/lots`, the SSE subscriptions and webhook
filters means "tenders this organization won" — and now leaves out the awards
where the winner **is the tender's own buyer**: the award notice names one
organization as buyer and as contractor (the same party, or the same name up to
case and accents). ~2,650 notices in the corpus have that shape (census 1981 at
stride 10); in the 63 read by hand it was a publisher repeating its own block in
the contractor slot (57) or a genuine in-house award to the authority's own
service (4). Neither is a supplier win to count.

**The award itself is unchanged and still served**, as published:
`lot_results[].winners[]` keeps the winner and adds `"is_buyer": true`, and the
detail's `parties[]` `winner` / `Tenderer` entry carries the same key (absent
everywhere else — additive). A tender the organization ALSO won on another,
unflagged award still matches. On `/v1/sql`, `v_lot_results` and `v_awards` gain
`winner_is_buyer` (1 or NULL): supplier statistics filter `WHERE winner_is_buyer IS
NULL`, as the filter does.

The flag is written when a tender is folded: a tender not re-folded since the
change reads unflagged (served and counted exactly as before) until the next
re-fold reaches it.

## 2026-09-11 (last) — the derived EUR column never reports zero

Completing the two entries below: the column also declines a conversion that
**rounds** to zero. Six Tenders reached €0.00 that way — published CZK 0.10,
CZK 0.12, HUF 0.79, HUF 1.48 and LIT 2.89, all real figures smaller than half a
euro cent, which the documented half-away-from-zero rounding puts on 0.

This is not a placeholder rule and the published figures are fine. It is about
the column's own vocabulary: 0 already means "no value was elected", so a derived
0 would have the column asserting €0.00 for a HUF 1.48 procurement. Declining
says "no value this column can express", which is the true statement.

So `min_value`/`max_value` never match on a zero, `?max_value=0` returns nothing,
and a zero in the derived column means one thing rather than three. Published
amounts are unchanged, as in both entries below.

## 2026-09-11 (later) — the value filters also skip a published 0.01 or 1.00

A fifth class joins the four below: **exactly one minor unit or one major unit**.
112,244 Tenders served one as their headline value, which makes this the largest
placeholder class in the corpus — larger than the zeros and the negatives put
together.

The evidence is a distribution with two spikes and nothing after them: 59,030
Tenders at €0.01, 53,214 at €1.00, and the third-placed value 36× smaller. Ten
currencies each spike at exactly one major unit against their own two (EUR 125×,
CZK 730×, DKK 412×). Most of it sits on `result_value`, on ordinary award notices
— subtypes 29/16/33/30, no concession marker — for a gymnasium renovation, site
security, painting works, school meals. You cannot award a contract for a cent.

**The rule is two exact values, not a floor.** `0.10` (1,463 Tenders) and `2.00`
(658) are a tail rather than a convention and are still elected.

**Published amounts are unchanged**, as ever: `amounts` carries every figure as it
arrived. What changes is that a Tender whose only figure is a one-unit token now
has **no known value** and is returned by neither value bound.

## 2026-09-11 — the value filters skip placeholder amounts, zero among them

`min_value`/`max_value` compare a derived EUR column, and that column now
declines to elect an amount that is a placeholder rather than a figure. Four
classes, each measured on the corpus rather than assumed (issue 366):

- **Negative** (~15,650 Tenders, 15,529 of them exactly −1.00, the eForms SDK's
  marker for a withheld figure).
- **Exactly zero** (24,647 Tenders). A 0 is an absence, not a price: 11,793 of
  the zero rows sit on `result_value` and 1,408 on `framework_maximum`, and you
  cannot award a contract for nothing or cap a framework at nothing.
- **A run of nine or more identical digits**, with or without the decimal point
  (€999,999,999.99 on street cleaning in a town of 47,000; PLN 22,222,222,222) —
  a form-width maximum typed by holding a key down.
- **Above €100 bn** EUR-equivalent.

Two observable consequences. A Tender whose only published amount falls in one
of these classes has **no known value** and is returned by NEITHER bound —
notably `?max_value=0`, which used to hand back thousands of contracts that are
not free. And the `value` in a payload can be a figure the value filters ignore.

**Published amounts are unchanged.** The `amounts` array still carries every
figure exactly as it arrived, negatives and zeros included; this entry is about
the derived column the filters compare, which layers beside the published
values rather than over them.

## 2026-08-27 — `min_value`/`max_value` compare derived EUR, not raw cents

The value bounds on `/v1/tenders` and `/v1/lots` now compare against the
tender's highest amount **converted to EUR at its publication date**
(ADR-0014), instead of comparing published cents numerically across mixed
currencies. Pass the bounds in EUR cents. Rows with no convertible amount no
longer match a value bound (previously a 20,000,000-SEK tender matched
`min_value=10000000` as if it were EUR). The old comparison was documented as
a caveat, not a contract; this entry retires it. Published values themselves
are unchanged and still served exactly as published — the conversion layers
beside them, never over them.

Also new on the same surface: the `currency` filter (published ISO-4217 code),
the `lang` selector (preferred language for picked titles), and
`eur_cents`/`awarded_eur_cents` columns on the `/v1/sql` tables and the
`v_tender_amounts` view.

## 2026-08-27 — /v1/sql surface promise stated; `currency_rates` queryable

ADR-0015 states what the SQL endpoint promises: allow-listed names, existing
columns and the response envelope are contract (renames/removals only with an
entry here); the dialect is described, not promised, and a canary test suite
pins representative query shapes against every build. `currency_rates` — the
full EUR-pivot rate series behind `eur_cents` (ECB daily 1999→, daily ECU
1993–1998 via Eurostat CC BY 4.0, irrevocable conversions) — joins the
allow-list as reference data.
