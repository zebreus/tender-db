#!/usr/bin/env python3
# Check a served /v1/tenders/{id} body's versions against the fold-order invariant:
# (published instant, source_rank, publication_id, caused_by_notice_id) strictly ascending in seq,
# source_rank = 1 for TED, 0 otherwise. Source is read off the publication_id shape (TED 'NNNNNNNN-YYYY';
# DÖE a UUID with a '-NN' suffix). A date-only published_at is the civil day's UTC midnight (issue 418).
import json, re, sys
from datetime import datetime, timezone
TED = re.compile(r'^\d{8}-\d{4}$')
DOE = re.compile(r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}-\d+$')
def instant(s):
    if len(s) == 10:
        return int(datetime.strptime(s, '%Y-%m-%d').replace(tzinfo=timezone.utc).timestamp())
    return int(datetime.fromisoformat(s).timestamp())
d = '/home/user/tender-db/.scratch/tender-db/539-triage/post-drain-chains'
for tid in sys.argv[1:]:
    j = json.load(open(f'{d}/t{tid}.json'))
    vs = j['versions']
    print(f'== tender {tid}: source {j["source"]}, top published_at {j["published_at"]}, version {j.get("version")}, {len(vs)} versions')
    prev = None; bad = 0
    for v in vs:
        pid = v['publication_id']
        src = 'ted' if TED.match(pid) else ('doe' if DOE.match(pid) else '?')
        key = (instant(v['published_at']), 1 if src == 'ted' else 0, pid, v['caused_by_notice_id'])
        ok = prev is None or prev < key
        tie = prev is not None and prev[0] == key[0]
        bad += not ok
        print(f"  seq {v['seq']:>2} {src} {v['published_at']:<25} {pid:<44} n{v['caused_by_notice_id']}"
              f"{'' if ok else '  <-- VIOLATION'}{'  (same instant as previous: tie-break ' + ('rank' if prev[1] != key[1] else 'publication_id' if prev[2] != key[2] else 'notice id') + ')' if tie else ''}")
        prev = key
    seqs = [v['seq'] for v in vs]
    print(f'  seqs contiguous 1..n: {seqs == list(range(1, len(vs) + 1))}; violations: {bad}; head published_at == last version: {j["published_at"] == vs[-1]["published_at"]}')
