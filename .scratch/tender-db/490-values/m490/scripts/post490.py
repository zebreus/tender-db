"""Issue 490's closing compare: stored tender_version_lots.value_* at each tender's current seq
against the REST lot value served right now (deploy A: summarise elects at read time through
elect_lot_value). Run after the backfill project finishes, per window:

    python3 post490.py <name> <outdir>

It reads <outdir>/<name>.sql.json for the window bounds, writes <outdir>/<name>.post.sql.json
(the stored values) and re-pulls REST with restpull.py into <outdir>/<name>.post.rest.json, then
prints the differences. Pass condition: 0 differences on every window."""
import json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))


def sql(query):
    p = subprocess.run(["ssh", "-o", "ConnectTimeout=15", "root@zebreus.click", "/root/sq.sh"],
                       input=query, capture_output=True, text=True, timeout=90)
    d = json.loads(p.stdout)
    if "rows" not in d:
        raise SystemExit(f"SQL error: {p.stdout[:300]!r}")
    if d.get("truncated"):
        raise SystemExit("SQL result truncated: shrink the window")
    return d["rows"]


def main():
    name, outdir = sys.argv[1], sys.argv[2]
    base = json.load(open(os.path.join(outdir, f"{name}.sql.json")))
    lo, hi = base["window"]
    rows = sql(
        "SELECT v.tender_id, v.lot_id, v.value_cents, v.value_currency, t.projection_epoch "
        "FROM tenders t JOIN tender_version_lots v ON v.tender_id = t.id AND v.seq = t.current_seq "
        f"WHERE t.id BETWEEN {lo} AND {hi}"
    )
    json.dump({"window": [lo, hi], "rows": rows}, open(os.path.join(outdir, f"{name}.post.sql.json"), "w"))
    stale = sorted({r[0] for r in rows if r[4] != 4})
    # restpull.py reads <name>.sql.json for the lot set; give it a copy under the post name.
    post = f"{name}.post"
    with open(os.path.join(outdir, f"{post}.sql.json"), "w") as f:
        json.dump(base, f)
    subprocess.run([sys.executable, os.path.join(HERE, "restpull.py"), post, outdir], check=True)
    rest = {it["id"]: it for it in json.load(open(os.path.join(outdir, f"{post}.rest.json")))["items"]}
    diffs, compared, missing = [], 0, 0
    for tender_id, lot_id, cents, currency, _epoch in rows:
        it = rest.get(lot_id)
        if it is None:
            missing += 1
            continue
        compared += 1
        served = it.get("value")
        stored = {"cents": cents, "currency": currency} if cents is not None else None
        if served != stored:
            diffs.append((tender_id, lot_id, stored, served))
    print(f"{name} [{lo},{hi}]: {compared} lots compared, {missing} not served, {len(stale)} tenders not at "
          f"epoch 4, {len(diffs)} differences")
    for d in diffs[:20]:
        print("  DIFF", d)
    if stale:
        print("  STALE", stale[:20])


main()
