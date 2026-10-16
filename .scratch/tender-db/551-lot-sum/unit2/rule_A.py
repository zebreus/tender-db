"""551 unit 2, design A: a GUARDED LOT-SUM fallback for head_value_eur_cents_with.

    serve(rec, entry=None) -> (eur_cents | None, reason: str, detail: dict)

`rec` is one results.jsonl record.  The record holds sums and counts, not the per-lot figures the guards
need, so the rule reads the Tender's head + chain rows from u2/A/ctx.pkl (rebuilt from u1's /v1/sql bodies
with u1's own loader, the same inputs that wrote results.jsonl).  `entry` = that ctx entry; None = look it
up by rec["id"].  The 4,745 records are all present in ctx.pkl.

THE RULE (plain words)
----------------------
T  Trigger.  The election admitted NO procedure-scope figure (every one is absent, refused or
   not-electable).  Award copies (Lot(None), 492 (c)) are lot figures here, not procedure figures.
   Otherwise: today's head, unchanged.

A  537-vouched sum.  If a procedure figure F was refused by 537's lot-sum rule (F ~ 10^k x a lot sum S),
   serve the electable lot sum that matches F, tried in the order: 537's matched kind if electable,
   awards, BT-27-Lot (estimated_value), BT-271-Lot (framework_maximum) -- the first whose sum S' passes
   lot_sum_exponent(F, S', n').  A partner-only match (BT-709/BT-660, 506) is never served itself; it only
   points at the electable kind with the same scale (1024214: BT-709 matched, BT-271 served).
   Only guard: the EUR 100 bn cap.  The buyer's own total, slipped, IS the sum, so no repeat guard runs
   (7427765: six equal EUR 500 k awards whose copy is lot-null; the repeat guard would refuse it).

B  Same-kind sum.  Otherwise sum the lots in the KIND OF TODAY'S ELECTED FIGURE (an award copy -> awards;
   a head-lot estimated_value -> estimated_value; framework_maximum -> framework_maximum; ...), in its
   currency.  Today null -> the first of awards, estimated_value, framework_maximum carried by >= 2 lots.
   One figure per lot as elect.lot_sum picks it (latest award; largest figure per field; partner-only,
   withheld, sentinel and <= 10.00 figures out; Lots only, Parts only in a Lot-less head, never a group).
   Serve it unless a guard fires:
   G0  fewer than 2 lots carry the kind, or the sum does not exceed today's figure (equal: no change;
       smaller: the kinds disagree, e.g. 6885102's call-off award under a GBP 1.5 bn estimate copy).
   G1  REPEATED TOTAL, confirmed: a figure carried by >= 2 summed lots is also printed lot-null (any field)
       or as a procedure figure in any version of the chain, and it is at least 1/10 of the largest summed
       lot figure (so a coincidental pair of small equal awards, copied lot-null by TED, does not fire:
       5995030).  [8440393, 7955558, G-Cloud 8596072, 8374102, 8422921, ...]
   G2  REPEATED TOTAL, unconfirmed: one figure is carried by >= 4 summed lots and by at least half of
       them.  [465993/786738 13 x EUR 4 bn, 543944, 274271, the Colmar BT-271 frameworks]
   G3  ONE LOT IS A TOTAL: with >= 3 lots, one lot's figure is within 1 % of 10^k x the sum of the others
       (k = 0..3).  [5344896, 5316712 (0.9 %), 4787274 (k = 2)]
   G4  SCALE DISAGREEMENT: another electable kind on >= 2 of the same lots, same currency, sums to less
       than 1/10 or more than 10 x this sum.  [175096: BT-27-Lot EUR 3.17 bn per lot under a EUR 124 m
       BT-271 framework maximum]
   G5  CAP: the sum converts to more than EUR 100 bn, or any procedure / lot-null figure of the head is
       not-electable as over EUR 100 bn (366): the buyer's scale is broken.  [8811130, 7991207, 5781391]
   G6  CURRENCY: the kind also carries lot figures in another currency (the sum would be partial).
   G7  WITHHELD: a procedure figure of the head is withheld (372); a lot sum would publish it.

   The served value is the lot sum in its own currency, converted with the head's own rate for that
   currency (eur_cents / cents of a head row in it; Rust would call RatesLookup at the same date).

NOT IN SCOPE: the adjacent class, where an admitted (or award-copy) total is beaten by one lot's x10 slip
(5855551, 8422921, 8488015, 631213, 1136952, 1140133, 1142891; and 114991, 867491 with an admitted
procedure figure).  T does not fire when a procedure figure is admitted, and inside T G1/G2 refuse those
sums, so the rule leaves every one of them at today's slipped figure: no better, no worse.
"""
from __future__ import annotations

import os
import sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
M551 = os.path.dirname(HERE)
sys.path.insert(0, M551)
sys.path.insert(0, os.path.join(HERE, "A"))
import elect as E  # noqa: E402

