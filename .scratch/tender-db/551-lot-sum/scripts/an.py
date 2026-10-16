"""551 unit 1: run the election port over B (band + 537) and R (uniform id sample); write results.jsonl."""
import json, os, statistics, sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
M551 = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, M551)
import load
import elect as E

TRUNC = {"b_04", "b_06", "b_08", "b_09", "b_14", "b_18"}
NAMES = ([f"r_{i:02d}" for i in range(1, 6)] + [f"b_{i:02d}" for i in range(1, 26) if f"b_{i:02d}" not in TRUNC]
         + ["cm_mon1", "cm_mon2", "cm_big1", "cm_big2"] + [f"cm_{i:02d}" for i in range(1, 8)])
PRIMARY_ORDER = ["awards", "estimated_value", "framework_maximum", "result_value"]
EXHIBITS = [7427765, 5943121, 369057]


def ids(fn):
    return [int(x) for x in open(os.path.join(HERE, fn)).read().split()]


def rule_of(detail):
    for tag in ("471", "492", "505", "537 lot sum", "537 group copy"):
        if detail and detail.startswith(tag):
            return tag
    return "other"


def primary_kind(kinds):
    """Awards first, then estimates: the first kind in PRIMARY_ORDER summing >= 2 lots, then any other kind
    with the most lots (>= 2); failing all, the first kind present in that order."""
    real = {k: v for k, v in kinds.items() if k != "per_lot_max_any_field"}
    for k in PRIMARY_ORDER:
        if k in real and real[k]["lots"] >= 2:
            return k
    others = sorted((k for k in real if k not in PRIMARY_ORDER and real[k]["lots"] >= 2),
                    key=lambda k: (-real[k]["lots"], k))
    if others:
        return others[0]
    for k in PRIMARY_ORDER:
        if k in real:
            return k
    return min(real, key=lambda k: (-real[k]["lots"], k)) if real else None


