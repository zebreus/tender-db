"""551 unit 2, design B: the MINIMAL lot-sum fallback for a refused head.

RULE B (core, the one proposed)
-------------------------------
Keep head_value_eur_cents_with's max election everywhere, except in one shape:

  (1) no procedure-scope figure of the head survives (none elected/admitted; award copies,
      Lot(None), are not procedure figures), AND
  (2) at least one procedure figure F was REFUSED by a scale rule, AND that refusal itself matched a
      head lot sum S:
        - 537: S is the very sum named by the refusal (LotSumMatch.sum, kind, exponent k);
          when that kind is partner-only (BT-709/BT-660, issue 506) S is replaced by the
          electable lot sum (awards / BT-27-Lot / BT-271-Lot / lot result_value) that matches F at
          the SAME exponent k with 537's slack (|F - 10^k S'| <= 10^k n'), closest first;
        - 471 / 492: the refusal says F = 10^k x partner p; S is an electable lot sum matching F at
          that k with 537's slack (lot_sum_exponent(F, S', n') == k),
      AND every refused procedure figure that yields an S yields the SAME S (else keep), AND
  (3) S converts to more than EUR 0 and at most EUR 100 bn (366's cap), at the refused F's own rate,
  -> serve S (EUR cents), marked DERIVED ("lot sum S, kind K over n lots, that rule R's refusal of F
     matched at 10^k").  Otherwise: the max election, unchanged.

The served value REPLACES the max election in that shape, even when one admitted lot figure of
another kind is larger (never observed in the 4,745; reported by score_B.py if it happens).

R-SHAPE EXTENSION (evaluated, NOT proposed -- see score_B.py's tables)
----------------------------------------------------------------------
For a head with NO procedure figure at all (proc_class 'absent'), serve the same-kind lot sum only if
  R1  the head has no procedure-scope row, and no head row is refused or over EUR 100 bn;
  R2  no lot-null row repeats a figure of two or more lots, or of a lotless award (the "procedure
      total stored lot-null and scored as an award copy" class);
  R3  the summed kind K is the elected figure's kind (awards for an award copy / a null head);
      K's figures sit on >= 2 lots and are PAIRWISE DISTINCT;
  R4  no other electable kind in that currency repeats one figure on >= 2 lots;
  R5  no lot's K figure equals the sum of the others (within 1 %);
  R6  S <= EUR 100 bn.
Variant 'r_shape_sentinel' also admits a head whose only procedure figures are sentinels (0, 0.01,
1.00, repdigits: 59219), keeping R1's other conditions.

R-SPLIT (the narrowest R-shape, evaluated): r_shape2 AND the summed kind is awards AND the served figure is
an award copy of one lot AND the lots' latest awards come from >= 2 different award notices (each lot awarded by its
own CAN; the head version's tender-scope total is only the LAST notice's lot: 5995030, and in the holdout 294968,
467176, 7256284, 5977060).

API
---
    rule_B(rec, detail=None, variant="core") -> (eur_cents | None, reason, derived: bool)
        rec      one results.jsonl record
        detail   details()[rec["id"]] (needed only for the R-shape variants)
        variant  "core" | "r_shape" | "r_shape_sentinel" | "r_shape2" | "r_split"   (the R variants include core)
    details()    {tender_id: per-lot figures, lot-null copy facts} rebuilt from u1/data via load.py
"""
from __future__ import annotations

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
M551 = os.path.dirname(HERE)
sys.path.insert(0, M551)
sys.path.insert(0, os.path.join(M551, "u1"))
import elect as E  # noqa: E402

CAP = E.IMPLAUSIBLE_EUR_CENTS
LABEL_KIND = {"awards": "awards", "BT-27-Lot": "estimated_value", "BT-271-Lot": "framework_maximum",
              "BT-709": None, "BT-660": None}
KIND_ORDER = ["awards", "estimated_value", "framework_maximum", "result_value"]
RE_537 = re.compile(r"^537 lot sum: F ~ 10\^(\d+) x (\S+) sum (\d+) over (\d+) lots")
RE_471 = re.compile(r"^471: exactly 10\^(\d+) x partner (\d+)")
RE_492 = re.compile(r"^492: exactly 100 x partner (\d+)")


def _kinds(rec, cur):
    """Electable lot sums of the record in one currency: {kind: (sum, lots)} (partner-only already dropped)."""
    out = {}
    for k, v in (rec.get("lot_sums") or {}).get(cur, {}).items():
        if k != "per_lot_max_any_field":
            out[k] = (v["sum"], v["lots"])
    return out


