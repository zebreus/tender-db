#!/bin/bash
# refute2_3 (Hull x1 on Artifax): sequential public GETs under the shared flock (.get.lock), >=2 s apart
# (sleep 2, and >=2 s after the shared log's last line), logged with the (refute2) tag, cached under bodies/; no retries.
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
LOG="$U3/api_requests.log"
mkdir -p "$U3/bodies"
for p in "$@"; do
  f="$U3/bodies/$(echo "$p" | sed 's#^/##; s#[/?&=]#_#g').json"
  if [ -s "$f" ]; then echo "cached $f"; continue; fi
  (
    flock 9
    sleep 2
    while :; do
      last=$(tail -1 "$LOG" | cut -d' ' -f1)
      lt=$(date -u -d "$last" +%s 2>/dev/null || echo 0)
      now=$(date -u +%s)
      [ $((now - lt)) -ge 2 ] && break
      sleep 1
    done
    code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
    line="$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $p rc=$code (refute2)"; echo "$line" >> "$LOG"; echo "$line" >> "$U3/refute2_3_gets.log"
    if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $p -> $f"; else mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; fi
  ) 9>"$U3/.get.lock"
done
