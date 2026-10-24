"""Per archive and per job: retirement blocks (tender removed + the consecutive version-null
removed rows of its lots/lot_results/bids/contracts that retire_chunk_tx writes right after it),
other removed rows, minted (new) tender ids vs 'added' on existing ids, job attribution by
changed_at against the job ledger. Read-only; prints JSON."""
import json, sys, glob, datetime
from collections import Counter, defaultdict

spec = json.load(open(sys.argv[1]))
jobs = json.load(open(sys.argv[2]))  # {job_id: {...}}

def ts(s):
    return datetime.datetime.fromtimestamp(s, datetime.UTC).strftime('%Y-%m-%dT%H:%M:%SZ')

def load(a):
    files = sorted(f for f in glob.glob(a["glob"]) if not f.endswith('manifest.json'))
    rows = {}
    for f in files:
        d = json.load(open(f))
        if "columns" in d:
            ix = {c: i for i, c in enumerate(d["columns"])}
            for r in d["rows"]:
                rows[r[ix["cursor"]]] = (r[ix["cursor"]], r[ix["entity_kind"]], r[ix["entity_id"]],
                                         r[ix["version_seq"]], r[ix["op"]], r[ix["changed_at"]])
        else:
            for e in d["events"]:
                ca = int(datetime.datetime.strptime(e["changed_at"], '%Y-%m-%dT%H:%M:%SZ').replace(tzinfo=datetime.UTC).timestamp())
                rows[int(e["cursor"])] = (int(e["cursor"]), e["entity"], e["id"], e.get("version"), e["op"], ca)
    return [rows[k] for k in sorted(rows)]

# job attribution: the job (any kind) whose [started_at-1, finished_at+1] contains the stamp;
# prefer the latest-started one (a project starts the second its refold ends)
WRITERS = {"project", "rederive-eur-recent", "apply-rehoming", "refresh-org-names",
           "dissolve-guid-twins", "repair-notice-instants", "rederive-eur"}
jl = sorted((j for j in jobs.values() if j["kind"] in WRITERS), key=lambda j: (j["started_at"], j["job_id"]))
def job_of(st, only_vnull_changed):
    # writers whose [started_at, finished_at] holds the stamp. A stamp group of only version-null
    # `changed` rows goes to a rederive-eur(-recent) candidate when there is one (its D5 rows);
    # any other group goes to the latest-started candidate (a fold's project stamps all its rows
    # with the one `now` it took at start, the second the rederive before it ended).
    c = [j for j in jl if j["started_at"] <= st <= (j.get("finished_at") or j["started_at"])]
    if not c:
        return None
    if only_vnull_changed:
        r = [j for j in c if j["kind"].startswith("rederive")]
        if r:
            return r[0]["job_id"]
    return c[-1]["job_id"]

seen_tender_ids = set()
max_id = 0
out = []
for a in spec:
    rows = load(a)
    grp = defaultdict(list)
    for r in rows:
        grp[r[5]].append(r)
    stamp_job = {st: job_of(st, all(r[4] == "changed" and r[3] is None for r in g)) for st, g in grp.items()}
    # running max of tender ids seen BEFORE this archive (for minted classification)
    per_job = defaultdict(lambda: {"rows": 0, "tender_removed": 0, "tender_added": 0, "minted": 0,
                                    "added_existing": 0, "retire_attached": Counter(),
                                    "other_removed": Counter(), "tender_changed": 0,
                                    "removed_ids": [], "minted_range": [None, None], "stamps": set(),
                                    "cursor": [None, None]})
    i = 0
    n = len(rows)
    while i < n:
        c, kind, eid, vseq, op, st = rows[i]
        jid = stamp_job[st]
        pj = per_job[jid]
        pj["stamps"].add(ts(st))
        pj["cursor"][0] = c if pj["cursor"][0] is None else pj["cursor"][0]
        pj["cursor"][1] = c
        pj["rows"] += 1
        if kind == "tender" and op == "removed":
            pj["tender_removed"] += 1
            pj["removed_ids"].append(eid)
            seen_tender_ids.add(eid)
            j = i + 1
            while j < n and rows[j][4] == "removed" and rows[j][3] is None and rows[j][1] in ("lot", "lot_result", "bid", "contract") and rows[j][0] == rows[j-1][0] + 1:
                pj["retire_attached"][rows[j][1]] += 1
                pj["rows"] += 1
                pj["cursor"][1] = rows[j][0]
                j += 1
            i = j
            continue
        if op == "removed":
            pj["other_removed"][f"{kind}/{'vnull' if vseq is None else 'versioned'}"] += 1
        if kind == "tender":
            if op == "added":
                pj["tender_added"] += 1
                if eid > max_id and eid not in seen_tender_ids:
                    pj["minted"] += 1
                    lo, hi = pj["minted_range"]
                    pj["minted_range"] = [eid if lo is None else min(lo, eid), eid if hi is None else max(hi, eid)]
                else:
                    pj["added_existing"] += 1
            elif op == "changed":
                pj["tender_changed"] += 1
            seen_tender_ids.add(eid)
            max_id = max(max_id, eid)
        i += 1
    arc = {"name": a["name"], "path": a["path"], "lo": a["lo"], "hi": a["hi"], "rows": len(rows), "jobs": []}
    for jid in sorted(per_job, key=lambda k: per_job[k]["cursor"][0]):
        pj = per_job[jid]
        j = jobs.get(str(jid)) if jid is not None else None
        arc["jobs"].append({
            "job_id": jid, "kind": j["kind"] if j else None,
            "params": (j.get("params") or "")[:80] if j else None,
            "stamps": [min(pj["stamps"]), max(pj["stamps"])], "cursor": pj["cursor"], "rows": pj["rows"],
            "tender_changed": pj["tender_changed"], "tender_removed": pj["tender_removed"],
            "retire_attached": dict(pj["retire_attached"]), "tender_added": pj["tender_added"],
            "minted": pj["minted"], "minted_range": pj["minted_range"], "added_existing": pj["added_existing"],
            "other_removed": dict(pj["other_removed"]), "removed_ids": pj["removed_ids"],
        })
    out.append(arc)
print(json.dumps(out, indent=1))