def _korder(k):
    return (KIND_ORDER.index(k) if k in KIND_ORDER else len(KIND_ORDER), k)


def _matching_sum(rec, cur, f, k):
    """The electable lot sum matching F at exponent k with 537's slack; closest, then awards first."""
    best = None
    for kind, (s, n) in _kinds(rec, cur).items():
        if E.lot_sum_exponent(f, s, n) == k:
            key = (abs(f - 10**k * s), _korder(kind))
            if best is None or key < best[0]:
                best = (key, kind, s, n)
    return None if best is None else best[1:]


def _to_eur(s, f_cents, f_eur):
    """S at the refused F's own rate (the same date and RatesLookup the head's rows were stamped with)."""
    if f_eur is None or f_cents <= 0:
        return None
    q, r = divmod(s * f_eur, f_cents)
    return q + (1 if 2 * r >= f_cents else 0)


def core(rec):
    """-> (eur | None, reason) when rule B fires, else None."""
    reasons = rec.get("reasons") or []
    proc = [x for x in reasons if x["scope"] == "procedure" and x["status"] != "partner-only"]
    if not proc or any(x["status"] in ("elected", "admitted") for x in proc):
        return None
    found = []
    for x in proc:
        if x["status"] != "refused" or not x.get("detail"):
            continue
        f, cur, d = x["cents"], x["currency"], x["detail"]
        m = RE_537.match(d)
        hit = None
        if m:
            k, label, s, n = int(m.group(1)), m.group(2), int(m.group(3)), int(m.group(4))
            kind = LABEL_KIND.get(label)
            if kind is not None and kind in _kinds(rec, cur) and _kinds(rec, cur)[kind] == (s, n):
                hit = ("537", k, kind, s, n)
            else:   # partner-only S (506) or a kind the record does not carry: the electable sum at the same k
                alt = _matching_sum(rec, cur, f, k)
                if alt is not None:
                    hit = ("537 via " + label, k) + alt
        else:
            m = RE_471.match(d)
            k = int(m.group(1)) if m else (2 if RE_492.match(d) else None)
            if k is not None:
                alt = _matching_sum(rec, cur, f, k)
                if alt is not None:
                    hit = (d[:3], k) + alt
        if hit is not None:
            found.append((x, hit))
    if not found:
        return None
    sums = {(x["currency"], hit[3]) for x, hit in found}
    if len(sums) != 1:
        return None, None   # sentinel: conflicting S -> keep (caller treats as no fire)
    x, (rule, k, kind, s, n) = found[0]
    eur = _to_eur(s, x["cents"], x["eur_cents"])
    if eur is None or eur <= 0 or eur > CAP:
        return None
    return eur, (f"derived: {kind} lot sum {s} {x['currency']} over {n} lots, matched by the {rule} refusal of "
                 f"{x['field']} {x['cents']} (F ~ 10^{k} x S)")


# ---------------------------------------------------------------- R-shape (evaluated, not proposed)
def r_shape(rec, detail, allow_sentinel_proc=False):
    """-> (eur, reason) when the R-shape guard admits the same-kind lot sum, else (None, why_not)."""
    if detail is None:
        return None, "no detail"
    reasons = rec.get("reasons") or []
    proc = [x for x in reasons if x["scope"] == "procedure" and x["status"] != "partner-only"]
    if proc:
        if not allow_sentinel_proc:
            return None, "R1: a procedure figure exists"
        if not all(x["status"] == "not-electable" and (x["detail"] or "").startswith("366/379") for x in proc):
            return None, "R1: a non-sentinel procedure figure exists"
    if any(x["status"] == "refused" for x in reasons):
        return None, "R1: a head row is refused"
    if any(x["status"] == "not-electable" and (x["detail"] or "").startswith("366: over") for x in reasons):
        return None, "R1: a head row is over EUR 100 bn"
    for ln in detail["lot_null"]:
        if ln["lotless"] or len(ln["copy_of"]) >= 2:
            return None, "R2: a lot-null figure repeats on >= 2 lots or a lotless award"
    el = rec.get("elected")
    if el is not None:
        cur = el["currency"]
        kind = "awards" if rec["elected_scope"] == "award-copy" else el["field"]
    else:
        nl = rec.get("null_lot_sum")
        if not nl:
            return None, "R3: no lot figure"
        cur = nl["currency"]
        kinds = detail["per_lot"].get(cur, {})
        kind = next((k for k in KIND_ORDER if len(kinds.get(k, {})) >= 2), None)
        if kind is None:
            return None, "R3: no kind on >= 2 lots"
    per = detail["per_lot"].get(cur, {}).get(kind, {})
    vals = list(per.values())
    if len(vals) < 2:
        return None, "R3: summed kind on < 2 lots"
    if len(set(vals)) != len(vals):
        return None, "R3: summed kind not pairwise distinct"
    for k2, p2 in detail["per_lot"].get(cur, {}).items():
        if k2 == kind:
            continue
        v2 = list(p2.values())
        if len(v2) >= 2 and len(set(v2)) < len(v2):
            return None, f"R4: {k2} repeats a figure across lots"
    s = sum(vals)
    mx = max(vals)
    if abs(mx - (s - mx)) <= mx // 100:
        return None, "R5: one lot = sum of the others"
    # EUR at the elected figure's rate (or the head's own rows' rate for a null head)
    if el is not None:
        eur = _to_eur(s, el["cents"], el["eur_cents"])
    else:
        rate = detail["rate"].get(cur)
        eur = None if rate is None else _to_eur(s, rate[0], rate[1])
    if eur is None or eur <= 0 or eur > CAP:
        return None, "R6: no EUR or over EUR 100 bn"
    return eur, f"derived (R-shape): {kind} lot sum {s} {cur} over {len(vals)} lots, no procedure figure"


