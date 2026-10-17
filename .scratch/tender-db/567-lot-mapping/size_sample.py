"""Issue 567 drain sizing (usage: python3 -I size_sample.py OUTDIR 18000000 23800000 60 220; seed 567003): a uniform draw of ted-export-r209 notices (random ids in the era's id
range, kept when the profile is ted-export-r209), each read from the public API (/v1/notices/{id}
then /content, >= 1.6 s apart); counts the TED-LOT_NO carriers and the notices whose award mapping
the 567 rule can move. Output: size/sample.jsonl (one row per kept notice)."""
import json, os, random, subprocess, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
OUT = sys.argv[1]
LO, HI, WANT, MAXGET = int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])
rng = random.Random(567003)
last = [0.0]
gets = [0]
def get(path, dest):
    w = 1.6 - (time.time() - last[0])
    if w > 0: time.sleep(w)
    last[0] = time.time(); gets[0] += 1
    r = subprocess.run(["curl", "-sS", "--max-time", "90", "-H", "Accept: application/json", "-A", "tender-db-567-size",
                        "-o", dest, "-w", "%{http_code}", "https://tenders.zebreus.click" + path], capture_output=True, text=True)
    return r.stdout.strip()
kept = 0
seen = set()
with open(os.path.join(OUT, "sample.jsonl"), "a") as out:
    while kept < WANT and gets[0] < MAXGET:
        nid = rng.randrange(LO, HI)
        if nid in seen: continue
        seen.add(nid)
        meta = os.path.join(OUT, f"m{nid}.json")
        code = get(f"/v1/notices/{nid}", meta)
        if code != "200":
            out.write(json.dumps({"id": nid, "meta": code}) + "\n"); continue
        m = json.load(open(meta))
        if m.get("profile") != "ted-export-r209":
            out.write(json.dumps({"id": nid, "profile": m.get("profile"), "published_at": m.get("published_at")}) + "\n"); continue
        cpath = os.path.join(OUT, f"c{nid}.json")
        code = get(f"/v1/notices/{nid}/content", cpath)
        if code != "200":
            out.write(json.dumps({"id": nid, "profile": "ted-export-r209", "content": code}) + "\n"); continue
        c = json.load(open(cpath))
        kinds = {}
        lotno_secs = set()
        for s in c["sections"]:
            kinds[s["kind"]] = kinds.get(s["kind"], 0) + 1
            for v in s["values"]:
                if v["field_id"] == "TED-LOT_NO": lotno_secs.add(s["kind"])
        out.write(json.dumps({"id": nid, "profile": "ted-export-r209", "published_at": m.get("published_at"),
                              "lot_no_in": sorted(lotno_secs), "lots": kinds.get("Lot", 0), "results": kinds.get("LotResult", 0)}) + "\n")
        out.flush()
        kept += 1
print("gets", gets[0], "kept", kept)
