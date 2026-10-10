# Changelog — the public /v1 API

Behavior changes a client could observe, newest first. Additive fields and new
endpoints land without an entry unless they change how an existing request
answers; this file exists for the rare case where one does.

## Unreleased (issue 506) — an award notice's framework values per lot are kept, and refuse scale slips

eForms award notices state a framework maximum and a re-estimated value per lot result (BT-709, BT-660). They are
now served in `amounts` at their lot under two new `field` values, `result_framework_maximum` and
`result_framework_reestimate`; they are never elected as a Tender's or a lot's `value`. A figure of €1 bn or more
that is exactly 100× (or 10ᵏ×) one of them is now refused as a scale slip, so two Tenders whose value was a
framework maximum typed 100× too large now serve their estimate. `?currency=` matches a Tender whose only amount
in that currency is one of these rows.

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
