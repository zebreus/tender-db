#!/bin/bash
# Page GB national-identifier orgs by identifier (unique-index seek), 2000 per bounded query.
last=""
: > /root/gb-orgs.jsonl
while :; do
  sql="SELECT identifier, id, name FROM organizations WHERE country = 'GB' AND identifier_kind = 'national' AND identifier > '$last' ORDER BY identifier LIMIT 2000"
  out=$(echo "$sql" | /root/sq.sh)
  n=$(echo "$out" | python3 -c "import json,sys; d=json.load(sys.stdin); rows=d.get('rows',[]); [print(json.dumps(r)) for r in rows]" >> /root/gb-orgs.jsonl; echo "$out" | python3 -c "import json,sys; d=json.load(sys.stdin); print(len(d.get('rows',[])))")
  [ "$n" -lt 2000 ] && break
  last=$(tail -1 /root/gb-orgs.jsonl | python3 -c "import json,sys; print(json.loads(sys.stdin.read())[0])")
done
wc -l /root/gb-orgs.jsonl