KIND_ORDER_NULL = ("awards", "estimated_value", "framework_maximum")
FIELD_OF_537 = {E.AWARDS: "awards", E.ESTIMATE: "estimated_value", E.FWMAX: "framework_maximum"}
G1_MIN_SHARE_OF_MAX = 10        # repeated figure x 10 >= largest lot figure
G2_MIN_LOTS = 4                 # unconfirmed repetition: >= 4 lots share one figure ...
G2_MIN_FRACTION = 0.5           # ... and at least half of the summed lots
G3_TOL = 0.01                   # one lot within 1 % of 10^k x the others
G3_MAX_K = 3
G4_RATIO = 10
# ablation switches (defaults = the rule): guards to skip, and the B kind policy
DISABLE = set()                 # e.g. {"A", "G1", "G2", "G3", "G4", "G5", "G6", "G7", "G0-below"}
KIND_POLICY = "elected"         # "elected" | "awards_first" (awards when >= 2 lots carry them, in the elected currency)

_CTX = None


def _ctx():
    global _CTX
    if _CTX is None:
        import ctx
        _CTX = ctx.get()
    return _CTX


# ------------------------------------------------------------------ helpers
def rate(entry, cur):
    """EUR cents per native cent for `cur`, from the head's own rows (same date as the election);
    falls back to the chain's rows.  None when no row in `cur` carries a conversion."""
    if cur == "EUR":
        return 1.0
    pools = [entry["head"]["amounts"]] + ([] if entry["compact"] else [v["amounts"] for v in entry["chain"]])
    for pool in pools:
        rs = sorted(a["eur_cents"] / a["cents"] for a in pool
                    if a["currency"] == cur and a["eur_cents"] and a["cents"] and a["cents"] > 0)
        if rs:
            return rs[len(rs) // 2]
    return None


def to_eur(entry, cur, cents):
    r = rate(entry, cur)
    return None if r is None else int(round(cents * r))


def lotnull_figures(entry, cur):
    """Every lot-null amount (any field, any status) and every procedure figure in `cur`, in any version."""
    s = {a["cents"] for a in entry["head"]["amounts"] if a["lot_id"] is None and a["currency"] == cur}
    if entry["compact"]:
        # compact chains keep (currency, cents, is_procedure); is_procedure = lot-null and not an award copy.
        # Lot-null award copies of earlier versions are not kept, so G1 can only miss there, never over-fire.
        s |= {c for (cu, c, isp) in entry["CA"] if cu == cur and isp}
    else:
        for v in entry["chain"]:
            s |= {a["cents"] for a in v["amounts"] if a["lot_id"] is None and a["currency"] == cur}
    return s


def kind_of_elected(rec):
    el = rec.get("elected")
    if el is None:
        return None
    if rec.get("elected_scope") == "award-copy":
        return "awards"
    return el["field"]


# ------------------------------------------------------------------ the rule
def serve(rec, entry=None):
    tid = rec["id"]
    today = rec["elected_eur_cents"]
    entry = entry if entry is not None else _ctx()[tid]
    head = entry["head"]
    reasons = rec["reasons"]
    proc = [x for x in reasons if x["scope"] == "procedure" and x["status"] != "partner-only"]
    det = {"today": today}

    # T -------------------------------------------------------------
    if any(x["status"] in ("elected", "admitted") for x in proc):
        return today, "T: a procedure figure is admitted (unchanged)", det

    ls = E.lot_sum(head["amounts"], head["lots"], head["results"], None, entry["keys"])

    def sum_of(cur, kind):
        v = ls.get(cur, {}).get(kind)
        return None if v is None else v

    # A: 537-vouched -------------------------------------------------
    r537 = [x for x in proc if x["status"] == "refused" and (x["detail"] or "").startswith("537 lot sum")]
    if r537 and "A" not in DISABLE:
        versions = [E.version_from_rows(v.get("amounts"), v.get("lots"), v.get("results"), entry["keys"])
                    for v in entry["chain"]]
        versions.append(E.version_from_rows(head["amounts"], head["lots"], head["results"], entry["keys"]))
        partners = E.ScalePartners.of_chain(versions)
        for x in sorted(r537, key=lambda x: -x["eur_cents"]):
            m = partners.lot_sum_match(x["currency"], x["cents"])
            if m is None:
                continue
            order = ([FIELD_OF_537[m.kind]] if m.kind in FIELD_OF_537 else []) + list(KIND_ORDER_NULL)
            for kind in dict.fromkeys(order):
                v = sum_of(x["currency"], kind)
                if v is None or v["lots"] < 2:
                    continue
                if E.lot_sum_exponent(x["cents"], v["sum"], v["lots"]) is None:
                    continue
                eur = to_eur(entry, x["currency"], v["sum"])
                det.update(path="A", kind=kind, currency=x["currency"], sum=v["sum"], lots=v["lots"],
                           matched=E.KIND_LABEL[m.kind], k=m.exponent)
                if eur is None:
                    return today, "A: no EUR rate for the sum (unchanged)", det
                if eur > E.IMPLAUSIBLE_EUR_CENTS:
                    return today, "A/G5: sum over EUR 100 bn (unchanged)", det
                return eur, (f"A: 537 refused F={x['cents']} {x['currency']} ~ 10^{m.exponent} x "
                             f"{E.KIND_LABEL[m.kind]} sum; served the {kind} sum over {v['lots']} lots"), det
        det["path"] = "A-miss->B"   # a 537 refusal without an electable same-scale sum: fall through to B

    # B: same-kind sum ----------------------------------------------
    kind = kind_of_elected(rec)
    if kind is not None:
        cur = rec["elected"]["currency"]
        v = sum_of(cur, kind)
        if KIND_POLICY == "awards_first":
            va = sum_of(cur, "awards")
            if va is not None and va["lots"] >= 2:
                kind, v = "awards", va
    else:
        cur, v = None, None
        # today null: awards first, then estimates, then framework maxima, in the currency with most lots
        for k in KIND_ORDER_NULL:
            cands = [(c, kinds[k]) for c, kinds in ls.items() if k in kinds and kinds[k]["lots"] >= 2]
            if cands:
                cur, v = max(cands, key=lambda cv: (cv[1]["lots"], cv[0]))
                kind = k
                break
    det.update(path=det.get("path", "B"), kind=kind, currency=cur)
    if v is None or v["lots"] < 2:
        return today, "G0: fewer than 2 lots carry the elected kind (unchanged)", det
    S, n, per = v["sum"], v["lots"], v["per_lot"]
    det.update(sum=S, lots=n)
    eur = to_eur(entry, cur, S)
    det["sum_eur"] = eur
    if eur is None:
        return today, "G0: no EUR rate for the sum (unchanged)", det
    if today is not None and (eur == today or (eur < today and "G0-below" not in DISABLE)):
        return today, ("G0: the sum equals today's figure (unchanged)" if eur == today
                       else "G0: the sum is below today's figure; the kinds disagree (unchanged)"), det

    # G7 withheld procedure figure
    if "G7" not in DISABLE and any(x["status"] == "not-electable" and (x["detail"] or "").startswith("372") for x in proc):
        return today, "G7: a procedure figure is withheld (unchanged)", det
    # G5 cap
    over = [x for x in reasons if x["status"] == "not-electable" and (x["detail"] or "").startswith("366: over")
            and (x["scope"] == "procedure" or x["scope"].startswith("award-copy"))]
    if over and "G5" not in DISABLE:
        return today, "G5: a procedure/lot-null figure is over EUR 100 bn; the buyer's scale is broken (unchanged)", det
    if eur > E.IMPLAUSIBLE_EUR_CENTS and "G5" not in DISABLE:
        return today, "G5: the sum is over EUR 100 bn (unchanged)", det
    # G6 currency
    other_cur = [c for c, kinds in ls.items() if c != cur and kind in kinds]
    if other_cur and "G6" not in DISABLE:
        return today, f"G6: the {kind} lots also carry {other_cur} figures (unchanged)", det
    vals = list(per.values())
    cnt = Counter(vals)
    mx = max(vals)
    # G1 confirmed repetition
    ln = lotnull_figures(entry, cur)
    rep = sorted((f for f, c in cnt.items() if c >= 2 and f in ln and f * G1_MIN_SHARE_OF_MAX >= mx), reverse=True)
    if rep and "G1" not in DISABLE:
        det["repeated"] = rep[0]
        return today, (f"G1: {cnt[rep[0]]} of {n} lots carry {rep[0]}, also a lot-null/procedure figure in the "
                       f"chain: a repeated total (unchanged)"), det
    # G2 unconfirmed repetition
    M, c = max(cnt.items(), key=lambda fc: (fc[1], fc[0]))
    if c >= G2_MIN_LOTS and c >= G2_MIN_FRACTION * n and "G2" not in DISABLE:
        return today, f"G2: {c} of {n} lots carry one figure {M}: a repeated total (unchanged)", det
    # G3 one lot is a total of the others
    if n >= 3 and "G3" not in DISABLE:
        f = mx                      # a lot that is a total of the others is the largest lot
        rest = S - f
        for k in range(0, G3_MAX_K + 1):
            t = rest * 10**k
            if t > 0 and abs(f - t) <= G3_TOL * f:
                return today, f"G3: one lot ({f}) ~ 10^{k} x the sum of the others ({rest}) (unchanged)", det
    # G4 scale disagreement with another electable kind over the same lots
    for k2, v2 in (ls.get(cur, {}).items() if "G4" not in DISABLE else ()):
        if k2 in (kind, "per_lot_max_any_field"):
            continue
        common = set(v2["per_lot"]) & set(per)
        if len(common) < 2:
            continue
        a = sum(per[l] for l in common)
        b = sum(v2["per_lot"][l] for l in common)
        if a > G4_RATIO * b or b > G4_RATIO * a:
            return today, f"G4: {kind} and {k2} disagree by more than 10x over {len(common)} lots (unchanged)", det
    return eur, f"B: served the {kind} sum over {n} lots ({S} {cur})", det


if __name__ == "__main__":
    import json
    ids = {int(a) for a in sys.argv[1:]}
    for line in open(os.path.join(M551, "results.jsonl")):
        r = json.loads(line)
        if not ids or r["id"] in ids:
            v, why, d = serve(r)
            print(r["id"], r["elected_eur_cents"], "->", v, "|", why)
