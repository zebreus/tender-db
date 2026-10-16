"""Python port of the head-value election, `crates/store/src/canonical.rs` @ 8c85af5:

    head_value_eur_cents_with   (3298-3389)  the head election (issues 366, 372, 378, 379, 506, 507)
    ScalePartners               (3672-4305)  scale rules 471 (k>=3), 492 (x100 + exemptions),
                                             505 (residual), 537 (lot sum + guard T + group copy)
    LotSums / LotSumKind        (4318-4466)  537's S
    lot_sum_exponent / lot_sum_vouches / match_lot_sums (4496-4535)
    sentinel_amount / repdigit_len (4974-5161)
    elect_lot_value             (3433-3455)  the stored lot value (ported for the tests; not used by elect())

Integer semantics follow the Rust: all comparisons are on i64 cents, the two EUR rescalings
(`admits_procedure`, `residual_lot_sum`) truncate toward zero after an i128 multiply, sums
saturate at i64::MAX.

PUBLIC API
----------
    elect(rows, lots, results, chain=None, lot_keys=None)
        -> (elected_eur_cents | None, elected_row | None, reasons: list[dict])

    rows     the HEAD version's tender_version_amounts rows
             (tender_id, seq, lot_id, field, cents, currency, tax_basis, eur_cents, quality),
             as tuples in that order or dicts with those keys.
    lots     the HEAD version's tender_version_lots rows: (lot_id, kind) or dicts
             {lot_id, kind[, lot_key]}.  kind in Lot | Part | LotsGroup.  REQUIRED when any row or
             result carries a lot_id: kind decides 507's Part set-aside, which lots count in 492/505/537's
             sums, and 537's LotsGroup copy rule.
    results  the HEAD version's tender_version_lot_results rows joined to lot_results:
             dicts {lot_result_id, lot_id, awarded_cents, awarded_currency, notice_id, result_key}
             (notice_id = lot_results.notice_id, the round's notice; result_key = lot_results.result_key).
             Needed for: 492's award-copy test (a tender-scope figure equal to a head award is scored
             Lot(None)), 492's lot count/lot sums, 537's award-side S (each lot's LATEST award by round
             notice id), and partners (471/492).
    chain    the EARLIER versions (seq < head seq), each a dict {amounts, lots, results} in the same
             shapes (seq order).  Only read for ScalePartners: partner figures (471/492: a partner
             may sit in any version), lot_only (492's framework exemption, 537's guard T: X may sit in
             any version), lot_keys (492 sibling-lot; never consulted by the head election).  None = a
             one-version chain (exact for one-version Tenders, otherwise loses those reaches).
    lot_keys {lot_id: lots.lot_key}.  Optional.  Only identity matters, so lot_id works as a key; pass the
             real keys to reproduce one exotic Rust collision (a lotless head's result key equal to an
             untyped award lot key, see SIMPLIFICATIONS).

    The EUR value of each candidate is the row's stored eur_cents (NULL = unconvertible).  Rust
    converts at election time with the same RatesLookup and the same date (civil_date of the head's
    published_at) that wrote eur_cents (EurContext, canonical.rs ~40065), so the two are equal by
    construction; partners and lot sums use published cents only.

    lot_sum(rows, lots, results, currency=None, lot_keys=None)
        -> {kind: {"sum", "lots", "per_lot": {lot: cents}}}            (currency given)
        -> {currency: {kind: {...}}}                                   (currency None)
       Per kind, the sum over the head's lots of ONE figure per lot, in ONE currency, partner-only
       rows (BT-709/BT-660, issue 506) excluded.  See lot_sum's docstring for the pick rules.

REASONS (one dict per amount row of the head, tender scope first, then each lot)
    status   elected | admitted | part-set-aside (507) | refused | not-electable | partner-only (506)
    detail   for refused: the rule ("471: exactly 10^k x partner ...", "492: exactly 100 x partner ...; no
             framework-total exemption / award copy / head-lot gets no exemption", "505 residual: ...",
             "537 lot sum: F ~ 10^k x <kind> sum S over n lots ...", "537 group copy: ...");
             for not-electable: withheld (372) / sentinel (366, 379) / no EUR conversion / over EUR 100 bn
             (366) / converts to EUR 0 (378).
    scope    procedure | award-copy (Lot(None)) (a tender-scope row equal to a head lot award, 492 (c)) |
             head-lot <key> (<kind>)
    lot_sum_match  for procedure rows: the 537 match even when it does not refuse (estimate side, no T).
    elect() asserts that why_refused() agrees with refuses() on every candidate.

SIMPLIFICATIONS (everything else is line-for-line)
 1. EUR: the stored eur_cents is used instead of RatesLookup at election time.  Equal by construction (same
    rates, same date: the fold writes eur_cents through EurContext with civil_date(published_at)); they can
    differ only if fetch-rates moved a rate and rederive-eur has not re-stamped the rows yet.
    eur_cents_at() reproduces RatesLookup::eur_cents (f64 divide, round half away from zero) for tests.
 2. Orders the store does not keep are proxied: lots by lot_id (Rust: the version's Vec order), rounds by
    notice_id, a round's results by lot_result_id, facts in Fact order (exact: BTreeSet).  Effects: (a) which
    row is REPORTED as elected among rows of equal eur_cents (Rust returns only the value; the read layer
    finds the row by eur_cents); (b) 505's residual_lot_sum takes the FIRST qualifying lot figure, so two
    lot figures of one field that are both residual slips of the same procedure figure could pick a
    different one; (c) 537's latest award on a tie of notice_id within one round (later offer wins);
    (d) add_version_figures' award map keeps the last of two awards with equal (currency, cents), which
    only feeds 492's sibling-lot key sets, never read by the head election.
 3. Lot identity: keys are lots.lot_key when lot_keys (or lots rows with lot_key) is given, else
    "lot#<lot_id>".  Identical outcomes except one exotic collision: in a head with no Lot/Part, 492 counts a
    lot-key-less award under its RESULT key (RES-n); Rust would merge it with an untyped award lot key of
    the same string.
 4. chain=None means a one-version chain.  Exact for one-version Tenders; otherwise it drops 471's partner
    in any version (6721266), 492's lot_only across versions (5864294), and 537's guard-T figure X from an
    earlier version (508666, 143300).  Versions with no amount / lot / result rows add nothing, so they
    need not be passed.
 5. The adapter de-duplicates identical facts per scope (BTreeSet); stored rows are already unique.
 6. Integer edge cases: sums saturate at i64::MAX as in Rust; Rust's `residual.checked_mul(10^k) == figure`
    is an exact product comparison here (equivalent: an overflowing product cannot equal an i64 figure);
    Rust's (debug) panic on i64 subtraction overflow is not reproduced.
 7. Not ported: the RatesLookup itself (canonical_currency legacy-code map, the daily-window rule) -- not
    needed given 1. -- and everything outside the election (lot value storage, read layer).
"""