def r_shape2(rec, detail):
    """The tuned R-shape guard (variant "r_shape2"): r_shape with three changes, each aimed at one labelled member.
      R3' K's figures need not be pairwise distinct: no one figure may sit on 2+ lots if that is half the lots or
          more, nor on 3+ lots at all (5995030: 34 distinct of 35; 4715842: 9 of 10).
      R4' a figure repeated on >= 2 lots in ANOTHER kind (partner-only BT-709/BT-660 included) does not block;
          it caps: S must not exceed it by more than 1 % (585917: BT-709 EUR 250 k x2, awards sum EUR 250 k).
      R5' one lot equal to 10^j x the sum of the others (j = 0..3, within 1 %) blocks (4787274: a x100
          object-level pseudo-lot).
    """
    if detail is None:
        return None, "no detail"
    reasons = rec.get("reasons") or []
    if [x for x in reasons if x["scope"] == "procedure" and x["status"] != "partner-only"]:
        return None, "R1: a procedure figure exists"
    if any(x["status"] == "refused" for x in reasons):
        return None, "R1: a head row is refused"
    if any(x["status"] == "not-electable" and (x["detail"] or "").startswith("366: over") for x in reasons):
        return None, "R1: a head row is over EUR 100 bn"
    for ln in detail["lot_null"]:
        if ln["lotless"] or len(ln["copy_of"]) >= 2:
            return None, "R2: a lot-null figure repeats on >= 2 lots or a lotless award"
    el = rec.get("elected")
    if el is not None:
        cur = el["currency"]
        kind = "awards" if rec["elected_scope"] == "award-copy" else el["field"]
    else:
        nl = rec.get("null_lot_sum")
        if not nl:
            return None, "R3: no lot figure"
        cur = nl["currency"]
        kinds = detail["per_lot"].get(cur, {})
        kind = next((k for k in KIND_ORDER if len(kinds.get(k, {})) >= 2), None)
        if kind is None:
            return None, "R3: no kind on >= 2 lots"
    per = detail["per_lot"].get(cur, {}).get(kind, {})
    vals = list(per.values())
    n = len(vals)
    if n < 2:
        return None, "R3: summed kind on < 2 lots"
    from collections import Counter
    mult = max(Counter(vals).values())
    if mult >= 3 or (mult == 2 and 2 * mult >= n):
        return None, "R3': summed kind repeats a figure"
    s = sum(vals)
    for k2, p2 in detail["per_lot"].get(cur, {}).items():
        if k2 == kind:
            continue
        rep = [v for v, c in Counter(p2.values()).items() if c >= 2]
        if rep and s * 100 > min(rep) * 101:
            return None, f"R4': {k2} repeats {min(rep)} on 2+ lots and the sum exceeds it"
    mx = max(vals)
    rest = s - mx
    if rest > 0 and any(abs(mx - 10**j * rest) * 100 <= mx for j in range(0, 4)):
        return None, "R5': one lot = 10^j x the sum of the others"
    if el is not None:
        eur = _to_eur(s, el["cents"], el["eur_cents"])
    else:
        rate = detail["rate"].get(cur)
        eur = None if rate is None else _to_eur(s, rate[0], rate[1])
    if eur is None or eur <= 0 or eur > CAP:
        return None, "R6: no EUR or over EUR 100 bn"
    return eur, f"derived (R-shape): {kind} lot sum {s} {cur} over {n} lots, no procedure figure"


