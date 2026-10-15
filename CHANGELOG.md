# Changelog — the public /v1 API

Behavior changes a client could observe, newest first. Additive fields and new
endpoints land without an entry unless they change how an existing request
answers; this file exists for the rare case where one does.

## Unreleased (issue 532) — a place of performance published only as a country is served in `country` and found by `?country=`

A notice may give its place of performance as a country alone, with no NUTS region: eForms' BT-5141 (often with
`anyw-cou`, "anywhere in the given country") and DÖE's realized-location country. Nothing read it, so such a tender
served `country: []` and no `?country=` value found it. It is now served as that country's NUTS level-0 code: `LT`
for Lithuania, `EL` for Greece, `UK` for the United Kingdom.
- `?country=LT` finds such a tender and its list row echoes `["LT"]`; `?country=LT0` does not find it.
- Only on a version that carries no NUTS place code. A version that already serves a NUTS place serves the same
  `country` as before, even when one of its lots names only a country.
- Only for the NUTS countries: the EU-27, the United Kingdom, Iceland, Liechtenstein, Norway, Switzerland, Albania,
  Montenegro, North Macedonia, Serbia and Türkiye. A place in any other country (Egypt, the USA), or given as an
  overseas region's own ISO code (Réunion, Åland), carries none. A later notice that restates a place that way clears
  the level-0 code that the same procedure or lot derived from an earlier notice.
- In `classifications[]` and on `/v1/sql` the row is scheme `nuts`, field `place_country`. Rows of field `place` are
  unchanged and still hold the NUTS codes as published.

Find a Tender notices are not affected. The stored tenders were refolded on 2026-10-15 (jobs 2521 and 2522,
3,288,104 notices; 197,053 tenders corrected).

## Unreleased (issue 524, unit 2b) — an organization's `name` is elected from its mentions, not its first-seen spelling

`name` on `/v1/organizations` (and `parties[].organization_name`, `lot_results[].winners[].organization_name`,
`bids[].parties[].organization_name`; on `/v1/sql` `organizations.name`, `v_organizations.name`,
`v_lot_results.winner_name`, `v_tender_buyers.buyer_name` and `v_awards.winner_name`/`buyer_name`) was the first
spelling any notice published for the organization, kept forever. Merges and refreshed mentions left heads that few or none of
an organization's notices say: the University of Sheffield served as `Sheffield Hallam University` over 16
Sheffield mentions, a German review chamber as `s.o.`.

For an organization WITH an identifier, the head is now elected from the names its mentions published (ADR-0013's
2026-10-15 amendment): the operator's `refresh-org-names` job serves a fit spelling of the winning cluster, its
whitespace collapsed (trimmed, every run one space). The votes are the mentions that published a name (one with no
name, a blank one or a non-name one does not vote), clustered as one name across casing, diacritics, `the` and a
legal form. The head changes only when a cluster other than its own holds at least 55% of the votes, at least 3
votes and at least twice the head's; or when the head's own cluster holds at most 1 vote and a unique top cluster
holds at least 3 votes and at least 25%; or when the head is empty or not a name (a placeholder, a blank, a
void-lot phrase) and some mention's name votes, whatever script that name is written in. Even then it stays when
the change would swap one legal-form family for another (`AG` to `Sàrl`), drop a consortium mark, fail the NHS ODS
name check the old head passed, or, on the 25% arm only, serve an initialism of the head; when no spelling of the
winning cluster is fit to serve; when the identifier carries a `wrong` or unapplied `related` review verdict; on
the 55% arm, while the organization's newest procedures still use the current head (a rename: the head names at
least 2 procedures and two thirds of the organization's newest as many); on the 55% and 25% arms, while the
current head is the only one of the organization's names printed on its newest publication day; both unless the
head is unfit to serve for a reason other than being all lower case, or carries a standalone number the other name
lacks; and, on the 55% and 25% arms only, when the head is in another script than its mentions (TED's English
rendering).

- A rename arrives as `organization changed` on `/v1/changes`, the SSE streams and webhooks. No `tender
  changed` is published: the parties join the organization when read, so every tender it is on shows the new
  name at once.
- `name_norm` follows the head, so `?name_prefix=` finds an organization by its elected name; its first-seen
  spelling is matched no longer.
