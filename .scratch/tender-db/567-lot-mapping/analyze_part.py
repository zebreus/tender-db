import collections, json, math, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import api, mapping

part = int(sys.argv[1])
confirm = "--confirm" in sys.argv
D = api.DIR
recs = [json.loads(l) for l in open(os.path.join(D, f"part{part}_probes.jsonl"))]


def beta_ppf(p, a, b):
    # bisection on the regularized incomplete beta via continued fraction
    def betacf(a, b, x):
        MAXIT, EPS, FPMIN = 200, 3e-14, 1e-300
        qab, qap, qam = a + b, a + 1, a - 1
        c, d = 1.0, 1 - qab * x / qap
        d = 1 / (d if abs(d) > FPMIN else FPMIN); h = d
        for m in range(1, MAXIT + 1):
            m2 = 2 * m
            aa = m * (b - m) * x / ((qam + m2) * (a + m2))
            d = 1 + aa * d; d = 1 / (d if abs(d) > FPMIN else FPMIN)
            c = 1 + aa / c; c = c if abs(c) > FPMIN else FPMIN
            h *= d * c
            aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2))
            d = 1 + aa * d; d = 1 / (d if abs(d) > FPMIN else FPMIN)
            c = 1 + aa / c; c = c if abs(c) > FPMIN else FPMIN
            de = d * c; h *= de
            if abs(de - 1) < EPS:
                break
        return h
    def ibeta(x):
        if x <= 0: return 0.0
        if x >= 1: return 1.0
        bt = math.exp(math.lgamma(a + b) - math.lgamma(a) - math.lgamma(b) + a * math.log(x) + b * math.log(1 - x))
        if x < (a + 1) / (a + b + 2):
            return bt * betacf(a, b, x) / a
        return 1 - bt * betacf(b, a, 1 - x) / b
    lo, hi = 0.0, 1.0
    for _ in range(200):
        mid = (lo + hi) / 2
        if ibeta(mid) < p: lo = mid
        else: hi = mid
    return (lo + hi) / 2


def clopper_pearson(k, n):
    lo = 0.0 if k == 0 else beta_ppf(0.025, k, n - k + 1)
    hi = 1.0 if k == n else beta_ppf(0.975, k + 1, n - k)
    return lo, hi