def rule_B(rec, detail=None, variant="core"):
    """-> (eur_cents | None, reason, derived)."""
    cur = rec.get("elected_eur_cents")
    c = core(rec)
    if c is not None and c[0] is not None:
        return c[0], c[1], True
    if c is not None:
        return cur, "keep: refusals matched conflicting lot sums", False
    if variant == "r_split":
        eur, why = r_shape2(rec, detail)
        if (eur is not None and rec.get("elected_scope") == "award-copy" and why.startswith("derived (R-shape): awards ")
                and detail["award_rounds"].get(rec["elected"]["currency"], 0) >= 2):
            return eur, why.replace("R-shape", "R-split: lots awarded in >= 2 award notices"), True
        return cur, "keep: not the split-award shape", False
    if variant == "r_shape2":
        eur, why = r_shape2(rec, detail)
        if eur is not None:
            return eur, why, True
        return cur, "keep: " + why, False
    if variant in ("r_shape", "r_shape_sentinel"):
        eur, why = r_shape(rec, detail, allow_sentinel_proc=(variant == "r_shape_sentinel"))
        if eur is not None:
            return eur, why, True
        return cur, "keep: " + why, False
    return cur, "keep: max election", False


# ---------------------------------------------------------------- per-lot detail from the raw pull
_DETAILS = None


def details():
    """{tid: {"per_lot": {cur: {kind: {lot: cents}}} (electable kinds, lot_sum's rules, plus partner-only
    fields under their own names), "lot_null": [{field, cents, currency, copy_of: [lot keys], lotless}],
    "rate": {cur: (cents, eur_cents)} (one head row per currency, for a null head's conversion)}}"""
    global _DETAILS
    if _DETAILS is not None:
        return _DETAILS
    import an
    import load
    D = load.parse(an.NAMES)
    out = {}
    for tid, t in D.items():
        seq, _ = t["T"]
        if seq is None:
            continue
        head, chain, keys = load.inputs(t, tid)
        v = E.version_from_rows(head["amounts"], head["lots"], head["results"], keys)
        ls = E.lot_sum_version(v)
        per_lot = {cur: {k: dict(x["per_lot"]) for k, x in kinds.items() if k != "per_lot_max_any_field"}
                   for cur, kinds in ls.items()}
        unit = "Lot" if any(l.kind == "Lot" for l in v.lots) else "Part"
        for lot in v.lots:
            if lot.kind != unit:
                continue
            for f in lot.facts:
                if E.electable_amount_field(f.field) or f.quality is not None or not E._lot_sum_figure(f.cents):
                    continue
                d = per_lot.setdefault(f.currency, {}).setdefault(f.field, {})
                d[lot.key] = max(d.get(lot.key, 0), f.cents)
        award_lots, lotless = {}, set()
        for rnd in v.rounds:
            for r in rnd.lot_results:
                if r.currency is None or r.cents is None:
                    continue
                if r.lot_key is None:
                    lotless.add((r.currency, r.cents))
                else:
                    award_lots.setdefault((r.currency, r.cents), set()).add(r.lot_key)
        lot_null = [{"field": f.field, "cents": f.cents, "currency": f.currency,
                     "copy_of": sorted(map(str, award_lots.get((f.currency, f.cents), ()))),
                     "lotless": (f.currency, f.cents) in lotless} for f in v.facts]
        rate = {}
        for a in head["amounts"]:
            if a["eur_cents"] and a["cents"] > 0:
                rate.setdefault(a["currency"], (a["cents"], a["eur_cents"]))
        rounds = {}
        latest = {}
        for rnd in v.rounds:
            for r in rnd.lot_results:
                if r.lot_key is None or r.currency is None or r.cents is None or r.lot_key in {
                        l.key for l in v.lots if l.kind != unit} or not E._lot_sum_figure(r.cents):
                    continue
                k = (r.currency, r.lot_key)
                if k not in latest or rnd.notice_id >= latest[k]:
                    latest[k] = rnd.notice_id
        for (cur, _), nid in latest.items():
            rounds.setdefault(cur, set()).add(nid)
        out[tid] = {"per_lot": per_lot, "lot_null": lot_null, "rate": rate,
                    "award_rounds": {c: len(x) for c, x in rounds.items()}}
    _DETAILS = out
    return out
