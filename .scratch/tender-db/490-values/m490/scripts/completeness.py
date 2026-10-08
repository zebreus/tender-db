"""Issue 490's completeness read: tenders not at projection_epoch 4, in bounded 100k-id windows
from MAX(id) down, with an epoch breakdown for any window that is not clean. Never retries a 408:
a window that times out is reported and skipped.

    python3 completeness.py [window]"""
import json, subprocess, sys

WINDOW = int(sys.argv[1]) if len(sys.argv) > 1 else 100_000


def sql(query):
    p = subprocess.run(["ssh", "-o", "ConnectTimeout=15", "root@zebreus.click", "/root/sq.sh"],
                       input=query, capture_output=True, text=True, timeout=90)
    try:
        d = json.loads(p.stdout)
    except Exception:
        return None, p.stdout[:200]
    return d.get("rows"), d


top = sql("SELECT MAX(id) FROM tenders")[0][0][0]
total, timeouts = 0, []
lo = (top // WINDOW) * WINDOW
while lo >= 0:
    hi = lo + WINDOW - 1
    rows, raw = sql(f"SELECT COUNT(*) FROM tenders WHERE id BETWEEN {lo} AND {hi} AND projection_epoch <> 4")
    if rows is None:
        timeouts.append((lo, hi, str(raw)[:120]))
    else:
        n = rows[0][0]
        total += n
        if n:
            by, _ = sql(
                f"SELECT projection_epoch, COUNT(*) FROM tenders WHERE id BETWEEN {lo} AND {hi} "
                "AND projection_epoch <> 4 GROUP BY projection_epoch"
            )
            print(f"[{lo},{hi}] {n} not at epoch 4: {by}", flush=True)
    lo -= WINDOW
print(f"MAX(id)={top}: {total} tenders not at epoch 4; {len(timeouts)} window(s) unread: {timeouts}")
