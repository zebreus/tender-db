"""Fetch the REST lot rows for one window's lots: page /v1/lots by lot-id cursor over the
dense cluster, and ?tender= for tenders whose lots sit outside it."""
import json, subprocess, sys, os, time

def get(path):
    p = subprocess.run(["ssh", "-o", "ConnectTimeout=15", "root@zebreus.click",
                        f'curl -s --max-time 30 "http://127.0.0.1:8080{path}"'],
                       capture_output=True, text=True, timeout=60)
    try:
        d = json.loads(p.stdout)
    except Exception:
        raise SystemExit(f"NON-JSON for {path}: {p.stdout[:300]!r} {p.stderr[:200]!r}")
    if "items" not in d:
        raise SystemExit(f"ERROR body for {path}: {p.stdout[:300]!r}")
    return d

def main():
    name, outdir = sys.argv[1], sys.argv[2]
    gap = int(sys.argv[3]) if len(sys.argv) > 3 else 5000
    s = json.load(open(os.path.join(outdir, f"{name}.sql.json")))
    lo, hi = s["window"]
    cur = {r[0]: r[1] for r in s["tenders"]["rows"]}
    current = sorted({r[2] for r in s["tvl"]["rows"] if cur.get(r[0]) == r[1]})
    lot_tender = {r[0]: r[1] for r in s["lots"]["rows"]}
    # dense cluster = the longest run of current lot ids without a gap > `gap`
    runs, start = [], 0
    for i in range(1, len(current) + 1):
        if i == len(current) or current[i] - current[i - 1] > gap:
            runs.append((start, i)); start = i
    a, b = max(runs, key=lambda r: r[1] - r[0])
    clo, chi = current[a], current[b - 1]
    outside = sorted({lot_tender[l] for l in current if not (clo <= l <= chi)})
    items = {}
    cursor = clo - 1
    pages = 0
    while True:
        d = get(f"/v1/lots?cursor={cursor}&limit=500")
        pages += 1
        for it in d["items"]:
            if it["id"] > chi: break
            items[it["id"]] = it
        if not d["items"] or d["items"][-1]["id"] >= chi or not d.get("next_cursor"):
            break
        cursor = int(d["next_cursor"])
    for t in outside:
        d = get(f"/v1/lots?tender={t}&limit=1000")
        for it in d["items"]:
            items[it["id"]] = it
    print(f"{name}: cluster {clo}-{chi}, {pages} pages, {len(outside)} straggler tenders, {len(items)} REST rows", flush=True)
    with open(os.path.join(outdir, f"{name}.rest.json"), "w") as f:
        json.dump({"fetched_at": time.time(), "items": list(items.values())}, f)

main()