def analyse(tid, t):
    seq, stored = t["T"]
    rec = {"id": tid, "stored_eur_cents": stored, "current_seq": seq, "source": t["src"]}
    if seq is None:
        rec["skip"] = "no current_seq"
        return rec
    head, chain, keys = load.inputs(t, tid)
    eur, row, reasons = E.elect(head["amounts"], head["lots"], head["results"], chain=chain, lot_keys=keys)
    kind_of_lot = {l["lot_id"]: l["kind"] for l in head["lots"]}
    rec["elected_eur_cents"] = eur
    rec["fidelity"] = eur == stored
    n_lot = sum(1 for l in head["lots"] if l["kind"] == "Lot")
    rec["lots"] = {"Lot": n_lot, "Part": sum(1 for l in head["lots"] if l["kind"] == "Part"),
                   "LotsGroup": sum(1 for l in head["lots"] if l["kind"] == "LotsGroup")}
    # --- elected figure
    if row is None:
        rec["elected"] = None
        rec["elected_scope"] = "null"
    else:
        r = next(x for x in reasons if x["status"] == "elected")
        lot_kind = kind_of_lot.get(row["lot_id"]) if row["lot_id"] is not None else None
        scope = ("procedure" if r["scope"] == "procedure" else
                 "award-copy" if r["scope"].startswith("award-copy") else f"head-lot:{lot_kind}")
        rec["elected"] = {"field": row["field"], "cents": row["cents"], "currency": row["currency"],
                          "eur_cents": row["eur_cents"], "lot_key": keys.get(row["lot_id"]) if row["lot_id"] else None}
        rec["elected_scope"] = scope
    # --- procedure-scope rows (award copies are scored as lot figures, 492 (c), and are not procedure rows)
    proc = [x for x in reasons if x["scope"] == "procedure" and x["status"] != "partner-only"]
    st = Counter(x["status"] for x in proc)
    if st["elected"] or st["admitted"]:
        pc = "admitted"
    elif st["refused"] and not st["not-electable"]:
        pc = "all-refused"
    elif st["refused"]:
        pc = "refused+not-electable"
    elif st["not-electable"]:
        pc = "not-electable-only"
    else:
        pc = "absent"
    rec["proc_class"] = pc
    rec["proc_refused_rules"] = sorted({rule_of(x["detail"]) for x in proc if x["status"] == "refused"})
    rec["refused_any_rules"] = sorted({rule_of(x["detail"]) for x in reasons if x["status"] == "refused"})
    rec["award_copies"] = Counter(x["status"] for x in reasons if x["scope"].startswith("award-copy"))
    rec["lot_sum_537_info"] = [x["lot_sum_match"] for x in reasons if x.get("lot_sum_match")]
    # --- lot figures (lot_sum's inclusion rules), per currency
    ls = E.lot_sum(head["amounts"], head["lots"], head["results"], None, keys)
    rec["lot_sums"] = {cur: {k: {"sum": v["sum"], "lots": v["lots"]} for k, v in kinds.items()}
                       for cur, kinds in ls.items()}
    def nlots(cur, pred):
        lots = set()
        for k, v in ls[cur].items():
            if k != "per_lot_max_any_field" and pred(k):
                lots |= set(v["per_lot"])
        return len(lots)
    rec["lot_carriers"] = {cur: {"any": nlots(cur, lambda k: True),
                                 "amounts": nlots(cur, lambda k: k != "awards"),
                                 "awards": nlots(cur, lambda k: k == "awards")} for cur in ls}
    rec["multi_any"] = any(v["any"] >= 2 for v in rec["lot_carriers"].values())
    rec["multi_amounts"] = any(v["amounts"] >= 2 for v in rec["lot_carriers"].values())
    rec["multi_awards"] = any(v["awards"] >= 2 for v in rec["lot_carriers"].values())
    rec["c551"] = rec["multi_any"] and pc != "admitted"
    # --- lot sum vs elected, in the elected figure's currency
    if row is not None and row["currency"] in ls:
        kinds = ls[row["currency"]]
        pk = primary_kind(kinds)
        same = "awards" if rec["elected_scope"] == "award-copy" else row["field"]
        c = row["cents"]
        rec["ratio"] = {
            "currency": row["currency"], "primary_kind": pk,
            "primary_sum": kinds[pk]["sum"] if pk else None, "primary_lots": kinds[pk]["lots"] if pk else None,
            "primary": kinds[pk]["sum"] / c if pk else None,
            "same_kind": same, "same_sum": kinds[same]["sum"] if same in kinds else None,
            "same_lots": kinds[same]["lots"] if same in kinds else None,
            "same": kinds[same]["sum"] / c if same in kinds else None,
            "max_any": kinds["per_lot_max_any_field"]["sum"] / c if "per_lot_max_any_field" in kinds else None,
        }
    elif row is not None:
        rec["ratio"] = {"currency": row["currency"], "primary": None, "note": "no lot figure in the elected currency"}
    else:
        # null: the currency with the most lot carriers
        if ls:
            cur = max(ls, key=lambda c: (rec["lot_carriers"][c]["any"], c))
            pk = primary_kind(ls[cur])
            # approximate EUR of the sum from the head's own rows in that currency (same date, same rate)
            rate = [a["eur_cents"] / a["cents"] for a in head["amounts"]
                    if a["currency"] == cur and a["eur_cents"] and a["cents"] > 0]
            s = ls[cur][pk]["sum"] if pk else None
            rec["null_lot_sum"] = {"currency": cur, "kind": pk, "sum": s, "lots": ls[cur][pk]["lots"] if pk else None,
                                   "eur_approx": round(s * statistics.median(rate)) if (s and rate) else None}
    # --- refinements
    adm_proc_eur = {x["eur_cents"] for x in proc if x["status"] in ("elected", "admitted")}
    rec["proc_partial_refused"] = pc == "admitted" and any(x["status"] == "refused" for x in proc)
    rec["lot_head_despite_proc"] = (pc == "admitted" and row is not None and rec["elected_scope"] != "procedure"
                                    and eur not in adm_proc_eur)
    if rec["elected_scope"] == "award-copy":
        rec["award_copy_of"] = sorted({(keys.get(r["lot_id"], f"lot#{r['lot_id']}") if r["lot_id"] is not None else "(no lot)")
                                       for r in head["results"]
                                       if r["awarded_currency"] == row["currency"] and r["awarded_cents"] == row["cents"]})
    cur = (row["currency"] if row is not None else (rec.get("null_lot_sum") or {}).get("currency"))
    if cur in ls:
        rec["uniform"] = {k: (v["lots"] >= 2 and len(set(v["per_lot"].values())) == 1)
                          for k, v in ls[cur].items() if k != "per_lot_max_any_field"}
        rec["equals_a_sum"] = bool(row is not None and any(
            v["sum"] == row["cents"] and v["lots"] >= 2 for v in ls[cur].values()))
    rec["reasons"] = [{k: x[k] for k in ("scope", "field", "cents", "currency", "eur_cents", "status", "detail")}
                      for x in reasons]
    return rec


def main():
    D = load.parse(NAMES)
    B, R = ids("ids_B.txt"), ids("ids_R.txt")
    band = {r[0] for r in load.body("band")["rows"]}
    l48, l22 = [[int(x) for x in l.split(",")] for l in open(os.path.join(HERE, "ids537.txt")).read().split()]
    missing_B = [i for i in B if i not in D]
    out = []
    for pop, lst in (("B", B), ("R", R)):
        for tid in lst:
            if tid not in D:
                continue
            rec = analyse(tid, D[tid])
            rec["pop"] = pop
            if pop == "B":
                rec["in_band"] = tid in band
                rec["r537_refused"] = tid in l22
                rec["r537_matched"] = tid in l48
            out.append(rec)
    with open(os.path.join(M551, "results.jsonl"), "w") as f:
        for rec in out:
            f.write(json.dumps(rec, default=dict) + "\n")
    print("records", len(out), "B missing", missing_B, "R existing", sum(1 for r in out if r["pop"] == "R"))


if __name__ == "__main__":
    main()
