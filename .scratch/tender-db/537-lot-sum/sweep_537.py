"""Issue 537 unit 1: fetch every >= EUR 1 bn head's Tender detail (public API, paced ~4/s) and keep what the lot-sum
measurement needs. Incremental: appends one JSON line per tender, resumes past ids already written.
python3 -I sweep_537.py <band.json> <out.jsonl>
"""
import json, os, sys, time, urllib.request, urllib.error
API = "https://tenders.zebreus.click/v1"
band = json.load(open(sys.argv[1]))["rows"]
out = sys.argv[2]
done = set()
if os.path.exists(out):
    for line in open(out):
        try: done.add(json.loads(line)["id"])
        except Exception: pass
def get(path):
    for attempt in range(4):
        try:
            with urllib.request.urlopen(API + path, timeout=60) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            if e.code == 429:
                time.sleep(10); continue
            if e.code in (404, 308):
                return {"_status": e.code}
            raise
        except Exception:
            time.sleep(5)
    return {"_status": "failed"}
with open(out, "a") as f:
    for n, (tid, eur) in enumerate(band):
        if tid in done: continue
        t = get(f"/tenders/{tid}")
        time.sleep(0.25)
        slim = {"id": tid, "head_eur_cents": eur}
        if "_status" in t:
            slim["status"] = t["_status"]
        else:
            slim.update({
                "source": t.get("source"), "kind": t.get("kind"), "procedure_type": t.get("procedure_type"),
                "publication_id": t.get("publication_id"), "notice_subtype": t.get("notice_subtype"),
                "published_at": t.get("published_at"), "value": t.get("value"), "amounts": t.get("amounts"),
                "lots": t.get("lots"), "lot_details": len(t.get("lot_details") or []),
                "lot_results": [{"lot": r.get("lot"), "notice_id": r.get("notice_id"), "awarded": r.get("awarded"),
                                  "decision": r.get("decision")} for r in (t.get("lot_results") or [])],
                "versions": [{"seq": v.get("seq"), "subtype": v.get("notice_subtype"), "pub": v.get("publication_id"),
                              "published_at": v.get("published_at"), "notice": v.get("caused_by_notice_id")}
                             for v in (t.get("versions") or [])],
                "title": (t.get("title") or "")[:160],
            })
        f.write(json.dumps(slim, ensure_ascii=False) + "\n"); f.flush()
        if n % 250 == 0: print(n, file=sys.stderr, flush=True)
print("done")
