"""Ports of the canonical.rs unit tests that pin head_value_eur_cents(_with) (and the predicates it
calls) onto elect.py.  Each fixture is rebuilt as STORED ROWS (tender_version_amounts /
tender_version_lots / tender_version_lot_results + lot_results) and run through the public
elect(rows, lots, results, chain, lot_keys) entry point, so the row adapter is exercised too.

Rust: crates/store/src/canonical.rs @ 8c85af5, test module (lines ~43080-44860).
Run:  python3 -I test_elect.py
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import elect as E  # noqa: E402

RESULTS = []          # (test, label, ok, got, want)


def check(test, label, got, want):
    RESULTS.append((test, label, got == want, got, want))


# ------------------------------------------------------------------ fixture builders
AUTO = object()


def amt(field, cents, currency="EUR", quality=None, eur=AUTO, tax_basis=None):
    # RatesLookup::default(): EUR converts at 1.0, everything else has no rate.
    if eur is AUTO:
        eur = cents if currency == "EUR" else None
    return {"field": field, "cents": cents, "currency": currency, "quality": quality, "eur": eur, "tax_basis": tax_basis}


def lot(key, facts, kind="Lot"):
    return {"key": key, "kind": kind, "facts": list(facts)}


def version(facts, lots=None, rounds=None):
    return {"facts": list(facts), "lots": list(lots or []), "rounds": list(rounds or [])}


def head(facts, lot_facts=()):
    """The Rust `head(facts, lot_facts)`: one Lot LOT-0001 carrying lot_facts."""
    return version(facts, [lot("LOT-0001", lot_facts)])


def award(lot_key, cents, currency="EUR", key=None):
    return {"key": key or f"RES-{lot_key}", "lot_key": lot_key, "cents": cents, "currency": currency}


def with_rounds(v, rounds):
    v = dict(v)
    v["rounds"] = [(n, list(rs)) for n, rs in rounds]
    return v


def with_awards(v, awards):
    return with_rounds(v, [(1, awards)])


class Reg:
    """Per-chain identity: lot keys -> lots.id, (notice, result key) -> lot_results.id."""

    def __init__(self):
        self.lots, self.results = {}, {}

    def lot_id(self, key):
        return self.lots.setdefault(key, 100 + len(self.lots))

    def result_id(self, notice, key):
        return self.results.setdefault((notice, key), 500 + len(self.results))


def to_rows(v, reg, seq):
    amounts, lots, results = [], [], []

    def row(f, lot_id):
        return (1, seq, lot_id, f["field"], f["cents"], f["currency"], f["tax_basis"], f["eur"], f["quality"])

    for f in v["facts"]:
        amounts.append(row(f, None))
    for l in v["lots"]:
        lid = reg.lot_id(l["key"])
        lots.append({"lot_id": lid, "kind": l["kind"]})
        for f in l["facts"]:
            amounts.append(row(f, lid))
    for notice, rs in v["rounds"]:
        for r in rs:
            results.append({
                "lot_result_id": reg.result_id(notice, r["key"]),
                "lot_id": None if r["lot_key"] is None else reg.lot_id(r["lot_key"]),
                "awarded_cents": r["cents"], "awarded_currency": r["currency"],
                "notice_id": notice, "result_key": r["key"],
            })
    return {"amounts": amounts, "lots": lots, "results": results}


def run(chain):
    reg = Reg()
    rows = [to_rows(v, reg, i + 1) for i, v in enumerate(chain)]
    lot_keys = {i: k for k, i in reg.lots.items()}
    h = rows[-1]
    return E.elect(h["amounts"], h["lots"], h["results"], chain=rows[:-1], lot_keys=lot_keys)


def one(v):
    return run([v])[0]


def elect_chain(chain):
    return run(chain)[0]


def versions(chain):
    reg = Reg()
    rows = [to_rows(v, reg, i + 1) for i, v in enumerate(chain)]
    lot_keys = {i: k for k, i in reg.lots.items()}
    return [E.version_from_rows(r["amounts"], r["lots"], r["results"], lot_keys) for r in rows]


def rule_of(chain):
    return E.ScalePartners.of_chain(versions(chain))


def lot_value(chain, key, part_aware=True):
    """The stored lot value of `key` in the last version (the tests feed eur_cents = cents)."""
    vs = versions(chain)
    rule = E.ScalePartners.of_chain(vs)
    l = next(l for l in vs[-1].lots if l.key == key)
    facts = [E.Amount(f.field, f.cents, f.currency, f.tax_basis, f.quality, f.cents) for f in l.facts]
    w = E.elect_lot_value(facts, rule, key if (not part_aware or l.kind != "Part") else None)
    return None if w is None else w.cents


P, L, H = E.PROCEDURE, E.LOT, E.HEADLOT

# ------------------------------------------------------------------ the_head_value_skips_sentinels_and_the_ceiling
t = "the_head_value_skips_sentinels_and_the_ceiling"
ev = lambda c: amt("estimated_value", c)  # noqa: E731
val = lambda facts: one(head(facts))       # noqa: E731
check(t, "-1.00 beside a figure", val([ev(-100), ev(500_000)]), 500_000)
check(t, "-1.00 alone", val([ev(-100)]), None)
check(t, "999,999,999.00", val([ev(99_999_999_900), ev(250_000)]), 250_000)
check(t, "9,999,999,999.00", val([ev(9_999_999_999_00), ev(250_000)]), 250_000)
check(t, "EUR 1 bn round kept", val([ev(100_000_000_000)]), 100_000_000_000)
check(t, "zero", val([ev(0)]), None)
check(t, "ceiling", val([ev(4_970_000_000_000_000_000), ev(1_000_000)]), 1_000_000)
check(t, "band kept", val([ev(1_060_000_000_000)]), 1_060_000_000_000)

# ------------------------------------------------------------------ sentinel_amounts_are_negatives_and_all_nines_runs
t = "sentinel_amounts_are_negatives_and_all_nines_runs"
for c, want in [(-100, True), (-1, True), (99_999_999_900, True), (999_999_999_900, True), (9_999_999_999_900, True),
                (2_222_222_222_200, True), (111_111_111_00, True), (444_444_444_400, True), (1_111_111_100, False),
                (9_999_999_900, False), (100_000_000_000, False), (123_456_700, False), (0, True),
                (99_999_999_999, True), (999_999_999, True), (9_999_999_999, True), (999_999_999_999, True),
                (9_999_999_999_999, True), (333_333_333, True), (111_111_111, True), (222_222_222, True),
                (8_888_888_888, True), (11_111_111, False), (99_999_999, False)]:
    check(t, f"sentinel({c})", E.sentinel_amount(c), want)

# ------------------------------------------------------------------ the_head_column_does_not_assert_a_price_of_zero
t = "the_head_column_does_not_assert_a_price_of_zero"
rv = lambda c: amt("result_value", c)  # noqa: E731
check(t, "0 alone", val([rv(0)]), None)
check(t, "0 beside", val([rv(0), rv(450_000_00)]), 450_000_00)
check(t, "0 and -1", val([rv(0), rv(-100)]), None)
check(t, "0.01", val([rv(1)]), None)
check(t, "0.10", val([rv(10)]), 10)
check(t, "2.00", val([rv(200)]), 200)

# ------------------------------------------------------------------ a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected
t = "a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected"
check(t, "6941544", one(version([amt("result_value", 8_000_000_000_000), amt("estimated_value", 8_000_000_000)])), 8_000_000_000)
genuine = version([amt("estimated_value", 1_080_000_000_000), amt("result_value", 1_080_000_000_000)],
                  [lot("LOT-0001", [amt("result_value", 1_080_000_000)])])
check(t, "8400892 corroborated", one(with_awards(genuine, [award("LOT-0001", 1_080_000_000)])), 1_080_000_000_000)
vending = version([amt("result_value", 1_200_000_000_000)],
                  [lot(f"LOT-{i}", [amt("estimated_value", 1_200_000_000)]) for i in range(1, 7)])
check(t, "5592948 award does not corroborate",
      one(with_awards(vending, [award(f"LOT-{i}", 1_200_000_000_000) for i in range(1, 7)])), 1_200_000_000)
check(t, "x100 single lot", one(version([amt("framework_maximum", 2_000_000_000_000)],
                                        [lot("LOT-0001", [amt("estimated_value", 20_000_000_000)])])), 20_000_000_000)
check(t, "x100 framework total", one(version([amt("framework_maximum", 2_000_000_000_000)],
                                             [lot("LOT-0001", [amt("estimated_value", 20_000_000_000)]),
                                              lot("LOT-0002", [amt("estimated_value", 1_980_000_000_000)])])), 2_000_000_000_000)
check(t, "round mantissa refused", one(version([amt("framework_maximum", 2_000_000_000_000)],
                                               [lot("LOT-0001", [amt("estimated_value", 2_000_000_000)])])), 2_000_000_000)
check(t, "round mantissa corroborated", one(version([amt("framework_maximum", 2_000_000_000_000), amt("estimated_value", 2_000_000_000_000)],
                                                    [lot("LOT-0001", [amt("estimated_value", 2_000_000_000)])])), 2_000_000_000_000)
check(t, "577127", one(version([], [lot("LOT-0001", [amt("framework_maximum", 1_500_000_000_000), amt("estimated_value", 1_500_000_000)])])), 1_500_000_000)
check(t, "224156", one(version([], [lot("LOT-0001", [amt("framework_maximum", 3_000_000_000)]),
                                    lot("LOT-0002", [amt("framework_maximum", 3_000_000_000_000)])])), 3_000_000_000)
earlier = version([amt("estimated_value", 1_234_567_891)])
later = version([amt("estimated_value", 1_234_567_891_000)])
check(t, "6721266 alone", one(later), 1_234_567_891_000)
check(t, "6721266 chain", elect_chain([earlier, later]), None)
placeholders = version([amt("result_value", 10_000_000_000_000), amt("estimated_value", 1_000_000_000_000)],
                       [lot("LOT-0001", [amt("estimated_value", 100_000_000_000)])])
check(t, "6988280", one(with_awards(placeholders, [award("LOT-0001", 1_000_000_000)])), None)
for floor in (100, 1_000):
    check(t, f"{floor} no partner", one(version([amt("framework_maximum", floor * 10_000_000_000), amt("estimated_value", floor)])), floor * 10_000_000_000)
check(t, "10.01 is a partner", one(version([amt("framework_maximum", 1_001_000_000_000), amt("estimated_value", 1_001)])), 1_001)
check(t, "GBP no partner", one(version([amt("result_value", 5_000_000_000_000), amt("estimated_value", 5_000_000_000, "GBP")])), 5_000_000_000_000)
check(t, "near miss", one(version([amt("result_value", 5_000_000_000_001), amt("estimated_value", 5_000_000_000)])), 5_000_000_000_001)
check(t, "below gate 5m/5000", one(version([amt("estimated_value", 500_000_000)], [lot("LOT-0001", [amt("result_value", 500_000)])])), 500_000_000)
check(t, "below gate 1m/1000 award", one(with_awards(version([amt("framework_maximum", 100_000_000)], [lot("LOT-0001", [])]),
                                                     [award("LOT-0001", 100_000)])), 100_000_000)
check(t, "exactly 1bn reached", one(version([amt("result_value", E.SCALE_ERROR_MIN_EUR_CENTS), amt("estimated_value", 100_000_000)])), 100_000_000)
check(t, "999,999,000 below", one(version([amt("result_value", 99_999_900_000), amt("estimated_value", 99_999_900)])), 99_999_900_000)
check(t, "6681865", one(version([amt("result_value", 970_000_000_000), amt("estimated_value", 970_000_000)])), 970_000_000)
rule = E.ScalePartners()
rule.add_partner("EUR", 1_000_000_000)
rule.add_head_amount("result_value", "EUR", 1_000_000_000_000)
check(t, "hand-fed refuses", rule.refuses_amount("result_value", "EUR", 1_000_000_000_000, 1_000_000_000_000, P), True)
check(t, "hand-fed at gate", rule.refuses_amount("result_value", "EUR", 1_000_000_000_000, E.SCALE_ERROR_MIN_EUR_CENTS, P), True)
check(t, "hand-fed below gate", rule.refuses_amount("result_value", "EUR", 1_000_000_000_000, E.SCALE_ERROR_MIN_EUR_CENTS - 1, P), False)
rule.add_head_amount("result_value", "EUR", 1_000_000_000_000)
check(t, "same field twice", rule.refuses_amount("result_value", "EUR", 1_000_000_000_000, 1_000_000_000_000, P), True)
rule.add_head_amount("estimated_value", "EUR", 1_000_000_000_000)
check(t, "corroborated", rule.refuses_amount("result_value", "EUR", 1_000_000_000_000, 1_000_000_000_000, P), False)

# ------------------------------------------------------------------ a_text_award_figure_partners_the_notice_total_it_slips_from
t = "a_text_award_figure_partners_the_notice_total_it_slips_from"


def text_head(facts, awards):
    return version(facts, [], [(1, [{"key": f"RES-{i + 1}", "lot_key": None, "cents": c, "currency": "EUR"}
                                    for i, c in enumerate(awards)])])


check(t, "35 bn refused", one(text_head([amt("result_value", 3_507_620_000_000)], [3_507_620_000])), None)
check(t, "corroborated kept", one(text_head([amt("result_value", 3_507_620_000_000), amt("estimated_value", 3_507_620_000_000)], [3_507_620_000])), 3_507_620_000_000)
check(t, "award restates total", one(text_head([amt("result_value", 79_180_500)], [79_180_500])), 79_180_500)
check(t, "awards elect nothing", one(text_head([], [400_875_960, 156_050_000])), None)
check(t, "framework over contracts", one(text_head([amt("result_value", 120_000_000_000)], [1_200_000_000, 118_800_000_000])), 120_000_000_000)
check(t, "contracts far below", one(text_head([amt("result_value", 120_000_000_000)], [1_200_000_000, 100_000_000])), None)
check(t, "one contract", one(text_head([amt("result_value", 120_000_000_000)], [1_200_000_000])), None)

# ------------------------------------------------------------------ a_x100_figure_is_refused_unless_it_is_a_framework_total_over_its_lots
t = "a_x100_figure_is_refused_unless_it_is_a_framework_total_over_its_lots"
f, p = 120_000_000_000, 1_200_000_000
framework = version([amt("estimated_value", f)],
                    [lot("LOT-1", [amt("estimated_value", 60_000_000_000)]), lot("LOT-2", [amt("estimated_value", 58_800_000_000)]),
                     lot("LOT-3", [amt("estimated_value", p)])])
check(t, "framework kept", one(framework), f)
aw = with_awards(version([amt("result_value", f), amt("result_value", p), amt("result_value", 18_800_000_000)],
                         [lot("LOT-1", []), lot("LOT-2", [])]),
                 [award("LOT-1", p), award("LOT-2", 18_800_000_000)])
check(t, "lot-null awards are lot figures", one(aw), f)
check(t, "474292 single", one(version([amt("estimated_value", f)], [lot("LOT-1", [amt("estimated_value", p)])])), p)
check(t, "8413585 thin", one(version([amt("estimated_value", f)], [lot("LOT-1", [amt("estimated_value", p)]), lot("LOT-2", [amt("estimated_value", 1_500_000_000)]),
                                                                   lot("LOT-3", [amt("estimated_value", 1_632_000_000)])])), 1_632_000_000)
check(t, "5864294 procedure partner", elect_chain([version([amt("estimated_value", p)]),
                                                   version([amt("result_value", f)], [lot("LOT-1", [amt("result_value", 60_000_000_000)]),
                                                                                       lot("LOT-2", [amt("result_value", 59_000_000_000)])])]), 60_000_000_000)
check(t, "lots over F", one(version([amt("estimated_value", f)], [lot("LOT-1", [amt("estimated_value", p)]), lot("LOT-2", [amt("estimated_value", 119_000_000_000)])])), 119_000_000_000)
rule = rule_of([framework])
check(t, "rule: framework as head asks", rule.refuses_amount("estimated_value", "EUR", f, f, P), False)
check(t, "rule: Lot(None)", rule.refuses_amount("estimated_value", "EUR", f, f, L(None)), True)
check(t, "rule: own lot", rule.refuses_amount("estimated_value", "EUR", f, f, L("LOT-3")), True)
check(t, "rule: sibling lot", rule.refuses_amount("estimated_value", "EUR", f, f, L("LOT-1")), False)
check(t, "two fields do not corroborate x100", one(version([amt("estimated_value", f), amt("result_value", f)], [lot("LOT-1", [amt("estimated_value", p)])])), p)
check(t, "below gate", one(version([amt("estimated_value", 1_000_000_000)], [lot("LOT-1", [amt("estimated_value", 10_000_000)])])), 1_000_000_000)
grouped = dict(framework, lots=framework["lots"] + [lot("GLO-1", [amt("framework_maximum", f)], "LotsGroup")])
check(t, "group not a lot", one(grouped), f)
check(t, "group not counted (head_lots)", rule_of([grouped]).head_lots, 3)
carried = dict(framework, lots=framework["lots"] + [lot("PAR-1", [amt("estimated_value", 110_000_000_000)], "Part")])
check(t, "carried part", one(carried), f)
check(t, "PIN sums parts", one(version([amt("estimated_value", f)], [lot("PAR-1", [amt("estimated_value", 118_800_000_000)], "Part"),
                                                                     lot("PAR-2", [amt("estimated_value", p)], "Part")])), f)
check(t, "award on group", one(with_awards(grouped, [award("GLO-1", 50_000_000_000)])), f)
sent = version(framework["facts"], [lot("LOT-1", [amt("estimated_value", 60_000_000_000), amt("framework_maximum", 9_999_999_999_999)])] + framework["lots"][1:])
check(t, "sentinel not in sum", one(sent), f)
wh = version(framework["facts"], [framework["lots"][0], lot("LOT-2", [amt("estimated_value", 58_800_000_000),
                                                                       amt("framework_maximum", 900_000_000_000, quality="withheld")]), framework["lots"][2]])
check(t, "withheld not in sum", one(wh), f)
copy = with_awards(version([amt("result_value", f)], [lot("LOT-1", [amt("estimated_value", p)]), lot("LOT-2", [])]), [award("LOT-1", f)])
check(t, "award copy is a lot figure", one(copy), p)

# ------------------------------------------------------------------ a_partner_only_amount_refuses_but_is_never_elected
t = "a_partner_only_amount_refuses_but_is_never_elected"
RFM, RFR = E.RESULT_FRAMEWORK_MAXIMUM, E.RESULT_FRAMEWORK_REESTIMATE
alone = version([], [lot("LOT-1", [amt(RFM, 500_000_000)])])
check(t, "alone", (one(alone), lot_value([alone], "LOT-1", False)), (None, None))
beside = version([], [lot("LOT-1", [amt("estimated_value", 100_000_000), amt(RFR, 900_000_000)])])
check(t, "beside", (one(beside), lot_value([beside], "LOT-1", False)), (100_000_000, 100_000_000))
pp, ff = 734_820_000, 299_867_940_000


def slip(result):
    facts = [amt("estimated_value", pp), amt("framework_maximum", ff)] + ([amt(*result)] if result else [])
    return version([amt("estimated_value", pp)], [lot("LOT-1", facts)])


check(t, "395737 without", one(slip(None)), ff)
refused = slip((RFM, ff // 100))
check(t, "395737", (one(refused), lot_value([refused], "LOT-1", False)), (pp, pp))
pp, ff = 666_666_667, 332_277_083_300
reest = version([amt("estimated_value", pp)], [lot("LOT-1", [amt("estimated_value", pp), amt("framework_maximum", ff), amt(RFR, ff // 100)])])
check(t, "627219", (one(reest), lot_value([reest], "LOT-1", False)), (pp, pp))
pp, ff = 1_000_000_000, 1_000_000_000_000
check(t, "partner-only copy vouches nothing", one(version([amt("estimated_value", pp)], [lot("LOT-1", [amt("framework_maximum", ff), amt(RFM, ff)])])), pp)
check(t, "electable corroborates", one(version([amt("estimated_value", pp)], [lot("LOT-1", [amt("framework_maximum", ff), amt("estimated_value", ff)])])), ff)
ff, partner = 100_000_000_000, 1_000_000_000


def fw(extra):
    return version([amt("framework_maximum", ff)], [lot("LOT-1", [amt("estimated_value", partner)]),
                                                     lot("LOT-2", [amt("estimated_value", 50_000_000_000)] + extra)])


check(t, "framework kept", one(fw([])), ff)
check(t, "partner-only not in sum", one(fw([amt(RFM, 99_500_000_000)])), ff)
big, small = 100_000_000_000, 1_000_000_000


def laundry(own):
    return version([amt("estimated_value", 271_000_000_000)],
                   [lot("LOT-1", [amt("estimated_value", big)] + own), lot("LOT-2", [amt("estimated_value", 170_000_000_000)]),
                    lot("LOT-4", [amt("estimated_value", small)])])


check(t, "8748271 lot kept", lot_value([laundry([])], "LOT-1", False), big)
check(t, "own result is no sibling", lot_value([laundry([amt(RFM, small)])], "LOT-1", False), None)


def vouched(proc):
    return version([proc], [lot("LOT-1", [amt("estimated_value", big)]), lot("LOT-2", [amt("estimated_value", 170_000_000_000)]),
                            lot("LOT-4", [amt("estimated_value", small)])])


check(t, "vouched", lot_value([vouched(amt("estimated_value", 271_000_000_000))], "LOT-1", False), big)
check(t, "partner-only procedure no vouching", lot_value([vouched(amt(RFM, 271_000_000_000))], "LOT-1", False), None)
switched = version([amt("framework_maximum", ff)], [lot("LOT-1", [amt("estimated_value", partner), amt(RFM, ff // 1000)]),
                                                     lot("LOT-2", [amt("estimated_value", 50_000_000_000)])])
check(t, "path switch (a)", one(switched), 50_000_000_000)


def corrob(extra):
    return version([amt("framework_maximum", ff), amt("estimated_value", ff)], [lot("LOT-1", [amt("estimated_value", partner)] + extra)])


check(t, "path switch (b) x100", one(corrob([])), partner)
check(t, "path switch (b) k>=3", one(corrob([amt(RFR, ff // 1000)])), ff)

# ------------------------------------------------------------------ a_x100_lot_figure_is_kept_only_beside_a_sibling_lot_partner
t = "a_x100_lot_figure_is_kept_only_beside_a_sibling_lot_partner"
laund = laundry([])
check(t, "8748271", lot_value([laund], "LOT-1"), big)
spv = version([amt("estimated_value", 4_700_000_000_000)], [lot("LOT-1", [amt("estimated_value", small)]), lot("LOT-2", [amt("estimated_value", small)]),
                                                            lot("LOT-3b", [amt("estimated_value", big)])])
check(t, "8811221", lot_value([spv], "LOT-3b"), big)
single = version([amt("estimated_value", small)], [lot("LOT-1", [amt("estimated_value", big)])])
check(t, "single lot value", lot_value([single], "LOT-1"), None)
check(t, "single head", one(single), small)
e1 = version([amt("estimated_value", 200_000_000_000)], [lot("LOT-1", [amt("estimated_value", small)]), lot("LOT-2", [amt("estimated_value", 500_000_000)])])
l1 = version([amt("estimated_value", 200_000_000_000)], [lot("LOT-1", [amt("estimated_value", big)]), lot("LOT-2", [amt("estimated_value", 500_000_000)])])
check(t, "own earlier figure", lot_value([e1, l1], "LOT-1"), None)
split = version([amt("estimated_value", 3_000_000_000)], [lot("LOT-1", [amt("estimated_value", big)]), lot("LOT-2", [amt("estimated_value", small)]),
                                                          lot("LOT-3", [amt("estimated_value", small)])])
check(t, "above procedure", lot_value([split], "LOT-1"), None)
check(t, "unbounded", lot_value([version([], [lot("LOT-1", [amt("estimated_value", big)]), lot("LOT-2", [amt("estimated_value", small)])])], "LOT-1"), None)
pt = dict(laund, facts=laund["facts"] + [amt("result_value", small)])
check(t, "partner at procedure scope", lot_value([pt], "LOT-1"), None)
copied = version([amt("estimated_value", big)], [lot("LOT-1", [amt("estimated_value", big)]), lot("LOT-2", [amt("estimated_value", small)])])
check(t, "refused procedure copy lot", lot_value([copied], "LOT-1"), None)
check(t, "refused procedure copy head", one(copied), small)
junk = version([amt("estimated_value", 15_000_000_000_000)], [lot("LOT-1", [amt("estimated_value", big)]), lot("LOT-2", [amt("estimated_value", small)])])
check(t, "junk bound", lot_value([junk], "LOT-1"), None)
ladder = version([amt("estimated_value", 200_000_000_000)], [lot("LOT-1", [amt("estimated_value", big)]), lot("LOT-2", [amt("estimated_value", small)]),
                                                             lot("LOT-3", [amt("estimated_value", 200_000_000)])])
check(t, "ladder", lot_value([ladder], "LOT-1"), None)
pin = version([], [lot("PAR-0001", [amt("estimated_value", small)], "Part")])
cn = version([amt("estimated_value", 150_000_000_000)], [lot("LOT-0001", [amt("estimated_value", big)]), lot("LOT-0002", [amt("estimated_value", 50_000_000_000)]),
                                                         pin["lots"][0]])
check(t, "part is no sibling", lot_value([pin, cn], "LOT-0001"), None)
pin = version([], [lot("PAR-0001", [amt("estimated_value", big)], "Part")])
cn = version([amt("estimated_value", 150_000_000_000)], [lot("LOT-0001", [amt("estimated_value", small)]), lot("LOT-0002", [amt("estimated_value", 149_000_000_000)]),
                                                         pin["lots"][0]])
check(t, "part's own x100", lot_value([pin, cn], "PAR-0001"), None)
keyless = version([amt("estimated_value", 300_000_000_000)], [lot("LOT-1", [amt("estimated_value", big)]), lot("LOT-2", [amt("estimated_value", small)]),
                                                              lot("LOT-3", [amt("estimated_value", 3_000_000_000)])],
                  [(1, [{"key": "RES-1", "lot_key": None, "cents": 300_000_000_000, "currency": "EUR"}])])
check(t, "keyless head", one(keyless), 3_000_000_000)
check(t, "keyless lot", lot_value([keyless], "LOT-1"), None)
grp = version([amt("estimated_value", 271_000_000_000)], [lot("LOT-1", [amt("estimated_value", 268_290_000_000)]), lot("LOT-2", [amt("estimated_value", 2_710_000_000)]),
                                                          lot("GLO-1", [amt("framework_maximum", 271_000_000_000)], "LotsGroup")])
check(t, "group lot value", lot_value([grp], "GLO-1"), 271_000_000_000)
check(t, "group head", one(grp), 271_000_000_000)

# ------------------------------------------------------------------ a_lot_exactly_ten_to_the_k_times_the_procedure_residual_is_refused
t = "a_lot_exactly_ten_to_the_k_times_the_procedure_residual_is_refused"
essex = version([amt("estimated_value", 100_000_000_000)], [lot("LOT-1", [amt("estimated_value", 6_000_000_000_000)]),
                                                            lot("LOT-2", [amt("estimated_value", 15_000_000_000)]),
                                                            lot("LOT-3", [amt("estimated_value", 25_000_000_000)])])
check(t, "8784848 lot", lot_value([essex], "LOT-1"), None)
check(t, "8784848 head", one(essex), 100_000_000_000)
hzd = version([amt("estimated_value", 2_246_500_000), amt("framework_maximum", 2_695_800_000)],
              [lot("LOT-1", [amt("estimated_value", 302_500_000), amt("framework_maximum", 363_000_000)]),
               lot("LOT-2", [amt("estimated_value", 1_200_000_000), amt("framework_maximum", 1_440_000_000_000)]),
               lot("LOT-3", [amt("estimated_value", 744_000_000), amt("framework_maximum", 892_800_000)])])
check(t, "553044 lot", lot_value([hzd], "LOT-2"), 1_200_000_000)
check(t, "553044 head", one(hzd), 2_695_800_000)
krakow = version([amt("estimated_value", 2_959_463_972)], [lot("LOT-1", [amt("estimated_value", 2_163_135_972)]), lot("LOT-5", [amt("estimated_value", 796_328_000_000)])])
check(t, "915781 lot", lot_value([krakow], "LOT-5"), None)
check(t, "k=0", lot_value([version([amt("estimated_value", 160_000_000_000)], [lot("LOT-1", [amt("estimated_value", 100_000_000_000)]), lot("LOT-2", [amt("estimated_value", 60_000_000_000)])])], "LOT-1"), 100_000_000_000)
check(t, "k=1", lot_value([version([amt("estimated_value", 20_000_000_000)], [lot("LOT-1", [amt("estimated_value", 100_000_000_000)]), lot("LOT-2", [amt("estimated_value", 10_000_000_000)])])], "LOT-1"), 100_000_000_000)
check(t, "other field", lot_value([version([amt("framework_maximum", 100_000_000_000)], [lot("LOT-1", [amt("estimated_value", 6_000_000_000_000)]), lot("LOT-2", [amt("estimated_value", 40_000_000_000)])])], "LOT-1"), 6_000_000_000_000)
near = dict(essex, lots=[lot("LOT-1", [amt("estimated_value", 6_000_000_000_100)])] + essex["lots"][1:])
check(t, "near miss", lot_value([near], "LOT-1"), 6_000_000_000_100)
check(t, "under gate", lot_value([version([amt("estimated_value", 1_000_000_000)], [lot("LOT-1", [amt("estimated_value", 60_000_000_000)]), lot("LOT-2", [amt("estimated_value", 400_000_000)])])], "LOT-1"), 60_000_000_000)
sentinel_v = dict(essex, lots=essex["lots"][:2] + [lot("LOT-3", [amt("estimated_value", 25_000_000_000), amt("estimated_value", 9_999_999_999_999),
                                                                    amt("estimated_value", 90_000_000_000, quality="withheld")])])
check(t, "sentinel/withheld sibling", lot_value([sentinel_v], "LOT-1"), None)
circular = version([amt("estimated_value", 100_000_000_000)], [lot("LOT-1", [amt("estimated_value", 6_000_000_000_000)]),
                                                               lot("LOT-2", [amt("estimated_value", 1_000_000_000)]),
                                                               lot("LOT-3", [amt("estimated_value", 39_000_000_000)])])
check(t, "circular lot", lot_value([circular], "LOT-1"), None)
check(t, "circular head", one(circular), 100_000_000_000)
doubled = dict(circular, lots=[lot("LOT-1", [amt("estimated_value", 6_000_000_000_000), amt("estimated_value", 20_000_000_000_000)])] + circular["lots"][1:])
check(t, "doubled lot", lot_value([doubled], "LOT-1"), None)
check(t, "doubled head", one(doubled), 100_000_000_000)
filtered = version([amt("estimated_value", 200_000_000_000)], [lot("LOT-1", [amt("estimated_value", 100_000_000_000_000)]),
                                                               lot("LOT-2", [amt("estimated_value", 2_000_000_000)]),
                                                               lot("LOT-3", [amt("estimated_value", 98_000_000_000)])])
check(t, "filtered head", one(filtered), 98_000_000_000)
alone2 = version([amt("estimated_value", 100_000_000), amt("framework_maximum", 100_000_000_000)],
                 [lot("LOT-1", [amt("estimated_value", 100_000_000_000)]), lot("LOT-2", [amt("framework_maximum", 5_000_000)])])
check(t, "no sibling figure", lot_value([alone2], "LOT-1"), 100_000_000_000)
crumbs = version([amt("estimated_value", 123_456_789)], [lot("LOT-1", [amt("estimated_value", 100_000_000_000)]), lot("LOT-2", [amt("estimated_value", 123_356_789)])])
check(t, "crumbs", lot_value([crumbs], "LOT-1"), 100_000_000_000)
far = version([amt("estimated_value", 25_000)], [lot("LOT-1", [amt("estimated_value", 100_000_000_000)]), lot("LOT-2", [amt("estimated_value", 15_000)])])
check(t, "k=7", lot_value([far], "LOT-1"), 100_000_000_000)

# ------------------------------------------------------------------ a_total_ten_to_the_k_times_its_lot_award_sum_is_not_elected
t = "a_total_ten_to_the_k_times_its_lot_award_sum_is_not_elected"
key = lambda i: f"LOT-{i + 1:04d}"  # noqa: E731
f = 6_237_873_628_000
awards_ = [158_878_238, 3_367_786_341, 2_711_209_050]
bt709 = [158_878_238, 3_367_786_340, 2_711_209_050]


def migranti(awarded, partner_only, f=f):
    lots_ = [lot(key(i), [amt(RFM, bt709[i])] if partner_only else []) for i in range(3)]
    v = version([amt("result_value", f)], lots_)
    return with_awards(v, [award(key(i), awards_[i]) for i in range(3)]) if awarded else v


check(t, "369057 full", one(migranti(True, True)), None)
check(t, "369057 awards", one(migranti(True, False)), None)
check(t, "369057 BT-709", one(migranti(False, True)), None)
check(t, "369057 no lot figure", one(migranti(False, False)), f)
full = migranti(True, True)
rule = rule_of([full])
m = rule.lot_sum_match("EUR", f)
check(t, "369057 match", (m.kind, m.exponent, m.sum, m.lots), (E.AWARDS, 3, 6_237_873_629, 3))
check(t, "refuses P", rule.refuses_amount("result_value", "EUR", f, f, P), True)
check(t, "Lot(None) outside", rule.refuses_amount("result_value", "EUR", f, f, L(None)), False)
check(t, "HeadLot outside", rule.refuses_amount("result_value", "EUR", f, f, H("LOT-0001")), False)
usd = with_awards(version([amt("result_value", f)], [lot(key(i), []) for i in range(3)]), [award(key(i), awards_[i], "USD") for i in range(3)])
check(t, "USD sum", one(usd), f)
running = rule_of([full])
check(t, "asked again", running.lot_sum_match("EUR", f), m)
running.set_head(versions([version([amt("result_value", f)])])[0])
check(t, "new head new sums", running.lot_sum_match("EUR", f), None)
check(t, "new head sums empty", running.head_lot_sums_by_kind("EUR"), [])
corr = with_awards(version([amt("estimated_value", 200_000_000_000), amt("result_value", 200_000_000_000)], [lot(key(i), []) for i in range(2)]),
                   [award("LOT-0001", 150_000_000), award("LOT-0002", 50_000_000)])
check(t, "no corroboration exemption", one(corr), None)
lot_est = [673_000_000, 670_000_000, 677_289_170]
estimated = lambda proc: version(proc, [lot(key(i), [amt("estimated_value", lot_est[i])]) for i in range(3)])  # noqa: E731
cn = estimated([amt("estimated_value", 2_020_289_170)])
can = with_awards(estimated([amt("estimated_value", 2_020_289_170), amt("result_value", 1_997_527_462_600)]),
                  [award("LOT-0001", 635_231_758), award("LOT-0002", 676_534_710), award("LOT-0003", 685_760_994)])
check(t, "165437", elect_chain([cn, can]), 2_020_289_170)
tolls = [104_004_201] * 15
tolls[0] = 104_004_209
tolls[7] = 1_036_974_789
assert sum(tolls) == 2_493_033_611
v1 = version([amt("estimated_value", 2_500_000_000)], [lot(key(i), []) for i in range(15)])
v2 = with_awards(version([amt("result_value", 249_303_361_300)], [lot(key(i), [amt("result_value", tolls[i])]) for i in range(15)]),
                 [award(key(i), tolls[i]) for i in range(15)])
check(t, "6921980", elect_chain([v1, v2]), 1_036_974_789)
awarded = [92_171_806] * 14
awarded[0] = 92_171_805
hospital = with_awards(version([amt("result_value", 129_040_528_200), amt("estimated_value", 1_271_119_000)], [lot(key(i), []) for i in range(17)]),
                       [award(key(i), awarded[i]) for i in range(14)])
check(t, "801998", one(hospital), 1_271_119_000)
f = 357_400_427_200


def corrected(first, second):
    return with_rounds(version([amt("result_value", f)], [lot("LOT-0001", []), lot("LOT-0002", [])]),
                       [(10, [award("LOT-0001", 1_787_002_136), award("LOT-0002", first)]), (11, [award("LOT-0002", second)])])


check(t, "392980 latest award", one(corrected(1_973_910_683, 1_787_002_136)), None)
check(t, "392980 per-lot max misses", E.lot_sum_exponent(f, 1_787_002_136 + 1_973_910_683, 2), None)
check(t, "392980 other way round", one(corrected(1_787_002_136, 1_973_910_683)), f)
caps = [120_000_000, 110_000_000, 110_000_000]


def shelter(po):
    return version([amt("result_value", 340_000_000_000)],
                   [lot(key(i), [amt("framework_maximum", caps[i])] + ([amt(RFM, caps[i])] if po else [])) for i in range(3)])


check(t, "1024214 BT-709 as S", one(shelter(True)), 120_000_000)
check(t, "1024214 BT-271 alone fails T", one(shelter(False)), 340_000_000_000)
m = rule_of([shelter(False)]).lot_sum_match("EUR", 340_000_000_000)
check(t, "1024214 kept match", (m.kind, m.exponent, m.third_figure, m.refuses()), (E.FWMAX, 3, False, False))
fwk = lambda proc: version(proc, [lot(key(i), [amt("estimated_value", 147_000_000), amt("framework_maximum", 220_500_000)]) for i in range(4)])  # noqa: E731
check(t, "508666", one(fwk([amt("framework_maximum", 882_000_000_000), amt("estimated_value", 588_000_000)])), 588_000_000)
check(t, "508666 no X", one(fwk([amt("framework_maximum", 882_000_000_000)])), 882_000_000_000)
check(t, "508666 X earlier", elect_chain([fwk([amt("estimated_value", 588_000_000)]), fwk([amt("framework_maximum", 882_000_000_000)])]), 220_500_000)
zones = lambda proc: version(proc, [lot(key(i), [amt("estimated_value", c)]) for i, c in enumerate([100_000_000, 50_000_000, 26_500_000])])  # noqa: E731
check(t, "114991", one(zones([amt("estimated_value", 176_500_000_000), amt("result_value", 75_683_700)])), 100_000_000)
check(t, "114991 no X", one(zones([amt("estimated_value", 176_500_000_000)])), 176_500_000_000)
check(t, "114991 award copy no X", one(with_awards(zones([amt("estimated_value", 176_500_000_000), amt("result_value", 75_683_700)]),
                                                   [award("LOT-0003", 75_683_700)])), 176_500_000_000)


def catering(proc):
    lots_ = [lot(key(i), [amt("estimated_value", c)]) for i, c in enumerate([60_000_000, 10_000_000, 20_000_000, 10_000_000])]
    lots_.append(lot("GLO-0001", [amt("estimated_value", 100_000_000)], "LotsGroup"))
    return version(proc, lots_)


both = lambda: [amt("estimated_value", 100_000_000_000), amt("framework_maximum", 100_000_000_000)]  # noqa: E731
check(t, "143300 no T", one(catering(both())), 100_000_000_000)
check(t, "143300 T earlier", elect_chain([version([amt("estimated_value", 100_000_000)]), catering(both())]), 100_000_000)
f = 464_940_000_000
grouped_ = lambda proc: version(proc, [lot("LOT-0001", [amt("estimated_value", 388_080_000)]), lot("LOT-0002", [amt("estimated_value", 76_860_000)]),  # noqa: E731
                                       lot("GLO-0005", [amt("estimated_value", f)], "LotsGroup")])
check(t, "74787 as stored", one(grouped_([amt("estimated_value", f)])), f)
vch = grouped_([amt("estimated_value", f), amt("result_value", 400_000_000)])
check(t, "74787 group copy refused", one(vch), 400_000_000)
rule = rule_of([vch])
check(t, "74787 group lot value refused", rule.refuses_amount("estimated_value", "EUR", f, f, L("GLO-0005")), True)
check(t, "74787 Lot not tested", rule.refuses_amount("estimated_value", "EUR", f, f, H("LOT-0001")), False)
housing = with_awards(version([amt("estimated_value", 1_080_000_000_000), amt("result_value", 1_080_000_000_000)], [lot("LOT-0001", [amt("result_value", 1_080_000_000)])]),
                      [award("LOT-0001", 1_080_000_000)])
check(t, "8400892", one(housing), 1_080_000_000_000)
thousands = lambda proc: version(proc, [lot("LOT-0001", [amt("estimated_value", 20_000_000)]), lot("LOT-0002", [amt("estimated_value", 20_000_000)])])  # noqa: E731
check(t, "8312215", one(thousands([amt("estimated_value", 400_000_000_000)])), 400_000_000_000)
check(t, "8312215 T decides", one(thousands([amt("estimated_value", 400_000_000_000), amt("result_value", 35_000_000)])), 35_000_000)
check(t, "8720567", one(version([amt("estimated_value", 160_000_000_000)], [lot("LOT-0001", [amt("estimated_value", 800_000_000)]),
                                                                            lot("LOT-0002", [amt("estimated_value", 800_000_000)])])), 160_000_000_000)
check(t, "8596393 k=1", one(with_awards(version([amt("estimated_value", 130_000_000_000)], [lot(key(i), [amt("estimated_value", 6_500_000_000)]) for i in range(2)]),
                                        [award(key(i), 6_500_000_000) for i in range(2)])), 130_000_000_000)
check(t, "1039053", one(version([amt("framework_maximum", 520_000_000_000)],
                                [lot(key(i), [amt("estimated_value", 1_000_000_000), amt("framework_maximum", 10_000_000_000)]) for i in range(52)])), 520_000_000_000)
check(t, "7955574", one(with_awards(version([amt("result_value", 180_000_000_000)], [lot(key(i), [amt("estimated_value", 20_000_000_000)]) for i in range(9)]),
                                    [award(key(i), 20_000_000_000) for i in range(9)])), 180_000_000_000)
check(t, "below gate", one(with_awards(version([amt("result_value", 50_000_000_000)], [lot(key(i), []) for i in range(2)]),
                                       [award(key(i), 25_000_000) for i in range(2)])), 50_000_000_000)
f, p = 120_000_000_000, 1_200_000_000
exempt = with_awards(version([amt("estimated_value", f)], [lot("LOT-1", [amt("estimated_value", 60_000_000_000)]), lot("LOT-2", [amt("estimated_value", 58_800_000_000)]),
                                                           lot("LOT-3", [amt("estimated_value", p)])]),
                     [award("LOT-1", 600_000_000), award("LOT-2", 600_000_000)])
check(t, "492 framework wins", one(exempt), f)
cost = with_awards(version([amt("estimated_value", f)], [lot("LOT-1", [amt("estimated_value", 60_000_000_000)]), lot("LOT-2", [amt("estimated_value", 60_000_000_000)])]),
                   [award("LOT-1", 600_000_000), award("LOT-2", 600_000_000)])
check(t, "recorded cost", one(cost), 60_000_000_000)
bounded = with_awards(version([amt("result_value", 300_000_000_000)], [lot("LOT-1", [amt("estimated_value", 100_000_000_000)]), lot("LOT-2", [amt("estimated_value", 1_000_000_000)])]),
                      [award("LOT-1", 150_000_000), award("LOT-2", 150_000_000)])
check(t, "no admitted bound", rule_of([bounded]).refuses_amount("estimated_value", "EUR", 100_000_000_000, 100_000_000_000, L("LOT-1")), True)
check(t, "bounded head", one(bounded), 1_000_000_000)

# ------------------------------------------------------------------ the_lot_sums_count_each_lot_once_and_only_lots
t = "the_lot_sums_count_each_lot_once_and_only_lots"
sums = E.LotSums([("LOT-1", "Lot"), ("LOT-2", "Lot"), ("PAR-1", "Part"), ("GLO-1", "LotsGroup")])
sums.award("LOT-1", 10, 100_000)
sums.award("LOT-1", 11, 90_000)
sums.award("LOT-1", 9, 500_000)
sums.award("LOT-2", 12, 40_000)
sums.award("LOT-2", 12, 50_000)
sums.award("LOT-2", 13, 2_222_222_222_222)
sums.award("LOT-2", 13, 1_000)
sums.award("GLO-1", 10, 999_000)
sums.award("PAR-1", 10, 999_000)
sums.award("LOT-9", 10, 7_000)
sums.amount("LOT-1", "estimated_value", 300_000, None)
sums.amount("LOT-1", "estimated_value", 200_000, None)
sums.amount("LOT-2", "estimated_value", 400_000, "withheld")
sums.amount("PAR-1", "estimated_value", 400_000, None)
sums.amount("LOT-1", "result_value", 400_000, None)
sums.amount("LOT-2", RFM, 60_000, None)
sums.amount("LOT-9", "framework_maximum", 60_000, None)
check(t, "sums", sums.sums(), [(E.AWARDS, 147_000, 3), (E.BT709, 60_000, 1), (E.ESTIMATE, 300_000, 1)])
pins = E.LotSums([("PAR-1", "Part"), ("PAR-2", "Part")])
pins.amount("PAR-1", "estimated_value", 5_000, None)
pins.amount("PAR-2", "estimated_value", 6_000, None)
check(t, "PIN sums parts", pins.sums(), [(E.ESTIMATE, 11_000, 2)])
s = 1_000_000
check(t, "slack edge", E.lot_sum_exponent(100 * s + 300, s, 3), 2)
check(t, "past slack", E.lot_sum_exponent(100 * s + 301, s, 3), None)
check(t, "minus slack", E.lot_sum_exponent(100 * s - 300, s, 3), 2)
check(t, "k=1", E.lot_sum_exponent(10 * s, s, 3), None)
check(t, "ratio 1", E.lot_sum_exponent(s, s, 3), None)
check(t, "k=6", E.lot_sum_exponent(1_000_000 * s, s, 3), 6)
check(t, "k=7", E.lot_sum_exponent(10_000_000 * s, s, 3), None)
check(t, "one lot", E.lot_sum_exponent(1000 * s, s, 1), None)
check(t, "T bounds", E.lot_sum_vouches(1000 * s, s, s // 10) and E.lot_sum_vouches(1000 * s, s, 10 * s), True)
check(t, "T outside", (not E.lot_sum_vouches(1000 * s, s, s // 10 - 1)) and (not E.lot_sum_vouches(1000 * s, s, 10 * s + 1)), True)
check(t, "F itself", E.lot_sum_vouches(10 * s, s, 10 * s), False)
check(t, "placeholder", E.lot_sum_vouches(1000 * 9_000, 9_000, 1_000), False)
check(t, "sentinel X", E.lot_sum_vouches(1000 * 50_000_000, 50_000_000, 111_111_112) and not E.lot_sum_vouches(1000 * 50_000_000, 50_000_000, 111_111_111), True)
mm = E.match_lot_sums(1000 * s, [(E.AWARDS, s, 2), (E.ESTIMATE, s, 2)], lambda _: False)
check(t, "award side first", (mm.kind, mm.refuses()), (E.AWARDS, True))
check(t, "kept but named", E.match_lot_sums(1000 * s, [(E.ESTIMATE, s, 2)], lambda _: False).refuses(), False)
check(t, "T refuses", E.match_lot_sums(1000 * s, [(E.ESTIMATE, s, 2)], lambda _: True).refuses(), True)
check(t, "no match", E.match_lot_sums(1000 * s + 5_000, [(E.ESTIMATE, s, 2)], lambda _: True), None)

# ------------------------------------------------------------------ a_carried_part_is_no_head_candidate_beside_lots_that_state_a_figure
t = "a_carried_part_is_no_head_candidate_beside_lots_that_state_a_figure"
check(t, "8818621", one(version([amt("estimated_value", 118_762_000)], [lot("PAR-0001", [amt("estimated_value", 111_876_200_000)], "Part"),
                                                                        lot("LOT-0001", [amt("estimated_value", 28_288_000)]),
                                                                        lot("LOT-0002", [amt("estimated_value", 77_217_400)])])), 118_762_000)
check(t, "lots only", one(version([], [lot("PAR-0001", [amt("estimated_value", 70_000_000_000)], "Part"), lot("LOT-0001", [amt("estimated_value", 64_949_755)])])), 64_949_755)
check(t, "8819870 only part", one(version([], [lot("PAR-0001", [amt("estimated_value", 12_675_030_400)], "Part"), lot("LOT-0001", [])])), 12_675_030_400)
check(t, "PIN head", one(version([], [lot("PAR-0001", [amt("estimated_value", 5_000_000)], "Part")])), 5_000_000)
check(t, "withheld/sentinel", one(version([amt("estimated_value", -1, quality="withheld")],
                                          [lot("PAR-0001", [amt("estimated_value", 7_000_000)], "Part"), lot("LOT-0001", [amt("estimated_value", 99_999_999_999)])])), 7_000_000)
pin_v = version([amt("estimated_value", 1_200_000_000)], [lot("PAR-0000", [amt("estimated_value", 1_200_000_000)], "Part")])
cn_v = version([amt("estimated_value", 120_000_000_000)], [lot("PAR-0000", [amt("estimated_value", 1_200_000_000)], "Part"),
                                                           lot("LOT-0000", [amt("estimated_value", 120_000_000_000)])])
check(t, "8819939", elect_chain([pin_v, cn_v]), 1_200_000_000)
check(t, "unconvertible", one(version([amt("estimated_value", 900_000_000, "XYZ")], [lot("PAR-0001", [amt("estimated_value", 4_000_000)], "Part"), lot("LOT-0001", [])])), 4_000_000)

# ------------------------------------------------------------------ a_conversion_that_rounds_to_zero_elects_nothing
t = "a_conversion_that_rounds_to_zero_elects_nothing"
# HUF at 400 per EUR on 2024-01-01: the stored eur_cents is RatesLookup::eur_cents = round(cents / 400).
check(t, "HUF 1.48 -> 0", one(head([amt("estimated_value", 148, "HUF", eur=E.eur_cents_at(148, 400.0))])), None)
check(t, "HUF 4.00 -> 1", one(head([amt("estimated_value", 400, "HUF", eur=E.eur_cents_at(400, 400.0))])), 1)

# ------------------------------------------------------------------ one_unit_is_a_token_and_the_values_beside_it_are_not
t = "one_unit_is_a_token_and_the_values_beside_it_are_not"
for c, want in [(1, True), (100, True), (10, False), (200, False), (2, False), (99, False), (101, False)]:
    check(t, f"sentinel({c})", E.sentinel_amount(c), want)
check(t, "1.00 alone", val([amt("estimated_value", 100)]), None)
check(t, "1.00 beside", val([amt("estimated_value", 100), amt("estimated_value", 450_000_00)]), 450_000_00)

# ------------------------------------------------------------------ the_lot_election_skips_what_the_head_skips_and_keeps_the_published_figure
t = "the_lot_election_skips_what_the_head_skips_and_keeps_the_published_figure"


def la(field, cents, currency, eur, quality=None):
    return E.Amount(field, cents, currency, None, quality, eur)


none = E.ScalePartners()
pick = lambda xs, r: (lambda w: None if w is None else (w.field, w.cents, w.currency))(E.elect_lot_value(xs, r, "LOT-1"))  # noqa: E731
check(t, "empty", pick([], none), None)
check(t, "withheld", pick([la("estimated_value", -100, "EUR", -100, "withheld")], none), None)
check(t, "token", pick([la("estimated_value", 100, "GBP", 117)], none), None)
check(t, "zero", pick([la("estimated_value", 0, "EUR", 0)], none), None)
check(t, "ceiling", pick([la("result_value", 20_000_000_000_000_000, "EUR", 20_000_000_000_000_000), la("estimated_value", 5, "EUR", 5)], none), ("estimated_value", 5, "EUR"))
check(t, "no rate", pick([la("result_value", 20_000_000_000_000_000, "XYZ", None)], none), ("result_value", 20_000_000_000_000_000, "XYZ"))
check(t, "tie keeps first", pick([la("framework_maximum", 4_000_000, "XYZ", None), la("estimated_value", 4_000_000, "EUR", 4_000_000)], none), ("framework_maximum", 4_000_000, "XYZ"))
check(t, "published cents", pick([la("estimated_value", 2_000, "HUF", 5), la("estimated_value", 1_500, "EUR", 1_500)], none), ("estimated_value", 2_000, "HUF"))
rule = E.ScalePartners()
rule.add_partner("EUR", 3_000_000_000)
sl = 3_000_000_000_000
check(t, "slip refused", pick([la("framework_maximum", sl, "EUR", sl), la("estimated_value", 7, "EUR", 7)], rule), ("estimated_value", 7, "EUR"))
check(t, "needs conversion", pick([la("framework_maximum", sl, "XYZ", None)], rule), ("framework_maximum", sl, "XYZ"))

# ------------------------------------------------------------------ explanation sanity (not a Rust test): the reasons name the rule
t = "reasons (port-only)"
_, row, rs = run([migranti(True, True)])
check(t, "369057 reason", [(r["field"], r["status"], (r["detail"] or "")[:3]) for r in rs if r["status"] != "partner-only"],
      [("result_value", "refused", "537")])
_, row, rs = run([shelter(True)])
check(t, "1024214 elected row", (row["field"], row["cents"], row["lot_id"] is not None), ("framework_maximum", 120_000_000, True))
_, row, rs = run([version([amt("result_value", 8_000_000_000_000), amt("estimated_value", 8_000_000_000)])])
check(t, "6941544 reason", sorted((r["field"], r["status"], (r["detail"] or "")[:3]) for r in rs),
      [("estimated_value", "elected", ""), ("result_value", "refused", "471")])
_, row, rs = run([version([amt("estimated_value", 120_000_000_000)], [lot("LOT-1", [amt("estimated_value", 1_200_000_000)])])])
check(t, "474292 reason", sorted((r["field"], r["status"], (r["detail"] or "")[:3]) for r in rs),
      [("estimated_value", "elected", ""), ("estimated_value", "refused", "492")])
_, row, rs = run([essex])
check(t, "8784848 reason", [(r["cents"], r["status"], (r["detail"] or "")[:3]) for r in rs if r["status"] == "refused"],
      [(6_000_000_000_000, "refused", "505")])
_, row, rs = run([vch])
check(t, "74787 reason", sorted((r["scope"][:9], r["status"], (r["detail"] or "")[:9]) for r in rs if r["status"] == "refused"),
      [("head-lot ", "refused", "537 group"), ("procedure", "refused", "537 lot s")])
_, row, rs = run([version([amt("estimated_value", 118_762_000)], [lot("PAR-0001", [amt("estimated_value", 111_876_200_000)], "Part"),
                                                                  lot("LOT-0001", [amt("estimated_value", 28_288_000)])])])
check(t, "507 reason", [(r["scope"], r["status"]) for r in rs],
      [("procedure", "elected"), ("head-lot PAR-0001 (Part)", "part-set-aside"), ("head-lot LOT-0001 (Lot)", "admitted")])

# ------------------------------------------------------------------ lot_sum (port-only checks of the 551 helper)
t = "lot_sum (port-only)"
reg = Reg()
rows = to_rows(corrected(1_973_910_683, 1_787_002_136), reg, 1)
ls = E.lot_sum(rows["amounts"], rows["lots"], rows["results"], "EUR", {i: k for k, i in reg.lots.items()})
check(t, "392980 awards latest", (ls["awards"]["sum"], ls["awards"]["lots"]), (1_787_002_136 * 2, 2))
check(t, "392980 per-lot max", ls["per_lot_max_any_field"]["sum"], 1_787_002_136 + 1_973_910_683)
reg = Reg()
rows = to_rows(migranti(True, True), reg, 1)
ls = E.lot_sum(rows["amounts"], rows["lots"], rows["results"], "EUR", {i: k for k, i in reg.lots.items()})
check(t, "369057 partner-only excluded", sorted(ls), ["awards", "per_lot_max_any_field"])
check(t, "369057 award sum", ls["awards"]["sum"], 6_237_873_629)
reg = Reg()
rows = to_rows(dict(catering(both()), rounds=[]), reg, 1)
ls = E.lot_sum(rows["amounts"], rows["lots"], rows["results"], "EUR", {i: k for k, i in reg.lots.items()})
check(t, "LotsGroup excluded", (ls["estimated_value"]["sum"], ls["estimated_value"]["lots"]), (100_000_000, 4))

# ------------------------------------------------------------------ report
fails = [r for r in RESULTS if not r[2]]
by_test = {}
for test, label, ok, got, want in RESULTS:
    a = by_test.setdefault(test, [0, 0])
    a[0] += 1
    a[1] += ok
for test, (n, ok) in by_test.items():
    print(f"{'PASS' if n == ok else 'FAIL'} {ok}/{n}  {test}")
for test, label, ok, got, want in fails:
    print(f"  FAILED {test} :: {label}: got {got!r}, want {want!r}")
print(f"TOTAL {len(RESULTS) - len(fails)}/{len(RESULTS)} assertions agree")
sys.exit(1 if fails else 0)
