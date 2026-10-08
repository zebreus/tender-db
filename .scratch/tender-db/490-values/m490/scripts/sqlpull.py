"""Pull one window's single-table range reads through /v1/sql (bounded, PK/index seeks)."""
import json, subprocess, sys, os, time

def sq(sql):
    p = subprocess.run(["ssh", "-o", "ConnectTimeout=15", "root@zebreus.click", "/root/sq.sh"],
                       input=sql, capture_output=True, text=True, timeout=60)
    out = p.stdout
    try:
        d = json.loads(out)
    except Exception:
        raise SystemExit(f"NON-JSON body for {sql!r}: {out[:300]!r} {p.stderr[:300]!r}")
    if "rows" not in d or "columns" not in d or "row_count" not in d:
        raise SystemExit(f"ERROR body for {sql!r}: {out[:300]!r}")
    if d.get("truncated"):
        raise SystemExit(f"TRUNCATED for {sql!r}: {d['row_count']} rows")
    return d

def main():
    name, lo, hi, outdir = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    step = int(sys.argv[5]) if len(sys.argv) > 5 else (hi - lo + 1)
    qs = {
        "tenders": "SELECT id, current_seq, source FROM tenders WHERE id BETWEEN {a} AND {b}",
        "lots": "SELECT id, tender_id, lot_key FROM lots WHERE tender_id BETWEEN {a} AND {b}",
        "tvl": "SELECT tender_id, seq, lot_id, kind FROM tender_version_lots WHERE tender_id BETWEEN {a} AND {b}",
        "amounts": "SELECT tender_id, seq, lot_id, field, cents, currency, eur_cents, quality FROM tender_version_amounts WHERE tender_id BETWEEN {a} AND {b}",
        "lot_results": "SELECT tender_id, seq, awarded_cents, awarded_currency FROM tender_version_lot_results WHERE tender_id BETWEEN {a} AND {b} AND awarded_cents IS NOT NULL",
    }
    res = {"window": [lo, hi], "fetched_at": time.time()}
    for key, q in qs.items():
        rows = []
        cols = None
        a = lo
        while a <= hi:
            b = min(hi, a + step - 1)
            d = sq(q.format(a=a, b=b))
            cols = d["columns"]
            rows.extend(d["rows"])
            print(f"{name} {key} {a}-{b}: {d['row_count']} rows", flush=True)
            a = b + 1
        res[key] = {"columns": cols, "rows": rows}
    with open(os.path.join(outdir, f"{name}.sql.json"), "w") as f:
        json.dump(res, f)

main()
