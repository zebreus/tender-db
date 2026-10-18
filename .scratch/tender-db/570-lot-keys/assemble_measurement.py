"""Issue 570: assemble measurement.json from the judge's skeptic-final verdicts and the option-B port replay.

The judge's per-tender table (workflow measurer + skeptic + judge, 2026-10-17/18) is carried here as data: the class
of every tender follows from the replay's count of lot-publishing notices except the 14 the judge classed
disagree_* / unnumbered, listed below with the judge's counts and a condensed reason. The derived class totals must
equal the judge's (single_lot_notice 47, consistent 143, disagree_harmful 7, disagree_harmless 6, unnumbered 1) and
the served result total 1,596; the script asserts both.

Usage: python3 -I assemble_measurement.py port_result.json groups.json measurement.json
"""
import json
import math
import random
import sys
from collections import Counter

JUDGE = {
    # tender: (class, results misattributed, lot facts misattributed, B fixes it?, note)
    7274985: ("disagree_harmful", 1, 2, "fixed",
              "CN 22137062 LOT-1..4 = S1..S4; CAN 22223285 lists 3 lots by position, LOT-3 = S4 (its RES-3 estimate "
              "equals the CN's S4 value). RES-3 (S4, Kusiak) serves on LOT-3 = the CN's S3; LOT-3 serves S4's title and "
              "description (S3's text appears nowhere); LOT-4 = S4 duplicated, no award."),
    7511890: ("disagree_harmful", 4, 9, "fixed",
              "CN 22690061 Pakiet k at LOT-k (1..6); award 22907340 LOT-1/2/3 = 1/2/5; award 22980479 LOT-1/2/3 = 3/4/6 "
              "(titles and descriptions follow the printed number). Served LOT-1..3 titled Pakiet 3/4/6, Pakiet 1-3 "
              "missing; 4 awards on another lot's key; 9 facts (titles, descriptions, 2 winner parties, a CPV)."),
    7697322: ("disagree_harmful", 1, 0, "fixed",
              "CN 23121547 LOT-k = 0k (17); CAN 23276289's single LOT-1 = 02 (terrassement); CAN 23413021 omits 02, 07, "
              "09 and shifts LOT-2..14. 23276289 RES-1 (02, Mounard TP) serves on LOT-1 'v.r.d' (lot 01); lots 02/07/09 "
              "missing, 15-17 served twice; LOT-2..14 changed identity v1 -> v3."),
    5787834: ("disagree_harmful", 1, 2, "fixed",
              "CN 19020649 LOT-k = k (19); CAN 19154619's single LOT-1 = 18; CAN 19167246 LOT-18 = 19 (lot 18 omitted). "
              "19154619 RES-1 (18, clos-nw) serves on LOT-1 (lot 1); LOT-18 serves lot 19's title (2 facts)."),
    5827131: ("disagree_harmful", 0, 1, "fixed",
              "CN 19103274 LOT-1..3 = 1..3; CAN 19259840 LOT-2 = 3 (digitale). Award right by served identity; lot 2 "
              "vanished, lot 3 served twice; 1 fact (duration_end on LOT-2 from the CN's lot 2)."),
    5317587: ("disagree_harmful", 0, 7, "not-570",
              "Weld (issue 364): a 2014 procedure and its 2019 re-tender share numbers 1..7 with different lots. Numbers "
              "agree at every key, so printed-number keying changes nothing; 7 lot estimates of the 2014 CN sit on 2019 lots."),
    7465548: ("disagree_harmful", 2, 0, "not-fixable",
              "Framework whose call-off award notices each restart at an unnumbered LOT-1 (3 of 4 lot-publishing notices "
              "print no LOT_NO). The framework award and the Simor call-off serve on LOT-1 'Bazilika'. No number to key by."),
    5540787: ("disagree_harmless", 0, 0, "not-570",
              "Weld (issue 364): 2015 lots 1/2 and 2020 lots 1..5 share numbers; served facts all come from 2020; no 2015 award."),
    6980424: ("disagree_harmless", 0, 0, "realigned",
              "CN 21462872 and CAN 21769762 both permute 1..10 by ITEM order, differently (7 keys differ). Served lots "
              "carry the CAN's titles; results right by served identity; lot identity changes v1 -> v2."),
    7112657: ("disagree_harmless", 0, 0, "unchanged",
              "A second F02 (NO_LOT_DIVISION) republishes lot 3 as one unnumbered section keyed LOT-1. No served harm; "
              "unnumbered, so printed-number keying cannot see it."),
    6633011: ("disagree_harmless", 0, 0, "realigned",
              "CN 20668864 LOT-k = k (9); CAN 20957186 keys its 6 awarded lots by position (LOT-3..6 = 4,5,6,8). Results "
              "right by served identity; lot 3 vanished, lot 8 served twice; LOT-3..6 renamed v1 -> v2."),
    8433088: ("disagree_harmless", 0, 0, "refused",
              "CN 20329681 one section LOT-1 = '1-4' (all four groups); CAN 20739611 LOT-1/2/3 = '1' / '2-3' / '4'. A "
              "re-lotting, not a swap; served CAN titles, results right. B refuses (no carried lot prints '1')."),
    7693726: ("disagree_harmless", 0, 0, "realigned",
              "CN 23126006 LOT-k = k (16); CAN 23256169 keys 12 awarded lots by position (LOT-7..12 = 8,11,13..16). "
              "Results right by served title; lots 7/9/10/12 vanished, 13-16 served twice."),
    5939039: ("unnumbered", 0, 0, "unchanged",
              "CAN 19505376 (NO_LOT_DIVISION) has one unnumbered, untitled section LOT-1 = the whole procedure object; "
              "2 procedure-scope facts on lot 1, not another lot's."),
}

