import json, os
from collections import defaultdict

S = os.environ.get("CAMPAIGN_DIR", os.getcwd())  # where the cases file lives and the outputs go
os.makedirs(f"{S}/355-batches-v2", exist_ok=True)
cases = json.load(open(f"{S}/xb-cases.json"))


def stratum(c):
    ids = [m.get("identifier") for m in c["members"]]
    k = "identical" if all(ids) and len(set(ids)) == 1 else "different"
    n = sum(1 for m in c["members"] if m.get("country_agrees"))
    return f"{k}/{'one-agrees' if n == 1 else ('none-agree' if n == 0 else 'several-agree')}"


by = defaultdict(list)
for c in sorted(cases, key=lambda c: c["root"]):
    by[stratum(c)].append(c)
batches = []
n = 0
for st, cs in sorted(by.items()):
    for i in range(0, len(cs), 35):
        n += 1
        chunk = cs[i:i + 35]
        fn = f"{S}/355-batches-v2/batch-{n:02d}.json"
        json.dump(chunk, open(fn, "w"), ensure_ascii=False, indent=0)
        batches.append({"id": n, "stratum": st, "file": fn, "roots": [c["root"] for c in chunk], "n": len(chunk)})
# A blind second-reader sample: every 9th case in root order, across all strata.
allc = sorted(cases, key=lambda c: c["root"])
sample = [c for i, c in enumerate(allc) if i % 9 == 0]
for i in range(0, len(sample), 18):
    n += 1
    chunk = sample[i:i + 18]
    fn = f"{S}/355-batches-v2/sample-{n:02d}.json"
    json.dump(chunk, open(fn, "w"), ensure_ascii=False, indent=0)
    batches.append({"id": n, "stratum": "SAMPLE", "file": fn, "roots": [c["root"] for c in chunk], "n": len(chunk)})
json.dump(batches, open(f"{S}/355-batches-v2/index.json", "w"))
print("strata:", {k: len(v) for k, v in sorted(by.items())})
print("review batches:", len([b for b in batches if b["stratum"] != "SAMPLE"]), "| sample batches:",
      len([b for b in batches if b["stratum"] == "SAMPLE"]), "| sample cases:", len(sample))
print("largest batch file KB:", max(os.path.getsize(b["file"]) for b in batches) // 1024)