from __future__ import annotations

from dataclasses import dataclass, field as dc_field
from typing import Any, Iterable, Optional

# ---------------------------------------------------------------- constants (canonical.rs)
QUALITY_WITHHELD = "withheld"
RESULT_FRAMEWORK_MAXIMUM = "result_framework_maximum"
RESULT_FRAMEWORK_REESTIMATE = "result_framework_reestimate"
PARTNER_ONLY_AMOUNT_FIELDS = (RESULT_FRAMEWORK_MAXIMUM, RESULT_FRAMEWORK_REESTIMATE)
IMPLAUSIBLE_EUR_CENTS = 10_000_000_000_000          # EUR 100 bn (366)
SCALE_ERROR_MIN_EXPONENT = 3                         # 471
SCALE_PARTNER_FLOOR_CENTS = 1_000                    # 10.00 (379/380)
SCALE_ERROR_MIN_EUR_CENTS = 100_000_000_000          # EUR 1 bn gate (471)
LOT_SUM_MIN_EXPONENT = 2                             # 537
LOT_SUM_MAX_EXPONENT = 6
LOT_SUM_MIN_LOTS = 2
I64_MAX = 2**63 - 1
I64_MIN = -(2**63)

# LotSumKind, in declaration order (= match_lot_sums' order, = BTreeMap order)
AWARDS, BT709, BT660, ESTIMATE, FWMAX = 0, 1, 2, 3, 4
KIND_LABEL = {AWARDS: "awards", BT709: "BT-709", BT660: "BT-660", ESTIMATE: "BT-27-Lot", FWMAX: "BT-271-Lot"}
KIND_FIELD = {AWARDS: None, BT709: RESULT_FRAMEWORK_MAXIMUM, BT660: RESULT_FRAMEWORK_REESTIMATE,
              ESTIMATE: "estimated_value", FWMAX: "framework_maximum"}

_MISSING = object()


# ---------------------------------------------------------------- integer helpers
def sat_add(a: int, b: int) -> int:
    return max(I64_MIN, min(I64_MAX, a + b))


def sat_sub(a: int, b: int) -> int:
    return max(I64_MIN, min(I64_MAX, a - b))


def tdiv(a: int, b: int) -> int:
    """Rust integer division (truncates toward zero)."""
    q = abs(a) // abs(b)
    return q if (a >= 0) == (b >= 0) else -q


def rust_round(x: float) -> int:
    """f64::round: half away from zero (the conversion rule of RatesLookup::eur_cents)."""
    import math
    r = math.floor(abs(x))
    if abs(x) - r >= 0.5:
        r += 1
    return int(r if x >= 0 else -r)


def eur_cents_at(cents: int, per_eur: float) -> int:
    """RatesLookup::eur_cents for a resolved rate (EUR: per_eur = 1.0)."""
    return rust_round(cents / per_eur)


# ---------------------------------------------------------------- sentinel_amount (366/379)
def repdigit_len(n: int) -> int:
    repeated = n % 10
    digits = 0
    while n > 0:
        if n % 10 != repeated:
            return 0
        n //= 10
        digits += 1
    return digits