JUDGE_TOTALS = {"single_lot_notice": 47, "consistent": 143, "disagree_harmful": 7, "disagree_harmless": 6,
                "unnumbered": 1}


def binom_cdf(k, n, p):
    return sum(math.comb(n, i) * p ** i * (1 - p) ** (n - i) for i in range(k + 1))


def clopper_pearson(k, n, alpha=0.05):
    def bisect(pred):
        lo, hi = 0.0, 1.0
        for _ in range(100):
            mid = (lo + hi) / 2
            if pred(mid):
                hi = mid
            else:
                lo = mid
        return (lo + hi) / 2
    lower = 0.0 if k == 0 else bisect(lambda p: 1 - binom_cdf(k - 1, n, p) >= alpha / 2)
    upper = 1.0 if k == n else bisect(lambda p: binom_cdf(k, n, p) <= alpha / 2)
    return round(lower, 4), round(upper, 4)


def rate(k, n):
    lo, hi = clopper_pearson(k, n)
    return {"k": k, "n": n, "rate": round(k / n, 4), "ci95": [lo, hi]}


def bootstrap_mean(values, draws=20000, seed=570):
    rnd = random.Random(seed)
    n = len(values)
    means = sorted(sum(values[rnd.randrange(n)] for _ in range(n)) / n for _ in range(draws))
    return {"mean": round(sum(values) / n, 4), "ci95": [round(means[int(0.025 * draws)], 4),
                                                       round(means[int(0.975 * draws)], 4)]}


