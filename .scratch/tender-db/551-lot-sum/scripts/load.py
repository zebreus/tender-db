"""Turn the pulled /v1/sql bodies into per-tender election inputs.

Two shapes:
  full     (b_NN / r_NN): T row, every version's amounts and results, the HEAD version's lots.
  compact  (cm_*):        T row, the HEAD version's amounts / lots / results, and the chain reduced to
                          CA = DISTINCT (currency, cents, is_procedure) and CR = DISTINCT (currency, cents).
Both become: head = {amounts, lots, results}, chain = [ {amounts, lots, results}, ... ], lot_keys.
"""
import json, os, sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
FLOOR = 1000


def body(name):
    d = json.load(open(os.path.join(HERE, "data", name + ".json")))
    assert "rows" in d and "columns" in d, name
    return d


def _amount(r):
    # k, tid, seq, lot_id, n1=cents, s1=field, s2=currency, s3=tax_basis, n2=eur_cents, n3=quality
    return {"tender_id": r[1], "seq": r[2], "lot_id": r[3], "field": r[5], "cents": r[4], "currency": r[6],
            "tax_basis": r[7], "eur_cents": r[8], "quality": r[9]}


def _result(r):
    # k, tid, seq, lot_id, n1=awarded_cents, s1=awarded_currency, s2=result_key, s3, n2=lot_result_id, n3=notice_id
    return {"tender_id": r[1], "seq": r[2], "lot_id": r[3], "awarded_cents": r[4], "awarded_currency": r[5],
            "result_key": r[6], "lot_result_id": r[8], "notice_id": r[9]}


def _lot(r):
    return {"tender_id": r[1], "seq": r[2], "lot_id": r[3], "kind": r[5], "lot_key": r[6]}


def parse(names):
    """-> {tid: {"T": (seq, stored), "A": [...], "L": [...], "R": [...], "CA": set, "CR": set, "src": name}}"""
    out = {}
    for name in names:
        d = body(name)
        assert not d.get("truncated"), name
        for r in d["rows"]:
            src = "cm_big" if name.startswith("cm_big") else name   # cm_big1 + cm_big2 are one split query
            t = out.setdefault(r[1], {"A": [], "L": [], "R": [], "CA": None, "CR": None, "src": src})
            if t["src"] != src:
                continue          # a later source must replace, not merge (see merge())
            k = r[0]
            if k == "T":
                t["T"] = (r[2], r[4])
            elif k == "A":
                t["A"].append(_amount(r))
            elif k == "L":
                t["L"].append(_lot(r))
            elif k == "R":
                t["R"].append(_result(r))
            elif k == "CA":
                t["CA"] = t["CA"] or set()
                t["CA"].add((r[6], r[4], int(r[8])))
            elif k == "CR":
                t["CR"] = t["CR"] or set()
                t["CR"].add((r[5], r[4]))
    for t in out.values():
        t["compact"] = t["src"].startswith("cm_")
        if t["compact"]:
            t["CA"] = t["CA"] or set()
            t["CR"] = t["CR"] or set()
    return out


def compact_of_full(t):
    """The CA/CR the compact query would return, computed from full-shape rows (for the self-check)."""
    awards_by_seq = defaultdict(set)
    for r in t["R"]:
        if r["awarded_cents"] is not None and r["awarded_currency"] is not None:
            awards_by_seq[r["seq"]].add((r["awarded_currency"], r["awarded_cents"]))
    ca = set()
    for a in t["A"]:
        if a["cents"] > FLOOR:
            isp = int(a["lot_id"] is None and (a["currency"], a["cents"]) not in awards_by_seq[a["seq"]])
            ca.add((a["currency"], a["cents"], isp))
    cr = {(r["awarded_currency"], r["awarded_cents"]) for r in t["R"]
          if r["awarded_cents"] is not None and r["awarded_cents"] > FLOOR and r["awarded_currency"] is not None}
    return ca, cr


def synthetic_chain(tid, ca, cr):
    """Two versions reproducing ScalePartners' chain reads (figures, lot_only) exactly:
    P = every procedure-scope figure at tender scope, no awards; L = every lot-scope figure (lot facts and
    award copies) on one Lot, plus every award on that Lot."""
    p_rows = [{"tender_id": tid, "seq": -2, "lot_id": None, "field": "chain", "cents": c, "currency": cur,
               "tax_basis": None, "eur_cents": None, "quality": None} for cur, c, isp in sorted(ca) if isp]
    l_rows = [{"tender_id": tid, "seq": -1, "lot_id": -1, "field": "chain", "cents": c, "currency": cur,
               "tax_basis": None, "eur_cents": None, "quality": None} for cur, c, isp in sorted(ca) if not isp]
    res = [{"lot_result_id": i, "lot_id": -1, "awarded_cents": c, "awarded_currency": cur, "notice_id": 0,
            "result_key": f"CHAIN-{i}"} for i, (cur, c) in enumerate(sorted(cr))]
    return [{"amounts": p_rows, "lots": [], "results": []},
            {"amounts": l_rows, "lots": [{"lot_id": -1, "kind": "Lot"}], "results": res}]


def inputs(t, tid, mode=None):
    """-> (head, chain, lot_keys). mode: None = as pulled; 'compact' = force the synthetic chain (self-check)."""
    seq, _ = t["T"]
    lot_keys = {l["lot_id"]: l["lot_key"] for l in t["L"]}
    head_lots = [{"lot_id": l["lot_id"], "kind": l["kind"]} for l in t["L"]]
    head = {"amounts": [a for a in t["A"] if a["seq"] == seq], "lots": head_lots,
            "results": [r for r in t["R"] if r["seq"] == seq]}
    if t["compact"]:
        chain = synthetic_chain(tid, t["CA"], t["CR"])
    elif mode == "compact":
        ca, cr = compact_of_full(t)
        chain = synthetic_chain(tid, ca, cr)
    else:
        chain = []
        seqs = sorted({a["seq"] for a in t["A"]} | {r["seq"] for r in t["R"]})
        for s in seqs:
            if s >= seq:
                continue
            am = [a for a in t["A"] if a["seq"] == s]
            lots = [{"lot_id": i, "kind": "Lot"} for i in sorted({a["lot_id"] for a in am if a["lot_id"] is not None})]
            chain.append({"amounts": am, "lots": lots, "results": [r for r in t["R"] if r["seq"] == s]})
    return head, chain, lot_keys