def sentinel_amount(cents: int) -> bool:
    if cents < 0:
        return True
    if cents == 0:
        return True
    if cents == 1 or cents == 100:
        return True
    if repdigit_len(cents) >= 9:
        return True
    if cents % 100 != 0:
        return False
    return repdigit_len(cents // 100) >= 9


def electable_amount_field(field: str) -> bool:
    return field not in PARTNER_ONLY_AMOUNT_FIELDS


# ---------------------------------------------------------------- the in-memory TenderVersion
@dataclass
class Amount:
    field: str
    cents: int
    currency: str
    tax_basis: Optional[str] = None
    quality: Optional[str] = None
    eur_cents: Optional[int] = None      # stored conversion; None = unconvertible
    row: Any = None                      # the source row, echoed back

    def sort_key(self):
        # Fact's derived Ord for the Amount variant: field, cents, currency, tax_basis, quality
        # (Option: None < Some).
        return (self.field, self.cents, self.currency,
                (0, "") if self.tax_basis is None else (1, self.tax_basis),
                (0, "") if self.quality is None else (1, self.quality))


@dataclass
class Lot:
    key: Any
    kind: str
    facts: list = dc_field(default_factory=list)
    lot_id: Optional[int] = None


@dataclass
class Result:
    key: Any                  # result_key (RES-xxxx), local to its round
    lot_key: Any              # None = no lot
    cents: Optional[int]
    currency: Optional[str]


@dataclass
class Round:
    notice_id: int
    lot_results: list = dc_field(default_factory=list)


@dataclass
class Version:
    facts: list = dc_field(default_factory=list)     # tender-scope amounts, Fact order
    lots: list = dc_field(default_factory=list)      # Lot, Vec order
    rounds: list = dc_field(default_factory=list)    # Round, Vec order


def _dedup_sorted(facts):
    """A BTreeSet<Fact>: sorted, identical facts once."""
    out, seen = [], set()
    for f in sorted(facts, key=Amount.sort_key):
        k = f.sort_key()
        if k in seen:
            continue
        seen.add(k)
        out.append(f)
    return out


# ---------------------------------------------------------------- row adapters
AMOUNT_COLS = ("tender_id", "seq", "lot_id", "field", "cents", "currency", "tax_basis", "eur_cents", "quality")


def _as_amount_row(r) -> dict:
    return dict(r) if isinstance(r, dict) else dict(zip(AMOUNT_COLS, r))


def _as_lot_row(r) -> dict:
    if isinstance(r, dict):
        return r
    if len(r) == 2:
        return {"lot_id": r[0], "kind": r[1]}
    return {"lot_id": r[0], "kind": r[1], "lot_key": r[2]}


def version_from_rows(amounts, lots, results, lot_keys=None) -> Version:
    """Rebuild one TenderVersion from its stored rows.

    Order proxies (the Rust order is not stored): lots by lot_id; rounds by notice_id; a round's
    results by lot_result_id; facts in Fact order (sorted, which the Rust BTreeSet is).
    """
    lot_keys = dict(lot_keys or {})
    lot_rows = [_as_lot_row(r) for r in (lots or [])]
    for lr in lot_rows:
        if lr.get("lot_key") is not None:
            lot_keys.setdefault(lr["lot_id"], lr["lot_key"])

    def key_of(lot_id):
        return lot_keys.get(lot_id, f"lot#{lot_id}")

    by_lot = {}
    tender_facts = []
    for r in (amounts or []):
        d = _as_amount_row(r)
        a = Amount(d["field"], int(d["cents"]), d["currency"], d.get("tax_basis"), d.get("quality"),
                   None if d.get("eur_cents") is None else int(d["eur_cents"]), d)
        if d.get("lot_id") is None:
            tender_facts.append(a)
        else:
            by_lot.setdefault(d["lot_id"], []).append(a)
    v = Version(facts=_dedup_sorted(tender_facts))
    known = set()
    for lr in sorted(lot_rows, key=lambda x: x["lot_id"]):
        if lr["lot_id"] in known:
            continue
        known.add(lr["lot_id"])
        v.lots.append(Lot(key_of(lr["lot_id"]), lr["kind"], _dedup_sorted(by_lot.get(lr["lot_id"], [])), lr["lot_id"]))
    orphans = set(by_lot) - known
    if orphans:
        raise ValueError(f"amount rows at lot_ids {sorted(orphans)} that the version's tender_version_lots does not name")
    rounds = {}
    for r in sorted(results or [], key=lambda x: (x["notice_id"], x.get("lot_result_id") or 0)):
        rounds.setdefault(r["notice_id"], []).append(
            Result(r.get("result_key"), None if r.get("lot_id") is None else key_of(r["lot_id"]),
                   r.get("awarded_cents"), r.get("awarded_currency")))
    v.rounds = [Round(n, rs) for n, rs in rounds.items()]
    return v


# ---------------------------------------------------------------- scopes (FigureScope)
PROCEDURE = ("P", None)


def LOT(key):        # FigureScope::Lot(key)
    return ("L", key)


def HEADLOT(key):    # FigureScope::HeadLot(key)
    return ("H", key)


# ---------------------------------------------------------------- 537: LotSums & matching
def _lot_sum_figure(cents: int) -> bool:
    return cents > SCALE_PARTNER_FLOOR_CENTS and not sentinel_amount(cents)


def kind_of_field(field: str):
    for k, f in KIND_FIELD.items():
        if f is not None and f == field:
            return k
    return None


class LotSums:
    """One head version's candidate lot sums in ONE currency (canonical.rs LotSums)."""

    def __init__(self, lots: Iterable):
        lots = list(lots)
        unit = "Lot" if any(kind == "Lot" for _, kind in lots) else "Part"
        self.counted, self.excluded = set(), set()
        for key, kind in lots:
            (self.counted if kind == unit else self.excluded).add(key)
        self.awards = {}     # lot -> (notice_id, cents)
        self.amounts = {}    # (kind, lot) -> cents

    def award(self, lot, notice_id: int, cents: int):
        if lot in self.excluded or not _lot_sum_figure(cents):
            return
        latest = self.awards.setdefault(lot, (notice_id, cents))
        if notice_id >= latest[0]:
            self.awards[lot] = (notice_id, cents)

    def amount(self, lot, field: str, cents: int, quality):
        kind = kind_of_field(field)
        if kind is None:
            return
        if lot not in self.counted or quality is not None or not _lot_sum_figure(cents):
            return
        best = self.amounts.setdefault((kind, lot), cents)
        self.amounts[(kind, lot)] = max(best, cents)

    def sums(self):
        out = []
        if self.awards:
            s = 0
            for _, c in self.awards.values():
                s = sat_add(s, c)
            out.append([AWARDS, s, len(self.awards)])
        for (kind, lot) in sorted(self.amounts, key=lambda kl: (kl[0], str(kl[1]))):
            cents = self.amounts[(kind, lot)]
            if out and out[-1][0] == kind:
                out[-1][1] = sat_add(out[-1][1], cents)
                out[-1][2] += 1
            else:
                out.append([kind, cents, 1])
        return [tuple(x) for x in out]


@dataclass(frozen=True)
class LotSumMatch:
    kind: int
    exponent: int
    sum: int
    lots: int
    third_figure: bool

    def refuses(self) -> bool:
        return self.kind in (AWARDS, BT709, BT660) or self.third_figure

    def describe(self) -> str:
        return (f"F ~ 10^{self.exponent} x {KIND_LABEL[self.kind]} sum {self.sum} over {self.lots} lots"
                f" (slack {10**self.exponent * self.lots}; guard T {'holds' if self.third_figure else 'fails'})")


def lot_sum_exponent(f: int, s: int, lots: int):
    if f <= 0 or s <= 0 or lots < LOT_SUM_MIN_LOTS:
        return None
    for k in range(LOT_SUM_MIN_EXPONENT, LOT_SUM_MAX_EXPONENT + 1):
        scale = 10**k
        if abs(f - scale * s) <= scale * lots:
            return k
    return None


def lot_sum_vouches(f: int, s: int, x: int) -> bool:
    return x != f and x > SCALE_PARTNER_FLOOR_CENTS and not sentinel_amount(x) and x * 10 >= s and x <= s * 10


def match_lot_sums(f: int, sums, third_figure):
    kept = None
    for kind, s, lots in sums:
        e = lot_sum_exponent(f, s, lots)
        if e is None:
            continue
        m = LotSumMatch(kind, e, s, lots, bool(third_figure(s)))
        if m.refuses():
            return m
        if kept is None:
            kept = m
    return kept


# ---------------------------------------------------------------- ScalePartners
def _awards(v: Version):
    for rnd in v.rounds:
        for r in rnd.lot_results:
            if r.currency is not None and r.cents is not None:
                yield (r.lot_key, r.currency, r.cents)


def _scoped_amounts(v: Version):
    for f in v.facts:
        yield (f.field, f.currency, f.cents, None)
    for l in v.lots:
        for f in l.facts:
            yield (f.field, f.currency, f.cents, l.key)


def _amounts(v: Version):
    for field, cur, cents, _ in _scoped_amounts(v):
        yield (field, cur, cents)


class ScalePartners:
    def __init__(self):
        self.lot_keys = {}                 # (cur, cents) -> set | None
        self.head_procedure = []           # (field, cur, cents)
        self.head_lot_fields = {}          # (field, cur) -> {lot key: max cents}
        self.head_lot_field_figures = {}   # (field, cur) -> [(lot key, cents)]
        self.head_lot_count = 0
        self.head_lot_max = {}             # (lot key, cur) -> cents
        self.figures = set()               # (cur, cents)
        self.lot_only = {}                 # (cur, cents) -> bool
        self.head = {}                     # (cur, cents) -> field | None (None = two fields)
        self.head_lots = 0
        self.head_lot_sums = {}            # cur -> cents
        self.head_awards = set()           # (cur, cents)
        self.head_version: Optional[Version] = None
        self._lot_sums = None

    # -- building
    def add_partner(self, currency, cents):
        self._add_figure(currency, cents, PROCEDURE)

    def _add_figure(self, currency, cents, scope):
        if cents > SCALE_PARTNER_FLOOR_CENTS:
            k = (currency, cents)
            self.figures.add(k)
            lot = scope[0] == "L"
            self.lot_only[k] = (self.lot_only[k] and lot) if k in self.lot_only else lot
            if k not in self.lot_keys:
                self.lot_keys[k] = set()
            keys = self.lot_keys[k]
            if scope[0] == "L" and scope[1] is not None:
                if keys is not None:
                    keys.add(scope[1])
            else:
                self.lot_keys[k] = None

    def add_head_amount(self, field, currency, cents):
        if not electable_amount_field(field):
            return
        k = (currency, cents)
        if k in self.head:
            seen = self.head[k]
            if seen is not None and seen != field:
                self.head[k] = None
        else:
            self.head[k] = field

    def add_version_figures(self, v: Version):
        def sibling(key):
            if key is None:
                return None
            lot = next((l for l in v.lots if l.key == key), None)
            return key if (lot is None or lot.kind == "Lot") else None

        awards = {}
        for lot, cur, cents in _awards(v):
            awards[(cur, cents)] = lot          # HashMap collect: the last insert wins
        for _, cur, cents, lot in _scoped_amounts(v):
            if lot is not None:
                scope = LOT(sibling(lot))
            elif (cur, cents) in awards:
                scope = LOT(sibling(awards[(cur, cents)]))
            else:
                scope = PROCEDURE
            self._add_figure(cur, cents, scope)
        for lot, cur, cents in _awards(v):
            self._add_figure(cur, cents, LOT(sibling(lot)))

    def set_head(self, head: Version):
        self.head_version = head
        self._lot_sums = None
        self.head = {}
        for field, cur, cents in _amounts(head):
            self.add_head_amount(field, cur, cents)
        self.head_awards = {(cur, cents) for _, cur, cents in _awards(head)}
        self.head_procedure = [
            (f.field, f.currency, f.cents) for f in head.facts
            if f.quality is None and not sentinel_amount(f.cents) and electable_amount_field(f.field)
            and (f.currency, f.cents) not in self.head_awards
        ]
        unit = "Lot" if any(l.kind == "Lot" for l in head.lots) else "Part"
        not_lots = {l.key for l in head.lots if l.kind != unit}
        per_lot = {}
        keys = {l.key for l in head.lots if l.kind == unit}
        for lot in head.lots:
            if lot.kind != unit:
                continue
            for f in lot.facts:
                if (f.cents > SCALE_PARTNER_FLOOR_CENTS and f.quality is None and not sentinel_amount(f.cents)
                        and electable_amount_field(f.field)):
                    k = (lot.key, f.currency)
                    per_lot[k] = max(per_lot.get(k, 0), f.cents)
        lotless = not any(l.kind == unit for l in head.lots)
        for rnd in head.rounds:
            for r in rnd.lot_results:
                if r.currency is None or r.cents is None:
                    continue
                if r.lot_key is not None:
                    if r.lot_key in not_lots:
                        continue
                    lot = r.lot_key
                elif lotless:
                    lot = r.key
                else:
                    continue
                keys.add(lot)
                if r.cents > SCALE_PARTNER_FLOOR_CENTS and not sentinel_amount(r.cents):
                    k = (lot, r.currency)
                    per_lot[k] = max(per_lot.get(k, 0), r.cents)
        self.head_lots = len(keys)
        self.head_lot_sums = {}
        for (_, cur), c in per_lot.items():
            self.head_lot_sums[cur] = sat_add(self.head_lot_sums.get(cur, 0), c)
        self.head_lot_max = per_lot
        self.head_lot_count = sum(1 for l in head.lots if l.kind == "Lot")
        self.head_lot_fields = {}
        self.head_lot_field_figures = {}
        for lot in head.lots:
            if lot.kind != "Lot":
                continue
            for f in lot.facts:
                if f.quality is None and not sentinel_amount(f.cents) and electable_amount_field(f.field):
                    d = self.head_lot_fields.setdefault((f.field, f.currency), {})
                    d[lot.key] = max(d.get(lot.key, 0), f.cents)
                    self.head_lot_field_figures.setdefault((f.field, f.currency), []).append((lot.key, f.cents))

    @classmethod
    def of_chain(cls, chain):
        rule = cls()
        for v in chain:
            rule.add_version_figures(v)
        if chain:
            rule.set_head(chain[-1])
        return rule

    # -- partners
    def partner(self, currency, cents):
        if cents <= 0:
            return None
        for k in range(SCALE_ERROR_MIN_EXPONENT, 19):
            p = 10**k
            if not (cents // p > SCALE_PARTNER_FLOOR_CENTS):
                break
            if cents % p == 0 and (currency, cents // p) in self.figures:
                return cents // p
        return None

    def partner_x100(self, currency, cents):
        if cents > 0 and cents % 100 == 0 and cents // 100 > SCALE_PARTNER_FLOOR_CENTS:
            q = cents // 100
            return q if (currency, q) in self.figures else None
        return None

    def is_head_award(self, currency, cents) -> bool:
        return (currency, cents) in self.head_awards

    # -- 492 exemptions
    def framework_total(self, currency, cents, partner, lot_sum) -> bool:
        if self.head_lots < 2 or not self.lot_only.get((currency, partner), False):
            return False
        s = lot_sum if lot_sum is not None else self.head_lot_sums.get(currency)
        return s is not None and tdiv(cents, 10) <= s <= cents

    def sibling_lot(self, currency, cents, eur_cents, key, partner) -> bool:
        if self.head_lots < 2:
            return False
        keys = self.lot_keys.get((currency, partner))
        if not keys or key in keys:          # None (not lot-only-with-keys), empty, or this lot's own
            return False
        return any(c == currency and p >= cents and self.admits_procedure(f, c, p, cents, eur_cents, None)
                   for f, c, p in self.head_procedure)

    def admits_procedure(self, field, currency, procedure, cents, eur_cents, lot_sum) -> bool:
        eur = min(tdiv(eur_cents * procedure, cents), I64_MAX)
        return eur <= IMPLAUSIBLE_EUR_CENTS and not self.refuses(field, currency, procedure, eur, PROCEDURE, lot_sum)

    # -- 505 residual
    def residual_slip(self, field, currency, cents, eur_cents, key) -> bool:
        return self._residual_slip_detail(field, currency, cents, eur_cents, key) is not None

    def _residual_slip_detail(self, field, currency, cents, eur_cents, key):
        for f, c, p in self.head_procedure:
            if f == field and c == currency:
                residual = self.residual_of(f, c, p, key, cents)
                if residual is not None:
                    ls = self.lot_sum_at_residual(c, key, residual)
                    if self.admits_procedure(f, c, p, cents, eur_cents, ls):
                        return (p, residual)
        return None

    def residual_of(self, field, currency, procedure, key, figure):
        lots = self.head_lot_fields.get((field, currency))
        if lots is None:
            return None
        if self.head_lot_count < 2 or len(lots) < 2 or key not in lots:
            return None
        siblings = 0
        for k, c in lots.items():
            if k != key:
                siblings = sat_add(siblings, c)
        residual = procedure - siblings
        if (residual > SCALE_PARTNER_FLOOR_CENTS and residual >= tdiv(procedure, 100)
                and any(residual * 10**k == figure for k in range(2, 7))):
            return residual
        return None

    def lot_sum_at_residual(self, currency, key, residual):
        s = self.head_lot_sums.get(currency)
        if s is None:
            return None
        own = self.head_lot_max.get((key, currency), 0)
        return sat_add(sat_sub(s, own), residual)

    def residual_lot_sum(self, field, currency, procedure, eur_cents):
        figs = self.head_lot_field_figures.get((field, currency))
        if figs is None:
            return None
        for key, figure in figs:
            fe = min(tdiv(eur_cents * figure, procedure), I64_MAX)
            if not (SCALE_ERROR_MIN_EUR_CENTS <= fe <= IMPLAUSIBLE_EUR_CENTS) or \
                    self.refuses_by_partner(field, currency, figure, fe, HEADLOT(key), None):
                continue
            residual = self.residual_of(field, currency, procedure, key, figure)
            if residual is not None:
                ls = self.lot_sum_at_residual(currency, key, residual)
                if ls is not None:
                    return ls
        return None

    # -- the predicate
    def refuses_amount(self, field, currency, cents, eur_cents, scope) -> bool:
        return self.refuses(field, currency, cents, eur_cents, scope, None)

    def refuses(self, field, currency, cents, eur_cents, scope, lot_sum) -> bool:
        if eur_cents < SCALE_ERROR_MIN_EUR_CENTS:
            return False
        key = scope[1] if scope[0] in ("L", "H") else None
        if key is not None and self.residual_slip(field, currency, cents, eur_cents, key):
            return True
        if self.refuses_by_partner(field, currency, cents, eur_cents, scope, lot_sum):
            return True
        if scope[0] == "P":
            return self.lot_sum_slip(field, currency, cents, eur_cents, lot_sum)
        if key is not None:
            return self.lot_sum_group_copy(key, currency, cents, eur_cents)
        return False

    def lot_sum_group_copy(self, key, currency, cents, eur_cents) -> bool:
        return (any(c == currency and p == cents for _, c, p in self.head_procedure)
                and self.head_version is not None
                and any(l.key == key and l.kind == "LotsGroup" for l in self.head_version.lots)
                and any(c == currency and p == cents and self.lot_sum_slip(f, c, p, eur_cents, None)
                        for f, c, p in self.head_procedure))

    def lot_sum_slip(self, field, currency, cents, eur_cents, lot_sum) -> bool:
        if eur_cents > IMPLAUSIBLE_EUR_CENTS:
            return False
        m = self.lot_sum_match(currency, cents)
        return m is not None and m.refuses() and not self.framework_kept(field, currency, cents, eur_cents, lot_sum)

    def framework_kept(self, field, currency, cents, eur_cents, lot_sum) -> bool:
        if self.partner(currency, cents) is not None:
            return False
        p = self.partner_x100(currency, cents)
        if p is None:
            return False
        ls = lot_sum if lot_sum is not None else self.residual_lot_sum(field, currency, cents, eur_cents)
        return self.framework_total(currency, cents, p, ls)

    def head_lot_sums_by_kind(self, currency):
        return list(self._cached_lot_sums(currency))

    def _cached_lot_sums(self, currency):
        if self.head_version is None:
            return []
        if self._lot_sums is None:
            self._lot_sums = self._lot_sums_of(self.head_version)
        return self._lot_sums.get(currency, [])

    @staticmethod
    def _lot_sums_of(head: Version):
        by_cur = {}

        def new():
            return LotSums((l.key, l.kind) for l in head.lots)

        for lot in head.lots:
            for f in lot.facts:
                by_cur.setdefault(f.currency, new()).amount(lot.key, f.field, f.cents, f.quality)
        for rnd in head.rounds:
            for r in rnd.lot_results:
                if r.lot_key is not None and r.currency is not None and r.cents is not None:
                    by_cur.setdefault(r.currency, new()).award(r.lot_key, rnd.notice_id, r.cents)
        return {cur: s.sums() for cur, s in by_cur.items()}

    def lot_sum_match(self, currency, cents):
        if cents <= 0:
            return None

        def third(s):
            return any(c == currency and not only and lot_sum_vouches(cents, s, x)
                       for (c, x), only in self.lot_only.items())

        return match_lot_sums(cents, self._cached_lot_sums(currency), third)

    def refuses_by_partner(self, field, currency, cents, eur_cents, scope, lot_sum) -> bool:
        if self.partner(currency, cents) is None:
            p = self.partner_x100(currency, cents)
            if p is None:
                return False
            if scope[0] == "P":
                ls = lot_sum if lot_sum is not None else self.residual_lot_sum(field, currency, cents, eur_cents)
                return not self.framework_total(currency, cents, p, ls)
            if scope[0] == "L" and scope[1] is not None:
                return not self.sibling_lot(currency, cents, eur_cents, scope[1], p)
            return True
        seen = self.head.get((currency, cents), _MISSING)
        if seen is _MISSING:
            corroborated = False
        elif seen is None:
            corroborated = True
        else:
            corroborated = seen != field
        return not corroborated

    # -- explanation (same branches, same order as `refuses`; elect() asserts they agree)
    def why_refused(self, field, currency, cents, eur_cents, scope) -> Optional[str]:
        if eur_cents < SCALE_ERROR_MIN_EUR_CENTS:
            return None
        key = scope[1] if scope[0] in ("L", "H") else None
        if key is not None:
            d = self._residual_slip_detail(field, currency, cents, eur_cents, key)
            if d is not None:
                p, residual = d
                k = next(k for k in range(2, 7) if residual * 10**k == cents)
                return (f"505 residual: = 10^{k} x {residual}, what the admitted procedure {field} {p} leaves"
                        f" after the other Lots' {field}")
        if self.refuses_by_partner(field, currency, cents, eur_cents, scope, None):
            p = self.partner(currency, cents)
            if p is not None:
                k = len(str(cents // p)) - 1
                return (f"471: exactly 10^{k} x partner {p} {currency} (any version, any scope), not corroborated"
                        f" by a second electable field of the head")
            p = self.partner_x100(currency, cents)
            if scope[0] == "P":
                ls = self.residual_lot_sum(field, currency, cents, eur_cents)
                s = ls if ls is not None else self.head_lot_sums.get(currency)
                return (f"492: exactly 100 x partner {p}; no framework-total exemption (head_lots={self.head_lots},"
                        f" partner lot-only={self.lot_only.get((currency, p), False)}, lot sum={s},"
                        f" window [{tdiv(cents, 10)}, {cents}])")
            if scope[0] == "L" and scope[1] is None:
                return f"492: exactly 100 x partner {p}; a lot award's tender-scope copy gets no exemption"
            if scope[0] == "L":
                return f"492: exactly 100 x partner {p}; no sibling-lot exemption"
            return f"492: exactly 100 x partner {p}; a head-lot candidate gets no exemption"
        if scope[0] == "P" and self.lot_sum_slip(field, currency, cents, eur_cents, None):
            return "537 lot sum: " + self.lot_sum_match(currency, cents).describe()
        if key is not None and self.lot_sum_group_copy(key, currency, cents, eur_cents):
            return "537 group copy: a LotsGroup restating a procedure figure the lot-sum rule refuses"
        return None


# ---------------------------------------------------------------- the elections
def head_value(head: Version, partners: ScalePartners, explain: bool = False):
    """head_value_eur_cents_with: (eur, Amount) of the elected figure, or None.

    With explain=True returns (best, reasons) where reasons lists every Amount of the head.
    """
    has_lot = any(l.kind == "Lot" for l in head.lots)

    def candidates(with_parts):
        for f in head.facts:
            yield f, (LOT(None) if partners.is_head_award(f.currency, f.cents) else PROCEDURE), None
        for l in head.lots:
            if with_parts or l.kind != "Part":
                for f in l.facts:
                    yield f, HEADLOT(l.key), l

    def judge(f, scope):
        """(admitted_eur | None, status, detail)"""
        if not electable_amount_field(f.field):
            return None, "partner-only", "506: BT-709/BT-660 are never candidates (they only refuse, as partners and as 537 S)"
        if f.quality is not None:
            return None, "not-electable", f"372: quality={f.quality}"
        if sentinel_amount(f.cents):
            return None, "not-electable", "366/379: sentinel (<=0, 0.01, 1.00, or a 9+ repdigit)"
        eur = f.eur_cents
        if eur is None:
            return None, "not-electable", "no EUR conversion (eur_cents NULL)"
        if partners.refuses_amount(f.field, f.currency, f.cents, eur, scope):
            return None, "refused", partners.why_refused(f.field, f.currency, f.cents, eur, scope)
        if eur > IMPLAUSIBLE_EUR_CENTS:
            return None, "not-electable", "366: over EUR 100 bn"
        if eur == 0:
            return None, "not-electable", "378: converts to EUR 0"
        return eur, "admitted", None

    def elect(with_parts):
        best = None
        for f, scope, _ in candidates(with_parts):
            eur, _, _ = judge(f, scope)
            if eur is not None and (best is None or eur >= best[0]):   # Iterator::max keeps the LAST maximum
                best = (eur, f)
        return best

    if has_lot:
        first = elect(False)
        best = first if first is not None else elect(True)
    else:
        first = None
        best = elect(True)
    if not explain:
        return best
    reasons = []
    for f, scope, lot in candidates(True):
        eur, status, detail = judge(f, scope)
        refused = partners.refuses_amount(f.field, f.currency, f.cents, f.eur_cents, scope) \
            if (f.eur_cents is not None and electable_amount_field(f.field) and f.quality is None
                and not sentinel_amount(f.cents)) else False
        assert (status == "refused") == refused, (f, scope, status)
        if status == "refused":
            assert detail is not None, ("why_refused disagrees with refuses", f, scope)
        if status == "admitted":
            if lot is not None and lot.kind == "Part" and has_lot and first is not None:
                status, detail = "part-set-aside", "507: a Part is a candidate only when nothing else in a version with Lots is admitted"
            elif best is not None and f is best[1]:
                status = "elected"
            else:
                detail = "admitted, not the maximum"
        info = None
        if scope == PROCEDURE and electable_amount_field(f.field) and f.cents > 0:
            m = partners.lot_sum_match(f.currency, f.cents)
            if m is not None:
                info = "537 match: " + m.describe() + ("" if m.refuses() else " -> does not refuse (estimate side without T)")
        reasons.append({
            "row": f.row,
            "scope": {"P": "procedure", "L": "award-copy (Lot(None))", "H": "head-lot"}[scope[0]]
                     + ("" if lot is None else f" {lot.key} ({lot.kind})"),
            "field": f.field, "cents": f.cents, "currency": f.currency, "eur_cents": f.eur_cents,
            "status": status, "detail": detail, "lot_sum_match": info,
        })
    return best, reasons


def elect_versions(chain, explain=True):
    """The head election over in-memory Versions, oldest first; head = chain[-1]."""
    if not chain:
        return (None, None, []) if explain else None
    partners = ScalePartners.of_chain(chain)
    if not explain:
        best = head_value(chain[-1], partners)
        return None if best is None else best[0]
    best, reasons = head_value(chain[-1], partners, explain=True)
    return (None, None, reasons) if best is None else (best[0], best[1].row, reasons)


def elect(rows, lots=None, results=None, chain=None, lot_keys=None):
    """See the module docstring.  Returns (elected_eur_cents, elected_row, reasons)."""
    versions = [version_from_rows(v.get("amounts"), v.get("lots"), v.get("results"), lot_keys) for v in (chain or [])]
    versions.append(version_from_rows(rows, lots, results, lot_keys))
    return elect_versions(versions, explain=True)


def elect_lot_value(amounts, partners: ScalePartners, lot_key):
    """elect_lot_value: amounts are Amount (eur_cents may be None); returns the winning Amount."""
    best = None
    for a in amounts:
        if not electable_amount_field(a.field) or a.quality is not None or sentinel_amount(a.cents):
            continue
        if a.eur_cents is not None and a.eur_cents > IMPLAUSIBLE_EUR_CENTS:
            continue
        if a.eur_cents is not None and partners.refuses_amount(a.field, a.currency, a.cents, a.eur_cents, LOT(lot_key)):
            continue
        if best is None or a.cents > best.cents:
            best = a
    return best


# ---------------------------------------------------------------- the lot-sum figure (issue 551)
def lot_sum_version(head: Version, currency=None):
    """See lot_sum."""
    unit = "Lot" if any(l.kind == "Lot" for l in head.lots) else "Part"
    counted = {l.key for l in head.lots if l.kind == unit}
    excluded = {l.key for l in head.lots if l.kind != unit}
    out = {}   # cur -> kind -> lot -> cents
    latest = {}   # (cur, lot) -> (notice_id, cents)
    for lot in head.lots:
        if lot.key not in counted:
            continue
        for f in lot.facts:
            if (not electable_amount_field(f.field) or f.quality is not None
                    or not _lot_sum_figure(f.cents)):
                continue
            d = out.setdefault(f.currency, {}).setdefault(f.field, {})
            d[lot.key] = max(d.get(lot.key, 0), f.cents)
    for rnd in head.rounds:
        for r in rnd.lot_results:
            if r.lot_key is None or r.currency is None or r.cents is None:
                continue
            if r.lot_key in excluded or not _lot_sum_figure(r.cents):
                continue
            k = (r.currency, r.lot_key)
            prev = latest.setdefault(k, (rnd.notice_id, r.cents))
            if rnd.notice_id >= prev[0]:
                latest[k] = (rnd.notice_id, r.cents)
    for (cur, lot), (_, cents) in latest.items():
        out.setdefault(cur, {}).setdefault("awards", {})[lot] = cents
    # 492's per-lot maximum across every electable lot field and every award (head_lot_sums), for comparison
    for cur, kinds in list(out.items()):
        mx = {}
        for kind, per in kinds.items():
            for lot, c in per.items():
                mx[lot] = max(mx.get(lot, 0), c)
        # head_lot_sums takes the max over ALL awards, not the latest
        for rnd in head.rounds:
            for r in rnd.lot_results:
                if (r.lot_key is not None and r.currency == cur and r.cents is not None
                        and r.lot_key not in excluded and _lot_sum_figure(r.cents)):
                    mx[r.lot_key] = max(mx.get(r.lot_key, 0), r.cents)
        kinds["per_lot_max_any_field"] = mx

    def pack(per):
        s = 0
        for c in per.values():
            s = sat_add(s, c)
        return {"sum": s, "lots": len(per), "per_lot": dict(per)}

    res = {cur: {kind: pack(per) for kind, per in kinds.items()} for cur, kinds in out.items()}
    return res.get(currency, {}) if currency is not None else res


def lot_sum(rows, lots=None, results=None, currency=None, lot_keys=None):
    """Per kind, the sum over the head version's lots of ONE figure per lot, in ONE currency.

    Kinds: "awards" and one kind per electable lot-scope amount field present (estimated_value =
    BT-27-Lot, framework_maximum = BT-271-Lot, result_value = the lot's own result value, ...), plus
    "per_lot_max_any_field" (492's head_lot_sums shape, for comparison only).

    One figure per lot:
      - awards: the lot's LATEST award in that currency, by its round's notice id (lot_results.notice_id);
        a tie (same round) goes to the later lot_result_id.  537's LotSums rule (392980).
      - an amount field: the lot's LARGEST figure in that field and currency.  537's rule for BT-27/BT-271.
      - per_lot_max_any_field: the lot's largest figure over all of the above AND every award (not only
        the latest), i.e. 492's per-lot maximum.
    Which lots: kind Lot only; Parts only in a head with no Lot; never a LotsGroup.  An award naming a lot
    key the head does not type counts (as in LotSums and 492).
    Which figures: never a partner-only row (BT-709/BT-660, 506); never withheld (372), a sentinel, or a
    placeholder at or below 10.00 (SCALE_PARTNER_FLOOR_CENTS).  Lot figures that a scale rule would
    refuse are NOT removed (867491's LOT-0001 slip under the EUR 1 bn gate stays in).
    Differences from 537's S: partner-only kinds dropped; lot-scope result_value (and any other
    electable lot field) added as kinds of their own.
    Simplifications: one currency at a time, so a lot whose latest award is in another currency keeps
    its latest award in this one (LotSums does the same per currency); tender-scope rows equal to a lot
    award (the "award-copy" candidates the election falls to) are not lot figures here -- the award
    itself is counted under "awards"; per_lot_max_any_field skips lot-key-less awards, which 492 counts
    by result key in a head with no Lot/Part.
    """
    return lot_sum_version(version_from_rows(rows, lots, results, lot_keys), currency)
