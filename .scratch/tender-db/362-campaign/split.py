"""Split the enriched cases into 35-case review batches (round-robin over the
country-sorted list so each batch mixes shapes) plus one blind sample of every 9th case,
and write index.json + args.json for review.js.
    python3 split.py <batches-dir> [model]"""
import json, os, sys
C = sys.argv[1]; model = sys.argv[2] if len(sys.argv) > 2 else "sonnet"
cases = json.load(open(f"{C}/cases.json"))
cases.sort(key=lambda c: (c["country"], c["key"]))
n = max(1, round(len(cases) / 35))
inter = [cases[i::n] for i in range(n)]
index, roots, review = [], {}, []
for bid, batch in enumerate(inter, start=1):
    keys = [c["case"] for c in batch]
    json.dump(batch, open(f"{C}/batch-{bid:02d}.json", "w"), ensure_ascii=False)
    index.append({"id": bid, "stratum": "A", "keys": keys}); roots[bid] = keys; review.append([bid, "A"])
sample = cases[::9]; sid = 90
json.dump(sample, open(f"{C}/sample-{sid:02d}.json", "w"), ensure_ascii=False)
index.append({"id": sid, "stratum": "SAMPLE", "keys": [c["case"] for c in sample]}); roots[sid] = [c["case"] for c in sample]
json.dump(index, open(f"{C}/index.json", "w"))
args = {"dir": os.path.abspath(C), "rubric": os.path.abspath(os.path.join(C, "..", "rubric.md")), "model": model,
        "review": review, "samples": [sid], "roots": {str(k): v for k, v in roots.items()}}
json.dump(args, open(f"{C}/args.json", "w"), indent=1)
print("batches:", [len(b) for b in inter], "sample:", len(sample))
