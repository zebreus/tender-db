"""Cut a country-cluster-packet report into review batches (issue 357).

    python3 split.py <packet.json> <out-dir> [--exclude <slice-record.json>...]

<packet.json> is the body of GET /admin/reports/country-cluster-packet as the
admin helper returns it ({"body": ...}). Cases whose member lookup returned
fewer than two rows are dropped (the census counted rows the identity index
could not return, e.g. a NULL kind). --exclude drops identifiers already in a
committed slice record (.scratch/tender-db/357-cluster-country-verdicts-sliceN.json);
since c4fd5f8 the packet itself leaves out reviewed clusters, so this is a belt.

Writes <out-dir>/cases.json (what post.py reads), batch-NN.json per stratum
(35 cases each; stratum = census verdict / spray|balanced shape), sample-NN.json
(every 10th case in identifier order, 20 per file, for the blind second readers)
index.json and args.json — the Workflow tool's args for review.js (dir, rubric,
model, review, samples, roots); see README.md.
"""
import json
import os
import shutil
import sys
from collections import defaultdict

BATCH = 35
SAMPLE_EVERY = 10
SAMPLE_FILE = 20

args = sys.argv[1:]
packet, out = args[0], args[1]
exclude = set()
for rec in args[3:] if len(args) > 2 and args[2] == "--exclude" else []:
    exclude |= {c["identifier"] for c in json.load(open(rec))["cases"]}

d = json.load(open(packet))
b = d["body"] if "body" in d else d
if isinstance(b, str):
    b = json.loads(b)
cases = b["cases"]
good = [c for c in cases if len(c["members"]) >= 2 and c["identifier"] not in exclude]
print(f"packet: {b['clusters']} clusters, {b['eligible']} eligible, {b.get('already_reviewed', '?')} already reviewed, "
      f"{len(cases)} carried; {len(good)} cases with >=2 member rows and not excluded")


def shape(c):
    ms = sorted((m["mentions"] for m in c["members"]), reverse=True)
    return "spray" if len(ms) >= 2 and ms[0] >= 10 * max(1, ms[1]) else "balanced"


by = defaultdict(list)
for c in sorted(good, key=lambda c: c["identifier"]):
    by[f"{c['verdict']}/{shape(c)}"].append(c)
print("strata:", {k: len(v) for k, v in sorted(by.items())})

shutil.rmtree(out, ignore_errors=True)
os.makedirs(out)
json.dump(good, open(f"{out}/cases.json", "w"), ensure_ascii=False)
batches, n = [], 0
for st, cs in sorted(by.items()):
    for i in range(0, len(cs), BATCH):
        n += 1
        chunk = cs[i:i + BATCH]
        fn = f"{out}/batch-{n:02d}.json"
        json.dump(chunk, open(fn, "w"), ensure_ascii=False, indent=0)
        batches.append({"id": n, "stratum": st, "file": fn, "keys": [c["identifier"] for c in chunk], "n": len(chunk)})
allc = sorted(good, key=lambda c: c["identifier"])
sample = [c for i, c in enumerate(allc) if i % SAMPLE_EVERY == 0]
for i in range(0, len(sample), SAMPLE_FILE):
    n += 1
    chunk = sample[i:i + SAMPLE_FILE]
    fn = f"{out}/sample-{n:02d}.json"
    json.dump(chunk, open(fn, "w"), ensure_ascii=False, indent=0)
    batches.append({"id": n, "stratum": "SAMPLE", "file": fn, "keys": [c["identifier"] for c in chunk], "n": len(chunk)})
json.dump(batches, open(f"{out}/index.json", "w"))
rev = [x for x in batches if x["stratum"] != "SAMPLE"]
sam = [x for x in batches if x["stratum"] == "SAMPLE"]
print(f"review batches: {len(rev)} | sample batches: {len(sam)} | sample cases: {len(sample)} | "
      f"largest KB: {max(os.path.getsize(x['file']) for x in batches) // 1024}")
here = os.path.dirname(os.path.abspath(__file__))
wf_args = {"dir": os.path.abspath(out), "rubric": os.environ.get("RUBRIC", f"{here}/rubric-v3c.md"),
           "model": os.environ.get("REVIEW_MODEL", "sonnet"),
           "review": [[x["id"], x["stratum"]] for x in rev], "samples": [x["id"] for x in sam],
           "roots": {str(x["id"]): x["keys"] for x in batches}}
json.dump(wf_args, open(f"{out}/args.json", "w"))
print(f"workflow args written to {out}/args.json")
