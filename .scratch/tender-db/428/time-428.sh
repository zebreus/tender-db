#!/bin/bash
# time-428.sh FILE NAME... — issue 428 timings on the two snapshot copies ONLY.
# Cold = the copy's cached pages dropped first (fadvise DONTNEED on the COPY file;
# the serving DB's cache is untouched), warm = straight after. 120 s kill per run
# (turso cannot interrupt a statement, issue 425), memory-capped cgroup, idle IO.
set -u
F=$1; shift
D=/data/db/analyze-428
P=/root/plan-probe-428.new
drop() { python3 -c "import os,sys; fd=os.open(sys.argv[1], os.O_RDONLY); os.posix_fadvise(fd,0,0,os.POSIX_FADV_DONTNEED); os.close(fd)" "$1"; }
for name in "$@"; do
  for db in ${DBS:-base stats}; do
    drop "$D/$db.db"
    for pass in cold warm; do
      out=$(systemd-run --wait --pipe --quiet --collect -p MemoryMax=4G -p IOSchedulingClass=idle -p Nice=10 \
            timeout 120 "$P" run "$D/$db.db" "$F" "$name" 2>&1); rc=$?
      [ $rc -eq 124 ] && out="RUN	$name	TIMEOUT(120s)"
      [ $rc -ne 0 ] && [ $rc -ne 124 ] && out="RUN	$name	ERR rc=$rc $out"
      echo "$db	$pass	$out"
    done
  done
done