- An organization WITHOUT an identifier keeps its first-seen name: there the name is its matching key. So does
  every head the job has not renamed, a head with a single supporting mention included when the bar or a hold
  above keeps it.
- What one notice published is unchanged, on `parties[].mention_name` and `organization_mentions.name`.

Also: the `next_cursor` of a `?name_prefix=` search is now the last row's stored search key (`name_norm`). It
was rebuilt from the lower-cased name, which differs from the key where the key is `org_name_norm` of the name
(every key since issue 432, and every renamed organization's) and the name has a run of whitespace (a doubled
space, a tab) or a trailing `.`, `,` or `;`. An organization keyed before issue 432 while it had an identifier, and
never renamed since (a reverted rename restores that key), keeps its plain lower-cased name as its key, so its cursor
never diverged.
- Where the rebuilt cursor sorted before the key (a doubled space), the next page repeated rows.
- Where it sorted after the key (a trailing `.`, `,` or `;`: `ACME Ltd.` gave the cursor `acme ltd.` for the key
  `acme ltd`), the next page silently SKIPPED every row keyed between the two: any `acme ltd` with a higher id,
  and names continuing `acme ltd `, `acme ltd (` or `acme ltd -`.

A client that paged a `name_prefix` search to its end may have missed organizations and should page it again. A
cursor issued before is still accepted, but one issued on such a name repeats or skips once more.

## Unreleased (issue 529) — a withheld sole trader named `Individual` is no longer a party without an identifier

Find a Tender publishes a withheld sole-trader supplier under the name `Individual`. A bare one, or one carrying the
s.94 placeholder PPONs (issue 522), bound by name, so every withheld UK sole trader pooled into one organization named
`Individual`. Such a party is now dropped, like the other withheld-supplier stand-ins. It is not served in
`parties[]`, `bids[].parties` or `lot_results[].winners`, and the pooled organization is removed (`organization
removed`). On a Find a Tender result the publisher's decision (`selec-w`) and its awarded value stay. An `Individual`
with an identifier the gate admits is still a party, whoever the identifier belongs to. The rule reads the whole name,
not the source: a bare `Individual` (or `Individual (Lot 2)`) from any publisher drops the same way. A name that merely
contains the word (`Individual Centred Care`) is unchanged.

## Unreleased (issue 534) — legacy TED parties serve canonical roles for award winners, mediation and appeal bodies

Legacy TED notices (R2.0.7–R2.0.9) serve a party's role from the element that names it. Several elements were served
verbatim:
- `AWARD_AND_CONTRACT_VALUE` now serves `winner` (the R2.0.7/R2.0.8 utilities award F06, and the 2008 internal-ojs
  `AWARD_AND_CONTRACT_VALUE_SUM`);
- `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` and `BODY_RESPONSIBLE_MEDIATION` serve `mediation-body`;
- `BODY_RESPONSIBLE_APPEAL` serves `review-body`;
- `SERVICE_INFORMATION_LODGING_APPEALS` and R2.0.9's `ADDRESS_REVIEW_INFO` serve `appeal-information` (it was served as
  `review-body`).

Two consequences show in `parties[]`:
- parties carry forward role by role, so a later notice of the chain that publishes a `winner` now replaces the F06
  award winners on `parties[]` (the award decisions accumulate as before: `lot_results` and `?winner=` keep them);
- a folded award winner that is its notice's own buyer now carries `is_buyer: true` (issue 484).

The tenders projected before the change are refolded (about 2.0M notices, 1.1M tenders); a refolded tender whose served
reading moved publishes `tender changed`. The refold ran on 2026-10-15 (jobs 2512 and 2513).

## Unreleased (issue 536) — `publication_id=` matches TED's own spelling, the OJ S form and every stored width

`/v1/notices?publication_id=` and `/v1/tenders?publication_id=` were an exact string match on the stored form, so
TED's own spelling (`83685-2026`), the OJ S form (`2017/S 044-079997`) and the `/docs` example found nothing, and a
6-digit number returned whichever source stored those digits. A TED lookup now tries every stored width of the same
number (at most four spellings: the value sent, unpadded, 6 and 8 digits). A spelling the API adds, rather than the one
sent, matches TED rows only: `83685-2026`, `00083685-2026` and `2026/S 012-083685` find TED's `00083685-2026` and not
Find a Tender's `083685-2026`. The value as sent still matches a row of any source, so a 6-digit number both TED and
Find a Tender hold (`083685-2026`) returns both notices; pair it with `source=` to get one. The OJ S form with issue
`000` (`2026/S 000-083685`) reaches Find a Tender's 6-digit spelling only. When the page is not empty and matched at a
stored spelling other than the one sent, `resolved_filters.publication_id` reports `{"asked": <the raw value>,
"matched": [<stored spellings>]}`. A `publication_id` page that narrows its number's rows after the read can now be
short, or empty, while `more` is true: its `next_cursor` is the last row examined, so follow it to the end.

