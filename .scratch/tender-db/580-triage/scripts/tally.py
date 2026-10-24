"""Tally change rows per archive: (entity_kind, op) counts, per changed_at stamp groups,
tender removed/added ids, and cross-archive reappearance of removed tender ids.
Read-only: takes archive dirs as arguments via a spec JSON; prints JSON to stdout."""
import json, sys, glob, os, datetime
from collections import Counter, defaultdict

spec = json.load(open(sys.argv[1]))
out = {"archives": [], "removed_reappear": {}}

def ts(s):
    return datetime.datetime.fromtimestamp(s, datetime.UTC).strftime('%Y-%m-%dT%H:%M:%SZ')

all_rows = []  # (cursor, kind, id, vseq, op, changed_at, archive)
for a in spec:
    files = sorted(glob.glob(a["glob"]))
    files = [f for f in files if not f.endswith('manifest.json')]
    rows = []
    for f in files:
        d = json.load(open(f))
        if "columns" in d:
            cols = d["columns"]
            ix = {c: i for i, c in enumerate(cols)}
            for r in d["rows"]:
                kind = r[ix["entity_kind"]] if "entity_kind" in ix else "tender"
                rows.append((r[ix["cursor"]], kind, r[ix["entity_id"]],
                             r[ix["version_seq"]] if "version_seq" in ix else None,
                             r[ix["op"]], r[ix["changed_at"]] if "changed_at" in ix else None,
                             r[ix["lot_tender_id"]] if "lot_tender_id" in ix else None))
        elif "events" in d:
            for e in d["events"]:
                ca = e["changed_at"]
                ca = int(datetime.datetime.strptime(ca, '%Y-%m-%dT%H:%M:%SZ').replace(tzinfo=datetime.UTC).timestamp())
                rows.append((int(e["cursor"]), e["entity"], e["id"], e.get("version"), e["op"], ca, None))
    rows.sort()
    # dedupe by cursor
    seen = set(); uniq = []
    for r in rows:
        if r[0] in seen: continue
        seen.add(r[0]); uniq.append(r)
    rows = uniq
    lo, hi = a["lo"], a["hi"]
    inrange = all(lo < r[0] <= hi for r in rows)
    cnt = Counter((r[1], r[4]) for r in rows)
    vnull = Counter((r[1], r[4], r[3] is None) for r in rows)
    # stamp groups
    groups = defaultdict(list)
    for r in rows:
        groups[r[5]].append(r)
    g_out = []
    for st in sorted(groups):
        g = groups[st]
        c = Counter((r[1], r[4]) for r in g)
        g_out.append({
            "changed_at": ts(st) if st else None,
            "cursor_first": g[0][0], "cursor_last": g[-1][0], "rows": len(g),
            "tender_removed": c[("tender", "removed")], "tender_added": c[("tender", "added")],
            "tender_changed": c[("tender", "changed")],
            "removed_other": {k: v for (k, op), v in c.items() if op == "removed" and k != "tender"},
            "counts": {f"{k}/{op}": v for (k, op), v in sorted(c.items())},
        })
    rem = [r for r in rows if r[1] == "tender" and r[4] == "removed"]
    add = [r for r in rows if r[1] == "tender" and r[4] == "added"]
    tender_ids_touched = len({r[2] for r in rows if r[1] == "tender"})
    arc = {
        "name": a["name"], "path": a["path"], "lo": lo, "hi": hi, "files": len(files),
        "rows": len(rows), "all_in_range": inrange, "kinds": a.get("kinds", "all"),
        "counts": {f"{k}/{op}": v for (k, op), v in sorted(cnt.items())},
        "counts_vnull": {f"{k}/{op}/{'vnull' if n else 'versioned'}": v for (k, op, n), v in sorted(vnull.items())},
        "tender_removed": len(rem), "tender_added": len(add),
        "tender_removed_ids": [r[2] for r in rem],
        "tender_added_id_range": [min(r[2] for r in add), max(r[2] for r in add)] if add else None,
        "distinct_tenders_touched": tender_ids_touched,
        "stamp_groups": g_out,
    }
    out["archives"].append(arc)
    for r in rows:
        all_rows.append(r + (a["name"],))

# removed lot ids: are they the removed tenders' lots? (lot_tender_id is NULL after deletion, so can't join)
# reappearance: any row for a removed tender id at a later cursor (any archive)
by_tender = defaultdict(list)
for r in all_rows:
    if r[1] == "tender":
        by_tender[r[2]].append((r[0], r[4], r[7]))
re = {}
for arc in out["archives"]:
    for tid in arc["tender_removed_ids"]:
        evs = sorted(set(by_tender[tid]))
        rem_c = [c for c, op, _ in evs if op == "removed"]
        later = [(c, op, n) for c, op, n in evs if c > min(rem_c)]
        earlier = [(c, op, n) for c, op, n in evs if c < min(rem_c)]
        re[str(tid)] = {"removed_at": rem_c, "archive": arc["name"], "later": later, "earlier_ops": Counter(op for _, op, _ in earlier)}
out["removed_reappear"] = re
print(json.dumps(out, indent=1, default=lambda o: dict(o)))
