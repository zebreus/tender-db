#!/usr/bin/env python3
"""511 buyer canary: snapshot the buyer notices' Tenders/keys and the key families with
bounded /v1/sql reads (indexed seeks, IN lists of <=100). usage: snap.py plan.json out.json"""
import json, subprocess, sys

def sql(q):
    r = subprocess.run(["ssh", "root@zebreus.click", "/root/sq.sh"], input=q.encode(), capture_output=True, check=True)
    j = json.loads(r.stdout)
    if j.get("truncated"):
        raise SystemExit(f"truncated: {q[:120]}")
    return j["rows"]

def lit(s):
    return "'" + s.replace("'", "''") + "'"

plan = json.load(open(sys.argv[1]))
body = json.loads(plan["body"]) if isinstance(plan["body"], str) else plan["body"]
notices = sorted({b["notice"] for b in body["buyer_notices"]})
keys = sorted({k["key"] for k in body["key_families"]} | {b["procedure_key"] for b in body["buyer_notices"] if b["procedure_key"]})

n2t = {}
for i in range(0, len(notices), 100):
    for n, t in sql("SELECT caused_by_notice_id, tender_id FROM tender_versions WHERE caused_by_notice_id IN (%s)" % ",".join(map(str, notices[i:i+100]))):
        n2t.setdefault(n, []).append(t)
tenders = sorted({t for ts in n2t.values() for t in ts})
t2k = {}
for i in range(0, len(tenders), 100):
    for t, k in sql("SELECT id, procedure_key FROM tenders WHERE id IN (%s)" % ",".join(map(str, tenders[i:i+100]))):
        t2k[t] = k
fam = {}
for i in range(0, len(keys), 20):
    parts = []
    for k in keys[i:i+20]:
        parts.append("SELECT %s AS k, id, procedure_key FROM tenders WHERE procedure_key = %s" % (lit(k), lit(k)))
        parts.append("SELECT %s AS k, id, procedure_key FROM tenders WHERE procedure_key >= %s AND procedure_key < %s" % (lit(k), lit("refused:" + k + ":"), lit("refused:" + k + ";")))
    for k, t, own in sql(" UNION ALL ".join(parts)):
        fam.setdefault(k, {})[str(t)] = {"own": own}
ft = sorted({int(t) for f in fam.values() for t in f})
tv = {}
for i in range(0, len(ft), 100):
    for t, n in sql("SELECT tender_id, caused_by_notice_id FROM tender_versions WHERE tender_id IN (%s)" % ",".join(map(str, ft[i:i+100]))):
        tv.setdefault(t, []).append(n)
for k, f in fam.items():
    for t in f:
        f[t]["notices"] = sorted(tv.get(int(t), []))
json.dump({"buyer_notices": {str(n): {"tenders": sorted(n2t.get(n, [])), "keys": sorted({str(t2k.get(t)) for t in n2t.get(n, [])})} for n in notices},
           "families": fam}, open(sys.argv[2], "w"), indent=1, sort_keys=True)
print("notices", len(notices), "with tender", len(n2t), "tenders", len(tenders), "keys", len(keys), "families", len(fam), "family tenders", len(ft))
