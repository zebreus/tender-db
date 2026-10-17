"""Issue 551 unit 3: the ADOPTED merge (path V + path W), written from the issue's "## Unit 2 -- decided"
text alone, on top of elect.py (the faithful port of today's election).  It is the Python side of the
port check against the Rust build (`head_value_eur_cents_with` -> HeadValue).

    serve(entry, rates) -> dict(eur=..., derived=None | {...}, today=..., why=...)

entry  one ctx entry (head/chain/keys; `load_ctx()`).  rates {currency: per_eur} (units of currency per EUR),
       the SAME table the Rust side is handed, so both convert S identically (Rust: RatesLookup at the head's
       date; here (cents / per_eur) rounded half away from zero, the RatesLookup formula).

Inputs, all committed beside this file (unit 3's review made them so; they lived only in a session scratchpad):
- `data/ctx.json.gz`: unit 1's pull of the 4,745 records (the stored rows `elect.py` rebuilds each chain from;
  JSON of the old ctx.pkl, lot-id keys as strings, the loader's CA/CR sets as sorted lists), `load_ctx()`;
- `data/api/<id>.json`: unit 2's public API reads of the 8 fixtures not in the pull (`API_DUMPS`);
- `../results-slim.jsonl`: unit 1's per-record results (population, stored head), read by run_py.py;
- `../scripts/elect.py`: unit 1's port of today's election, imported here.
"""
from __future__ import annotations

import gzip
import json
import os
import re
import sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
sys.path.insert(0, os.path.join(HERE, "..", "scripts"))
import elect as E  # noqa: E402

# Unit 2's API reads, by tender id (the head version only).
API_DUMPS = {tid: os.path.join(DATA, "api", f"{tid}.json")
             for tid in (4930240, 928196, 7487018, 5625591, 467176, 294968, 7256284, 5977060)}


def load_ctx():
    """{tid: entry}, the 4,745 records as the old ctx.pkl held them (int ids, int lot-id keys)."""
    with gzip.open(os.path.join(DATA, "ctx.json.gz"), "rt") as f:
        raw = json.load(f)
    out = {}
    for tid, e in raw.items():
        e["keys"] = {int(k): v for k, v in e["keys"].items()}
        for k in ("CA", "CR"):
            if e.get(k) is not None:
                e[k] = {tuple(t) for t in e[k]}
        out[int(tid)] = e
    return out

# G8b (unit 3's second review round): a numbered lot key, once stripped: ASCII digits alone, or LOT in any case
# then digits, directly or after one '-', ' ' or '_' (TED's LOT-1, eForms' LOT-0001, Lot 2, LOT2, an OCDS 1).
_LOT_NUMBER = re.compile(r"(?:[Ll][Oo][Tt][-_ ]?)?([0-9]+)")


def lot_number(key):
    """canonical.rs `lot_number`: the lot number a numbered lot key spells, leading zeros dropped, else None."""
    m = _LOT_NUMBER.fullmatch(key.strip())
    return None if m is None else (m.group(1).lstrip("0") or "0")


def names_one_lot_twice(keys):
    """canonical.rs `names_one_lot_twice`: two DISTINCT keys spelling one lot number."""
    seen = {}
    for key in keys:
        n = lot_number(key)
        if n is not None and seen.setdefault(n, key) != key:
            return True
    return False


CAP = E.IMPLAUSIBLE_EUR_CENTS
ELECTABLE = (E.AWARDS, E.ESTIMATE, E.FWMAX)          # the kinds a derived head may serve (never BT-709/BT-660)
ORDER = {E.AWARDS: 0, E.ESTIMATE: 1, E.FWMAX: 2}     # tie order: Awards > Estimate > FrameworkMaximum


def rust_round(x: float) -> int:
    return E.rust_round(x)


def to_eur(rates, cur, cents):
    if cur == "EUR":
        return cents
    r = rates.get(cur)
    if r is None:
        return None
    return rust_round(cents / r)


def build(entry):
    keys = entry["keys"]
    versions = [E.version_from_rows(v.get("amounts"), v.get("lots"), v.get("results"), keys) for v in entry["chain"]]
    head = E.version_from_rows(entry["head"]["amounts"], entry["head"]["lots"], entry["head"]["results"], keys)
    versions.append(head)
    return versions, head, E.ScalePartners.of_chain(versions)


def lot_sums_by_currency(head):
    """ScalePartners._lot_sums_of, keeping each currency's LotSums object (per-lot figures)."""
    by = {}

    def new():
        return E.LotSums((l.key, l.kind) for l in head.lots)

    for lot in head.lots:
        for f in lot.facts:
            by.setdefault(f.currency, new()).amount(lot.key, f.field, f.cents, f.quality)
    for rnd in head.rounds:
        for r in rnd.lot_results:
            if r.lot_key is not None and r.currency is not None and r.cents is not None:
                by.setdefault(r.currency, new()).award(r.lot_key, rnd.notice_id, r.cents)
    return by