## Unreleased (issue 522) — the UK's withheld-supplier PPONs no longer name an organization

Find a Tender publishes placeholder PPONs on the suppliers a notice withholds under s.94 of the Procurement Act 2023:
`GB-PPON-PBXY-3232-WNHZ` ("Withheld Section94 Supplier") and its second profile `GB-PPON-PGYY-2165-QXNQ`. They were
read as identifiers, so every party carrying one was filed under one organization, with an insurer and a German
laboratory among them. These PPONs now key nothing. Each party binds by its published name, and the placeholder organizations are removed
(`organization removed`). The affected Tenders publish `tender changed`. Every other PPON is one supplier's own and
keys it as before.

## Unreleased (issue 526, unit 3) — lettered lot labels, `à <Company>` lists and address columns in 1993–2010 winner lists

The re-parse of 1993–2010 winner lists now reads several shapes the first one left as published.

- **Lettered and Roman labels** split the list as numbers do:
  - `Lot A) Somaco. Lot B) Geneton`, `Lot III: Dompe Biotec SpA`;
  - `lots B1) et B2) Buhrmann France Image`, `lot M00.14) Commune de Spincourt`;
  - `Lot BEY 01) Dumas TP`, `lot 6/1) Imprimerie Dessaint`;
  - the Italian `Lotto 1: Team Service`.
  On a sample of 217 such notices, names that keep a label fall from 453 to 0. A word that only starts like a
  label stays whole (`Lot Atlas`, `LOT Polish Airlines`, `Lot IVECO`).
- **`à` and `pour` award the lots** in the 2007–2010 hospital lists (`Lots 1 et 4 à Ansell`, `Lot 22 pour
  Sophysa`), and each lot keeps its own figure. `1 à 4` is still a range.
- **A company that opens on its digit** is a winner (`3M France`, `3M Santé`, `2SDA`, `Trois M Santé`). The first
  re-parse had refused these. A figure, a quantity or a description is not (`12 véhicules`, `3D Scanners`).
- **Entries joined by a comma, dash or slash** before the next lot's reference name both winners (`Seguridad
  Gallega Nosa Tera, SA, Lote II: Centro de Seguridad Marítima Integral "Jovellanos"`). A lot whose sub-references
  share one winner names it once (`lot 1) Tr. 5 et 6, lot 2) Tr. 1 et 2: Planète Aventures`).
- **The addresses some 2004–2010 values repeat lot by lot** after the winners are no organizations (`Lot 1:
  Frantour, … Lot 1: rue des Clercs, …`), nor are a column header (`Lot`) or a street (`Rua do Cinzeiro`, `Avda.
  Cortes Valencianas`).
- **A label that only describes a consortium** gives way to its name (`groupement d'entreprises: Falconnet Denis`
  → `Falconnet Denis`). A head cut inside a département list reads past it (`Île-de-France (75, 78, 92, 95)
  Lamos SA` → `Lamos SA`).

The organizations that carried a reference are removed (`organization removed`), and the affected Tenders publish
`tender changed`. A block that now names two winners keeps its total unattributed, as a joint award does.

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
- a keyed award sentence names its awardee (`lot 3) la société Storz a été retenue` → `Storz`);
- an entry that names its lot's subject before the company names the company (`Lot 1) VRD: Colas IDF Normandie`,
  `Lot 1) Terrassement - VRD, Entreprise Malet` → `Colas IDF Normandie`, `Entreprise Malet`), and a company's own
  lot list (`Société Laser: lot 1) Alpes Maritime; lot 3) …`) names the company once.

On a 912-notice sample, names carrying a lot reference fell from 4,423 to 208. Lettered and Roman lot labels
(`Lot A)`, `Lot III:`) were kept as published until unit 3 (above). The prefixed organizations are removed
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
