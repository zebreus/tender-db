"""Aggregate results.jsonl into the numbers summary.md quotes (printed as JSON-ish lines + tables)."""
import json, math, os
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
recs = [json.loads(l) for l in open(os.path.join(os.path.dirname(HERE), "results.jsonl"))]


def pct(xs, p):
    xs = sorted(xs)
    if not xs:
        return None
    k = (len(xs) - 1) * p
    f, c = math.floor(k), math.ceil(k)
    return xs[f] if f == c else xs[f] + (xs[c] - xs[f]) * (k - f)


def wilson(k, n, z=1.96):
    if n == 0:
        return (0, 0)
    p = k / n
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return (max(0, c - h), min(1, c + h))


def dist(xs):
    xs = [x for x in xs if x is not None]
    if not xs:
        return "n=0"
    return (f"n={len(xs)} median {pct(xs, .5):.3g} Q1 {pct(xs, .25):.3g} Q3 {pct(xs, .75):.3g} min {min(xs):.3g} max {max(xs):.3g}; "
            f"=1: {sum(1 for x in xs if abs(x - 1) < 1e-12)}; <1: {sum(1 for x in xs if x < 1 - 1e-12)}; "
            f">=1.5: {sum(1 for x in xs if x >= 1.5)} ({sum(1 for x in xs if x >= 1.5) / len(xs):.0%}); "
            f">=3: {sum(1 for x in xs if x >= 3)} ({sum(1 for x in xs if x >= 3) / len(xs):.0%})")


def block(name, rs):
    print(f"\n### {name}: {len(rs)} tenders")
    print("stored null:", sum(r["stored_eur_cents"] is None for r in rs))
    print("fidelity: equal", sum(r["fidelity"] for r in rs), "mismatch", [r["id"] for r in rs if not r["fidelity"]])
    print("elected scope:", dict(Counter(r["elected_scope"] for r in rs)))
    m = [r for r in rs if r["multi_any"]]
    print(f"(1) multi-figure-lot (any kind): {len(m)}; amounts-only def: {sum(r['multi_amounts'] for r in rs)}; awards-only def: {sum(r['multi_awards'] for r in rs)}")
    print("    multi proc_class:", dict(Counter(r["proc_class"] for r in m)))
    c = [r for r in m if r["c551"]]
    print(f"(2) 551 class: {len(c)}  by proc_class {dict(Counter(r['proc_class'] for r in c))}  refused rules {dict(Counter(tuple(r['proc_refused_rules']) for r in c if r['proc_refused_rules']))}")
    ca = [r for r in rs if r["multi_amounts"] and r["proc_class"] != "admitted"]
    print(f"    551 class under the amounts-only definition: {len(ca)}")
    print(f"(3) 551 elected scope: {dict(Counter(r['elected_scope'] for r in c))}")
    acs = Counter(("lotless" if r["award_copy_of"] == ["(no lot)"] else "lot" if "(no lot)" not in r["award_copy_of"] else "lot+lotless")
                  for r in c if r["elected_scope"] == "award-copy")
    print(f"    award-copy matches: {dict(acs)}")
    print(f"    equals a lot sum (head already a total): {[r['id'] for r in c if r.get('equals_a_sum')]}")
    el = [r for r in c if r["elected"] is not None]
    sk = lambda r: (r.get("ratio") or {}).get("same_kind")
    uni = [r["id"] for r in el if (r.get("uniform") or {}).get(sk(r) or "", False)]
    print(f"    same-kind lots all equal (sum = n x one figure): {len(uni)} of {len(el)}")
    print("    ratio same-kind sum / elected:", dist([(r.get("ratio") or {}).get("same") for r in el]))
    print("    ratio awards-first sum / elected:", dist([(r.get("ratio") or {}).get("primary") for r in el]))
    print("    ratio per-lot max any field / elected:", dist([(r.get("ratio") or {}).get("max_any") for r in el]))
    nul = [r for r in c if r["elected"] is None]
    print("    null:", [(r["id"], r.get("null_lot_sum")) for r in nul])
    print("(4) proc_class (whole population):", dict(Counter(r["proc_class"] for r in rs)))
    print("    all-refused by rule:", dict(Counter(tuple(r["proc_refused_rules"]) for r in rs if r["proc_class"] == "all-refused")),
          "ids", sorted(r["id"] for r in rs if r["proc_class"] == "all-refused"))
    print("    all-refused and multi:", sum(1 for r in rs if r["proc_class"] == "all-refused" and r["multi_any"]))
    print("    partial refusal (a procedure row refused, another admitted):", sorted(r["id"] for r in rs if r["proc_partial_refused"]))
    print("adjacent: lot-figure head although a procedure figure is admitted (multi):",
          sum(1 for r in m if r["lot_head_despite_proc"]), "(any lots):", sum(1 for r in rs if r["lot_head_despite_proc"]))
    return len(m), len(c)


B = [r for r in recs if r["pop"] == "B"]
R = [r for r in recs if r["pop"] == "R"]
block("B (band + 537)", B)
block("B in band (stored >= EUR 1 bn)", [r for r in B if r["in_band"]])
block("B 537-refused (left the band)", [r for r in B if r["r537_refused"]])
mR, cR = block("R (uniform id sample)", R)
N = round(len(R) / 1000 * 8848695)
for lab, k in (("multi-figure-lot", mR), ("551 class", cR)):
    lo, hi = wilson(k, len(R))
    print(f"R {lab}: {k}/{len(R)} = {k/len(R):.2%} (95% Wilson {lo:.2%}-{hi:.2%}); corpus ~{N:,} tenders -> ~{k/len(R)*N:,.0f} ({lo*N:,.0f}-{hi*N:,.0f})")