def per_lot(ls, kind):
    if ls is None:
        return {}
    if kind == E.AWARDS:
        return {lot: c for lot, (_, c) in ls.awards.items()}
    return {lot: c for (k, lot), c in ls.amounts.items() if k == kind}


def judge(f, scope, partners, rates, head_date_eur):
    """The election's judgement of one candidate, in the head election's order:
    partner-only / withheld / sentinel / no-eur / refused / over-cap / zero / admitted."""
    if not E.electable_amount_field(f.field):
        return "partner-only", None
    if f.quality is not None:
        return "withheld", None
    if E.sentinel_amount(f.cents):
        return "sentinel", None
    eur = head_date_eur(f)
    if eur is None:
        return "no-eur", None
    if partners.refuses_amount(f.field, f.currency, f.cents, eur, scope):
        return "refused", (refusal(partners, f, eur) if scope == E.PROCEDURE else None)
    if eur > CAP:
        return "over-cap", None
    if eur == 0:
        return "zero", None
    return "admitted", eur


def refusal(partners, f, eur):
    """why_refused's procedure branch: (rule, k, LotSumMatch | None)."""
    if partners.refuses_by_partner(f.field, f.currency, f.cents, eur, E.PROCEDURE, None):
        p = partners.partner(f.currency, f.cents)
        if p is not None:
            return ("471", len(str(f.cents // p)) - 1, None)
        return ("492", 2, None)
    assert partners.lot_sum_slip(f.field, f.currency, f.cents, eur, None), f
    m = partners.lot_sum_match(f.currency, f.cents)
    return ("537", m.exponent, m)


def matching_sum(f_cents, k, sums):
    best = None
    for kind, s, n in sums:
        if kind not in ELECTABLE:
            continue
        if E.lot_sum_exponent(f_cents, s, n) != k:
            continue
        key = (abs(f_cents - 10**k * s), ORDER[kind])
        if best is None or key < best[0]:
            best = (key, (kind, s, n))
    return None if best is None else best[1]


def elect_today(head, partners, conv):
    """head_value_eur_cents_with's max election: (eur, fact, scope) of the LAST maximum, or None."""
    has_lot = any(l.kind == "Lot" for l in head.lots)

    def run(with_parts):
        best = None
        for f in head.facts:
            scope = E.LOT(None) if partners.is_head_award(f.currency, f.cents) else E.PROCEDURE
            st, eur = judge(f, scope, partners, None, conv)
            if st == "admitted" and (best is None or eur >= best[0]):
                best = (eur, f, scope)
        for l in head.lots:
            if not with_parts and l.kind == "Part":
                continue
            for f in l.facts:
                st, eur = judge(f, E.HEADLOT(l.key), partners, None, conv)
                if st == "admitted" and (best is None or eur >= best[0]):
                    best = (eur, f, E.HEADLOT(l.key))
        return best

    if has_lot:
        b = run(False)
        return b if b is not None else run(True)
    return run(True)


def serve(entry, rates):
    versions, head, partners = build(entry)

    def conv(f):
        return to_eur(rates, f.currency, f.cents) if (f.currency == "EUR" or f.currency in rates) else None

    today = elect_today(head, partners, conv)
    today_eur = None if today is None else today[0]
    out = {"today": today_eur, "eur": today_eur, "derived": None, "why": "max election"}
    procs = []      # (fact, status, refusal)
    copies = []     # (fact, status)
    for f in head.facts:
        if partners.is_head_award(f.currency, f.cents):
            st, _ = judge(f, E.LOT(None), partners, rates, conv)
            copies.append((f, st))
            continue
        st, info = judge(f, E.PROCEDURE, partners, rates, conv)
        if st == "partner-only":
            continue
        procs.append((f, st, info if st == "refused" else None))
    by = lot_sums_by_currency(head)

    # ---------------- PATH V: the vouched lot sum
    v_tried = False
    if procs and not any(st == "admitted" for _, st, _ in procs):
        found = []
        for f, st, ref in procs:
            if st != "refused":
                continue
            rule, k, m = ref
            ls = by.get(f.currency)
            sums = ls.sums() if ls is not None else []
            if rule == "537" and m.kind in ELECTABLE:
                hit = (m.kind, m.sum, m.lots)
            else:
                hit = matching_sum(f.cents, k, sums)
            if hit is not None:
                found.append((f, rule, k, hit))
        if found:
            v_tried = True
            if len({(f.currency, hit[1]) for f, _, _, hit in found}) != 1:
                out["why"] = "V: refusals disagree on S"
                return out
            f0, rule0, k0, (kind0, s0, n0) = found[0]
            # V1: every summed lot repeats one figure M and F ~ 10^j x M (1 %)
            for f, _, _, (kind, s, n) in found:
                vals = list(per_lot(by.get(f.currency), kind).values())
                if vals and len(set(vals)) == 1:
                    M = vals[0]
                    for j in range(0, 19):
                        if 100 * abs(f.cents - 10**j * M) <= f.cents:
                            out["why"] = "V1: the lots repeat the slipped total"
                            return out
            eur = to_eur(rates, f0.currency, s0)
            if eur is None or eur <= 0 or eur > CAP:
                out["why"] = "V: S has no EUR value in (0, cap]"
                return out
            # V0 (added at unit 3's review): never below today's figure.
            if today_eur is not None and eur <= today_eur:
                out["why"] = "V0: not above today's figure"
                return out
            out.update(eur=eur, why="V", derived={
                "path": "lot-sum-vouched", "rule": rule0, "k": k0, "kind": kind0, "cents": s0,
                "currency": f0.currency, "lots": n0, "f_field": f0.field, "f_cents": f0.cents,
                "f_currency": f0.currency})
            return out

    # ---------------- PATH W: the unvouched awards lot sum
    if any(st != "sentinel" for _, st, _ in procs):
        out["why"] = "W/T': a procedure figure is not a sentinel"
        return out
    if today is not None and today[2] != E.LOT(None):
        out["why"] = "W/K: today's head is not an award copy"
        return out
    if today is not None:
        cur = today[1].currency
    else:
        cands = [(len(ls.awards), c) for c, ls in by.items() if ls.awards]
        if not cands:
            out["why"] = "W/K: no award lots"
            return out
        cur = max(cands)[1]
    ls = by.get(cur)
    per = per_lot(ls, E.AWARDS)
    n = len(per)
    if n < 2:
        out["why"] = "W/G0: fewer than 2 award lots"
        return out
    S = 0
    for c in per.values():
        S = E.sat_add(S, c)
    s_eur = to_eur(rates, cur, S)
    if s_eur is None:
        out["why"] = "W/G0: no EUR value"
        return out
    if today_eur is not None and s_eur <= today_eur:
        out["why"] = "W/G0: not above today's figure"
        return out
    # G5: S in (0, cap] (the lower bound added in unit 3's third review round, as path V's), and no
    # procedure or award-copy figure over the cap.
    if s_eur <= 0 or s_eur > CAP or any(st == "over-cap" for _, st, _ in procs) or any(st == "over-cap" for _, st in copies):
        out["why"] = "W/G5: cap"
        return out
    # G5' (added at unit 3's review): an award copy a scale rule refused (the copy of a x1000 lot award).
    if any(st == "refused" for _, st in copies):
        out["why"] = "W/G5': a refused award copy"
        return out
    # G5' (widened in unit 3's third review round): a summed award a scale rule refuses at its own lot,
    # copy or none (canonical.rs judges it as a `result_value` at Lot(Some(key))).
    for lot, c in per.items():
        e = to_eur(rates, cur, c)
        if e is not None and partners.refuses_amount("result_value", cur, c, e, E.LOT(lot)):
            out["why"] = "W/G5': a refused summed award"
            return out
    if any(c != cur and other.awards for c, other in by.items()):
        out["why"] = "W/G6: award lots in another currency"
        return out
    if any(st == "withheld" for _, st, _ in procs):
        out["why"] = "W/G7: withheld"
        return out
    if any(lot not in ls.counted for lot in per):
        out["why"] = "W/G8: an award names an untyped lot"
        return out
    # G8b (unit 3's second review round): two distinct summed keys spelling one lot number (a re-keyed chain
    # whose both key families are typed Lots would sum each lot twice).
    if names_one_lot_twice(per):
        out["why"] = "W/G8b: two keys spell one lot number"
        return out
    cnt = Counter(per.values())
    mx = max(per.values())
    if any(c >= 2 and v * 10 >= mx for v, c in cnt.items()):
        out["why"] = "W/G1': a repeated figure"
        return out
    if any(c >= 4 and 2 * c >= n for v, c in cnt.items()):
        out["why"] = "W/G2: a figure on >= 4 lots and >= half"
        return out
    if n >= 3:
        rest = S - mx
        for k in range(0, 4):
            t = rest * 10**k
            if t > 0 and 100 * abs(mx - t) <= mx:
                out["why"] = "W/G3: one lot is 10^k x the others"
                return out
    for kind in (E.ESTIMATE, E.FWMAX):
        p2 = per_lot(ls, kind)
        common = set(p2) & set(per)
        if len(common) >= 2:
            a = sum(per[l] for l in common)
            b = sum(p2[l] for l in common)
            if a > 10 * b or b > 10 * a:
                out["why"] = "W/G4: estimates disagree by 10x"
                return out
    out.update(eur=s_eur, why="W", derived={"path": "lot-sum-awards", "kind": E.AWARDS, "cents": S,
                                            "currency": cur, "lots": n})
    return out


def rates_of(entry):
    """{currency: per_eur} from the head's rows (the largest-cents row with a conversion), else the chain's."""
    best = {}
    pools = [entry["head"]["amounts"]] + [v.get("amounts") or [] for v in entry["chain"]]
    for pi, pool in enumerate(pools):
        for a in pool:
            cur, c, e = a["currency"], a["cents"], a.get("eur_cents")
            if cur == "EUR" or e is None or not c or c <= 0 or e <= 0:
                continue
            key = (0 if pi == 0 else 1, -c)
            if cur not in best or key < best[cur][0]:
                best[cur] = (key, c / e)
    return {cur: v[1] for cur, v in best.items()}
