# Changelog — the public /v1 API

Behavior changes a client could observe, newest first. Additive fields and new
endpoints land without an entry unless they change how an existing request
answers; this file exists for the rare case where one does.

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
