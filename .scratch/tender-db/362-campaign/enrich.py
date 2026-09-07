"""On the box: enrich the R2 denied-names listing with provisional flags and mention
counts (bounded PK/index reads through /root/sqlq.py) and write the cases file.
    python3 enrich.py <denied-listing.json> <cases-out.json>"""
import json, subprocess, sys
L = json.load(open(sys.argv[1]))
ids = sorted({m["org_id"] for g in L for m in g["members"]})
prov, ment = {}, {}
def q(sql):
    out = subprocess.run(["python3", "/root/sqlq.py", sql], capture_output=True, text=True).stdout
    return json.loads(out)["rows"]
for i in range(0, len(ids), 200):
    chunk = ",".join(map(str, ids[i:i+200]))
    for oid, p, c in q(f"SELECT id, provisional, country FROM organizations WHERE id IN ({chunk})"):
        prov[oid] = (p, c)
    for oid, n in q(f"SELECT organization_id, count(*) FROM organization_mentions WHERE organization_id IN ({chunk}) GROUP BY 1"):
        ment[oid] = n
cases = []
for g in L:
    key = f'{g["country"]}/{g["scheme"]}/{g["key"]}'
    members = [{"org": m["org_id"], "name": m["name"], "identifier": m["identifier"], "kind": m["kind"],
                "country": prov.get(m["org_id"], (None, None))[1],
                "provisional": prov.get(m["org_id"], (None, None))[0],
                "mentions": ment.get(m["org_id"], 0)} for m in g["members"]]
    cases.append({"case": key, "country": g["country"], "scheme": g["scheme"], "key": g["key"],
                  "member_ids": [m["org_id"] for m in g["members"]], "members": members})
json.dump(cases, open(sys.argv[2], "w"), ensure_ascii=False)
print("cases", len(cases), "members", len(ids), "missing rows", sum(1 for i in ids if i not in prov))
