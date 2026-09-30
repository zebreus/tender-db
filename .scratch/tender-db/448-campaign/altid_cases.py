"""Issue 448 unit 4: join the altid plan listing with Companies House and split the campaign.

usage: altid_cases.py <altid-merge-plan.json> <ch-cache.json> <out-dir> [batch-size]

Every PLAN pair and every judgment-denied pair (witness-only, uncorroborated-*, generic,
legal-form, conflicts) becomes a case with its register entry. A plan pair is
REGISTER-CONFIRMED, and needs no reader, when:
  - every name on the PPON side matches a register name (current or previous) by core, AND
  - the company-number side's names match one too, AND
  - the two orgs never co-occur outside the witnesses.
Everything else is read by a reviewer: plan pairs for a `keep`, denied pairs for a HIGH `merge`.
"""
import json, os, re, sys, collections

FORMS = r"\b(limited|ltd|plc|llp|lp|cic|company|co|the|uk|u k|group|holdings|services)\b"

def core(name, loose=False):
    s = (name or "").lower()
    s = re.split(r"\s+t/a\s+|\(t/a|\s+trading as\s+|\(trading as", s)[0]
    s = s.replace("&", " and ")
    s = re.sub(r"\(\d{6,8}\)", " ", s)
    s = re.sub(r"[^a-z0-9 ]", " ", s)
    s = re.sub(r"\b([a-z]) (?=[a-z]\b)", r"\1", s)
    s = re.sub(r"\b(limited|ltd|plc|llp|lp|cic|company|co|the)\b" if not loose else FORMS, " ", s)
    s = re.sub(r"\band\b", " ", s)
    return " ".join(s.split())

def matches(name, register):
    c, cl = core(name), core(name, loose=True)
    for r in register:
        rc, rcl = core(r), core(r, loose=True)
        if not rc:
            continue
        if c == rc or (cl and cl == rcl):
            return True
        # a trading name that carries the register name, or the reverse
        if len(rcl) >= 4 and (cl.startswith(rcl + " ") or rcl.startswith(cl + " ")) and len(cl) >= 4:
            return True
    return False

def main(plan_path, ch_path, out, size=40):
    d = json.load(open(plan_path))
    b = d.get("body", d)
    if isinstance(b, str):
        b = json.loads(b)
    assert not b["plan_listing_truncated"], "plan listing truncated"
    assert len(b["plan"]) == len(b["pairs"]), (len(b["plan"]), len(b["pairs"]))
    ch = json.load(open(ch_path))
    cases, confirmed, stats = [], [], collections.Counter()
    listings = [("plan", e) for e in b["plan"]] + [("denied", e) for e in b["denied"]] + \
               [("conflict", e) for e in b["conflict_listing"]]
    for kind, e in listings:
        coh = e["key"].split("~")[0]
        reg = ch.get(coh, {})
        register = [n for n in [reg.get("name")] + reg.get("previous", []) if n]
        c = {
            "case": e["key"], "gate": e["gate"], "kind": kind,
            "register": {"number": coh, "http": reg.get("http"), "name": reg.get("name"),
                         "status": reg.get("status"), "previous": reg.get("previous", [])},
            "members": [{"org": m["org_id"], "identifier": m["identifier"], "head": m["name"]} for m in e["members"]],
            "keep": e.get("keep"),
            "coh_names": e.get("coh_names", []), "coh_name_keys": e.get("coh_name_keys"),
            "ppon_names": e.get("ppon_names", []), "ppon_name_keys": e.get("ppon_name_keys"),
            "corroborated_by": e.get("corroborated_by"),
            "witnesses": e["witnesses"], "witness_publications": e["witness_publications"],
            "cooccurring": e.get("cooccurring", 0), "cooccur_publications": e.get("cooccur_publications", []),
            "first_coh": e["first_coh"], "first_ppon": e["first_ppon"],
        }
        if kind == "plan":
            ppon_ok = bool(c["ppon_names"]) and all(matches(n, register) for n in c["ppon_names"])
            coh_ok = bool(c["coh_names"]) and any(matches(n, register) for n in c["coh_names"])
            if register and ppon_ok and coh_ok and c["cooccurring"] == 0 \
                    and c["ppon_name_keys"] == len(c["ppon_names"]):
                confirmed.append(c)
                stats["plan:register-confirmed"] += 1
                continue
            stats["plan:review"] += 1
        elif kind == "denied" and e["gate"] in ("consortium",):
            stats[f"{kind}:{e['gate']}:skipped"] += 1
            continue
        else:
            stats[f"{kind}:{e['gate']}"] += 1
        cases.append(c)
    os.makedirs(out, exist_ok=True)
    json.dump(confirmed, open(f"{out}/confirmed.json", "w"), indent=1)
    json.dump(cases, open(f"{out}/cases.json", "w"), indent=1)
    # Batches: plan cases first (a missed keep merges wrongly), then the denied.
    cases.sort(key=lambda c: (c["kind"] != "plan", c["case"]))
    index = []
    for i in range(0, len(cases), size):
        batch = cases[i:i + size]
        index.append({"id": len(index) + 1, "keys": [c["case"] for c in batch]})
        json.dump(batch, open(f"{out}/batch-{len(index):03d}.json", "w"), indent=1)
    json.dump(index, open(f"{out}/index.json", "w"), indent=1)
    missing = sum(1 for c in cases + confirmed if c["register"]["http"] != "200")
    print(dict(stats), f"| cases {len(cases)} in {len(index)} batches | register missing {missing}")

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]) if len(sys.argv) > 4 else 40)
