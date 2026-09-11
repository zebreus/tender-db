# Changelog — the public /v1 API

Behavior changes a client could observe, newest first. Additive fields and new
endpoints land without an entry unless they change how an existing request
answers; this file exists for the rare case where one does.

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