def main():
    port_path, groups_path, out_path = sys.argv[1:4]
    port = json.load(open(port_path))
    groups = json.load(open(groups_path))
    meta = {it["tender_id"]: dict(it, group=g["key"]) for g in groups["groups"] for it in g["items"]}
    rows = []
    totals = Counter()
    for row in port["sample"]:
        tid = row["tender_id"]
        if tid in JUDGE:
            cls, rm, lf, b_effect, note = JUDGE[tid]
        else:
            cls = "single_lot_notice" if row["lot_publishing_notices"] <= 1 else "consistent"
            rm, lf, note = 0, 0, None
            b_effect = "realigned" if row["B_moved_sections"] else "unchanged"
        totals[cls] += 1
        m = meta[tid]
        rows.append({
            "tender_id": tid, "can_notice_id": m["can_notice_id"], "profile": m["profile"], "year": m["year"],
            "group": m["group"], "cls": cls, "notices_with_lots": row["lot_publishing_notices"],
            "results_total": row["results"], "results_misattributed": rm, "lot_facts_misattributed": lf,
            "maps": row["maps"],
            "B": {"effect": b_effect, "notice_outcomes": row["B_notice_outcomes"],
                  "moved_sections": row["B_moved_sections"], "results_moved": row["results_moved_by_B"],
                  "conflated_keys_today": row["conflated_keys_today"], "conflated_keys_B": row["conflated_keys_B"],
                  "keys_minted": row["keys_minted_by_B"], "keys_swept": row["keys_swept_by_B"],
                  "title_check": [{k: t[k] for k in ("notice", "section", "target", "verdict")}
                                  for t in row["B_title_check"]]},
            "A": row["A"],
            "parity_today_vs_served": row["parity_today_vs_served"],
            "note": note,
        })
    assert dict(totals) == JUDGE_TOTALS, (dict(totals), JUDGE_TOTALS)
    n = len(rows)
    results_total = sum(r["results_total"] for r in rows)
    assert results_total == 1596, results_total
    ids = [r["tender_id"] for r in rows]
    by_id = {r["tender_id"]: r for r in rows}
    fixed = [t for t, j in JUDGE.items() if j[3] == "fixed"]
    changed = [r["tender_id"] for r in rows if r["B"]["moved_sections"]]
    period = lambda r: "2016-2019" if int(r["year"]) <= 2019 else "2020-2023"
    stats = port["stats"]
    rates = {
        "disagree (any)": rate(sum(1 for r in rows if r["cls"].startswith("disagree")), n),
        "disagree_harmful": rate(totals["disagree_harmful"], n),
        "harmful fixed by B": rate(len(fixed), n),
        "harmful not fixable by printed numbers (weld 364, unnumbered call-offs)": rate(2, n),
        "tenders B changes": rate(len(changed), n),
        "tenders B refuses (keeps today's keys on a notice)": rate(
            sum(1 for r in rows if any(v.startswith("refused") for v in r["B"]["notice_outcomes"].values())), n),
        "tenders B mints or sweeps a lot key": rate(
            sum(1 for r in rows if r["B"]["keys_minted"] or r["B"]["keys_swept"]), n),
        "tenders option A re-keys": rate(sum(1 for r in rows if r["A"]["sections_rekeyed"]), n),
        "tenders option A mints or sweeps a lot key": rate(
            sum(1 for r in rows if r["A"]["keys_minted"] or r["A"]["keys_swept"]), n),
        "served results misattributed": rate(sum(r["results_misattributed"] for r in rows), results_total),
        "served results fixed by B": rate(sum(by_id[t]["results_misattributed"] for t in fixed), results_total),
        "served results B moves": rate(stats["sample:results_moved_by_B"], results_total),
        "B-moved sections whose title contradicts the target (sample + exhibits)": rate(
            stats.get("sample:B_title_own", 0) + stats.get("exhibits:B_title_own", 0)
            + stats.get("sample:B_title_tie", 0) + stats.get("exhibits:B_title_tie", 0),
            stats["sample:B_sections_moved"] + stats["exhibits:B_sections_moved"]),
    }
    for p in ("2016-2019", "2020-2023"):
        sub = [r for r in rows if period(r) == p]
        rates[f"tenders B changes, {p}"] = rate(sum(1 for r in sub if r["B"]["moved_sections"]), len(sub))
        rates[f"disagree_harmful, {p}"] = rate(sum(1 for r in sub if r["cls"] == "disagree_harmful"), len(sub))
    per_can = {
        "results misattributed per multi-lot CAN": bootstrap_mean([by_id[t]["results_misattributed"] for t in ids]),
        "results fixed by B per multi-lot CAN": bootstrap_mean(
            [by_id[t]["results_misattributed"] if t in fixed else 0 for t in ids]),
        "lot facts misattributed per multi-lot CAN": bootstrap_mean([by_id[t]["lot_facts_misattributed"] for t in ids]),
        "lot facts fixed by B per multi-lot CAN": bootstrap_mean(
            [by_id[t]["lot_facts_misattributed"] if t in fixed else 0 for t in ids]),
        "results B moves per multi-lot CAN": bootstrap_mean([len(by_id[t]["B"]["results_moved"]) for t in ids]),
    }
    exhibits = [{k: e[k] for k in ("tender_id", "chain", "maps", "B_notice_outcomes", "B_moved_sections",
                                   "conflated_keys_today", "conflated_keys_B", "keys_minted_by_B", "keys_swept_by_B",
                                   "parity_today_vs_served", "A")}
                | {"results_moved_by_B": len(e["results_moved_by_B"]),
                   "results_moved_by_B_summary": dict(Counter(f'{m["today"]}->{m["B"]} (prints {m["printed"]})'
                                                              for m in e["results_moved_by_B"])),
                   "title_check": dict(Counter(t["verdict"] for t in e["B_title_check"]))}
                for e in port["exhibits"]]
    out = {
        "issue": 570,
        "assembled": "2026-10-18",
        "sample": {
            "source": groups["source"], "n": n,
            "unit": "one multi-lot legacy CAN per random OJ S day 2016-2023 and its Tender (/v1/tenders/{id}) with every "
                    "version notice's /v1/notices/{id}/content (caches cache_k*/, k*_skeptic/live/)",
            "profiles": dict(Counter(r["profile"] for r in rows)),
            "periods": dict(Counter(period(r) for r in rows)),
            "verdicts": "workflow measurer + adversarial skeptic per group of 17 + judge; skeptic-final classes below",
        },
        "class_totals": dict(totals),
        "rates": rates,
        "per_can": per_can,
        "port": {
            "script": "remap_port.py",
            "result": "port_result.json",
            "parity_today": "today's port equals the served lot on every block the API had placed (1,316 of 1,316); "
                            "the other 280 are R2.0.8 award blocks served lot null at cache time that 568's port places "
                            "(568's drain 2593/2594 was running) - parity is exact outside 568's pending drain",
            "stats": stats,
        },
        "exhibits": exhibits,
        "tenders": rows,
    }
    json.dump(out, open(out_path, "w"), indent=1, ensure_ascii=False)
    print(json.dumps({"class_totals": out["class_totals"], "rates": rates, "per_can": per_can}, indent=1))


if __name__ == "__main__":
    main()