classes = collections.Counter(r["class"] for r in recs)
days = sorted({r["day"] for r in recs})
hits = [r for r in recs if r["class"] == "multilot-can"]
out_notices = []
cnt = collections.Counter()
for r in hits:
    c = json.load(open(os.path.join(D, "content", f"{r['notice_id']}.json")))
    lex = mapping.analyze(c, "lex")
    doc = mapping.analyze(c, "doc")
    cnt["read"] += 1
    cnt[f"profile:{r['profile']}"] += 1
    nlots = len(lex["lots"])
    if nlots == 0:
        cnt["no_lot_sections"] += 1
    elif nlots == 1:
        cnt["one_lot_section"] += 1
    else:
        cnt["2plus_lot_sections"] += 1
    if any(l["lot_no"] for l in lex["lots"]):
        cnt["lot_sections_carry_lot_no"] += 1
    if lex["item_ne_lotno"]: cnt["item_ne_lotno"] += 1
    if lex["at_risk"]: cnt["at_risk"] += 1
    if lex["award_item_ne_lotno"]: cnt["award_item_ne_lotno"] += 1
    if lex["wrong"]: cnt["misattributed_lex"] += 1
    if doc["wrong"]: cnt["misattributed_doc"] += 1
    st = collections.Counter(a["status"] for a in lex["awards"])
    for k, v in st.items():
        cnt[f"award_status:{k}"] += v
    if any(a["status"] == "unresolved" for a in lex["awards"]): cnt["notices_with_unresolved"] += 1
    if any(a["status"] == "code-guess" for a in lex["awards"]): cnt["notices_with_code_guess"] += 1
    if any(a["status"] == "ok" for a in lex["awards"]): cnt["notices_with_some_ok_link"] += 1
    rec = {
        "notice_id": r["notice_id"], "tender_id": r["tender_id"], "publication_id": r["publication_id"],
        "published": r.get("notice_published"), "profile": r["profile"], "n_award_blocks": r["n_res"],
        "n_lot_sections": nlots, "lots": lex["lots"],
        "awards": [{k: a[k] for k in ("res", "award_item", "award_lot_no", "code_lot", "correct_lot", "correct_how", "status")} for a in lex["awards"]],
        "doc_order_awards_differ": [a["code_lot"] for a in lex["awards"]] != [a["code_lot"] for a in doc["awards"]],
        "item_ne_lotno": lex["item_ne_lotno"], "at_risk": lex["at_risk"],
        "misattributed": bool(lex["wrong"]), "misattributed_doc_order": bool(doc["wrong"]),
    }
    if (lex["wrong"] or doc["wrong"] or lex["at_risk"]) and confirm:
        t = api.get(f"/v1/tenders/{r['tender_id']}", cache=f"tenders/{r['tender_id']}.json")
        api_rows = [lr for lr in t.get("lot_results", []) if lr.get("notice_id") == r["notice_id"]]
        conf = []
        for a in lex["awards"]:
            lr = next((x for x in api_rows if x["key"] == a["res"]), None)
            conf.append({
                "res": a["res"], "award_lot_no": a["award_lot_no"], "code_lot": a["code_lot"], "correct_lot": a["correct_lot"],
                "api_lot": lr.get("lot") if lr else "(no row)",
                "api_winners": [w["organization_name"] for w in (lr or {}).get("winners", [])],
                "api_awarded": (lr or {}).get("awarded"),
            })
        rec["api"] = {"tender_lots": [l["lot_key"] for l in t.get("lot_details", [])],
                      "lot_titles": {l["lot_key"]: l.get("title") for l in t.get("lot_details", [])},
                      "results": conf}
    out_notices.append(rec)

n = cnt["read"]
k = cnt["misattributed_lex"]
lo, hi = clopper_pearson(k, n) if n else (0, 0)
kr = cnt["at_risk"]
rlo, rhi = clopper_pearson(kr, n) if n else (0, 0)
summary = {
    "part": part,
    "draw": {
        "days": days, "n_days": len(days), "probes": len(recs), "probe_classes": dict(classes),
        "method": "per month of the part's years: random OJ S day (Tue-Sat, seeded); /v1/tenders?source=ted&published_after=D&published_before=D+1&limit=1000 (<=2 pages); "
                  "rows whose submission_deadline >= published_at dropped (current version = contract notice); rest shuffled (seeded) and probed via "
                  "/v1/notices?source=ted&publication_id= then /v1/notices/{id}/content until 2 multi-lot CANs (3 in one random month per year; day replaced after 60 probes or <100 rows). "
                  "multi-lot CAN = profile ted-export-r208/r209, TED-TD_DOCUMENT_TYPE 7, >=2 LotResult sections, >=2 distinct award LOT_NO/LOT_NUMBER values.",
    },
    "counts": dict(cnt),
    "rate_misattributed": {"k": k, "n": n, "point": (k / n if n else None), "ci95_clopper_pearson": [lo, hi]},
    "rate_at_risk": {"k": kr, "n": n, "point": (kr / n if n else None), "ci95_clopper_pearson": [rlo, rhi]},
    "misattributed": [x for x in out_notices if x["misattributed"] or x["misattributed_doc_order"]],
    "at_risk_notices": [x["notice_id"] for x in out_notices if x["at_risk"]],
    "notices": out_notices,
}
json.dump(summary, open(os.path.join(D, f"part{part}.json"), "w"), indent=1, ensure_ascii=False)
print(json.dumps({k: summary[k] for k in ("counts", "rate_misattributed", "rate_at_risk")}, indent=1))
print("draw:", len(days), "days", len(recs), "probes", dict(classes))
print("misattributed:", [(x["notice_id"], x["tender_id"]) for x in summary["misattributed"]])
