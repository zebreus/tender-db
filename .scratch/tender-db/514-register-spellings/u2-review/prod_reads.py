#!/usr/bin/env python3
"""Issue 514 U2.1, step 2: re-read every group member and lone mover on prod.

Bounded, index-served SELECTs through the box helper (`echo SQL | ssh root@zebreus.click
/root/sq.sh`), sequential, never retried; a 408 is recorded and the call is not repeated.
- organizations by primary key (79 ids);
- organization_mentions by organization_id (index organization_mentions_org), grouped by name,
  raw_identifier, scheme and country, in two calls (52 group members, 27 lone movers);
- organization_names by org_id (primary-key prefix);
- organizations on organizations_identity(country, identifier_kind, identifier) for the 53
  canonical values and for the 54 stored (pre-rewrite) values, to see who holds each today.

Every response is written verbatim to prod/NN-<tag>.out; prod/journal.jsonl holds the SQL, the
seconds, the exit code and the 408 flag of each call.

Usage: python3 -I prod_reads.py
"""
import json
import os
import subprocess
import time

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "prod")


def ids_sql(ids):
    assert 0 < len(ids) <= 100
    return ",".join(str(i) for i in ids)


def strs_sql(vals):
    assert 0 < len(vals) <= 100
    for v in vals:
        assert v.isalnum(), v
    return ",".join("'%s'" % v for v in vals)


def main():
    g = json.load(open(os.path.join(HERE, "groups.json")))
    member_ids = sorted(m["org_id"] for x in g["groups"] for m in x["members"])
    mover_ids = sorted(m["org_id"] for x in g["lone_movers"] for m in x["members"])
    all_ids = sorted(member_ids + mover_ids)
    canon = sorted({x["canonical"] for x in g["groups"] + g["lone_movers"]})
    stored = sorted({m["stored_identifier"] for x in g["groups"] + g["lone_movers"] for m in x["members"]
                     if m["moves"]})
    assert len(all_ids) == 79 and len(canon) == 53 and len(stored) == 54, (len(all_ids), len(canon), len(stored))

    calls = [
        ("orgs", "SELECT id, country, identifier_kind, identifier, name, provisional FROM organizations "
                 "WHERE id IN (%s) LIMIT 100" % ids_sql(all_ids)),
        ("mnames-members", "SELECT organization_id, name, raw_identifier, scheme, country, COUNT(*) AS n "
                           "FROM organization_mentions WHERE organization_id IN (%s) "
                           "GROUP BY organization_id, name, raw_identifier, scheme, country LIMIT 2000"
                           % ids_sql(member_ids)),
        ("mnames-movers", "SELECT organization_id, name, raw_identifier, scheme, country, COUNT(*) AS n "
                          "FROM organization_mentions WHERE organization_id IN (%s) "
                          "GROUP BY organization_id, name, raw_identifier, scheme, country LIMIT 2000"
                          % ids_sql(mover_ids)),
        ("onames", "SELECT org_id, lang, name FROM organization_names WHERE org_id IN (%s) LIMIT 400"
                   % ids_sql(all_ids)),
        ("holders-canonical", "SELECT id, identifier, name, provisional FROM organizations WHERE country = 'GB' "
                              "AND identifier_kind = 'national' AND identifier IN (%s) LIMIT 200" % strs_sql(canon)),
        ("holders-stored", "SELECT id, identifier, name, provisional FROM organizations WHERE country = 'GB' "
                           "AND identifier_kind = 'national' AND identifier IN (%s) LIMIT 200" % strs_sql(stored)),
    ]
    os.makedirs(OUT, exist_ok=True)
    jp = open(os.path.join(OUT, "journal.jsonl"), "a")
    for i, (tag, sql) in enumerate(calls, 1):
        fn = "%02d-%s.out" % (i, tag)
        if os.path.exists(os.path.join(OUT, fn)):
            print("skip (already read)", fn)
            continue
        t0 = time.time()
        p = subprocess.run(["ssh", "-o", "BatchMode=yes", "root@zebreus.click", "/root/sq.sh"],
                           input=sql.encode(), capture_output=True, timeout=120)
        secs = round(time.time() - t0, 2)
        body = p.stdout.decode("utf-8", "replace")
        open(os.path.join(OUT, fn), "w").write(body)
        rows = truncated = err = None
        try:
            d = json.loads(body)
            rows, truncated = d.get("row_count"), d.get("truncated")
            err = d.get("error")
        except Exception as e:  # non-JSON: an HTTP error body
            err = body[:300] or p.stderr.decode()[:300]
        is_408 = "408" in (str(err) if err else "") or "timed out" in (str(err) or "").lower()
        rec = dict(call=i, tag=tag, sql=sql, rc=p.returncode, secs=secs, out=fn, rows=rows,
                   truncated=truncated, error=err, is_408=is_408)
        jp.write(json.dumps(rec) + "\n")
        jp.flush()
        print(i, tag, "rc", p.returncode, "secs", secs, "rows", rows, "trunc", truncated, "err", err)
        if is_408:
            print("408 -- not retried")


if __name__ == "__main__":
    main()
