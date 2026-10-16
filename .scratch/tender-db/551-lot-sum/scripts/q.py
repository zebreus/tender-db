"""Run ONE named /v1/sql query (sql/<name>.sql) through the box helper, never retrying.

- refuses to run when the budget file says the hard cap is reached;
- paces >= 2.5 s after the previous query (any name);
- refuses to run anything but `health` while a 408/error flag is set (cleared by a good `health`);
- writes data/<name>.json (raw body) and appends a line to data/queries.log.
Exit code 0 = an answer with rows+columns, 2 = error body / 408 / non-JSON (recorded, NOT retried), 3 = refused.
"""
import json, os, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
CAP = 72                      # my own hard cap (the task's 80 minus the port author's 5, minus slack)
BUDGET = os.path.join(HERE, "data", "budget.json")
LOG = os.path.join(HERE, "data", "queries.log")


def load_budget():
    try:
        return json.load(open(BUDGET))
    except FileNotFoundError:
        return {"used": 0, "last": 0.0, "flag": None}


def main(name):
    b = load_budget()
    if b["used"] >= CAP:
        print(f"REFUSED: budget {b['used']}/{CAP} used"); return 3
    if b["flag"] and name != "health":
        print(f"REFUSED: flag set ({b['flag']}); run `health` first"); return 3
    sql = open(os.path.join(HERE, "sql", name + ".sql")).read().strip()
    wait = b["last"] + 2.5 - time.time()
    if wait > 0:
        time.sleep(wait)
    t0 = time.time()
    b["used"] += 1
    b["last"] = t0
    json.dump(b, open(BUDGET, "w"))
    try:
        p = subprocess.run(["ssh", "-o", "ConnectTimeout=15", "root@zebreus.click", "/root/sq.sh"],
                           input=sql, capture_output=True, text=True, timeout=120)
        out, err = p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        out, err = "", "local timeout 120 s"
    dt = time.time() - t0
    b["last"] = time.time()
    open(os.path.join(HERE, "data", name + ".json"), "w").write(out)
    ok = False
    try:
        d = json.loads(out)
        if isinstance(d, dict) and "rows" in d and "columns" in d:
            ok = True
            status = f"rows={d.get('row_count')} truncated={d.get('truncated')}"
        else:
            status = f"ERROR-BODY {out[:300]!r}"
    except Exception:
        status = f"NON-JSON {out[:300]!r} stderr={err[:300]!r}"
    if not ok:
        b["flag"] = f"{name}: {status[:120]}"
    elif name == "health":
        b["flag"] = None
    json.dump(b, open(BUDGET, "w"))
    line = f"{time.strftime('%Y-%m-%dT%H:%M:%S')} #{b['used']} {name} {dt:.1f}s {status}"
    print(line)
    with open(LOG, "a") as f:
        f.write(line + "\n")
    return 0 if ok else 2


if __name__ == "__main__":
    rc = 0
    for n in sys.argv[1:]:
        rc = main(n)
        if rc != 0:
            break
    sys.exit(rc)
