#!/bin/bash
# reviewer 5: fetch listed paths through get1.sh (sleep 2, log, cache)
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
for p in "$@"; do bash "$U3/get1.sh" "$p"; done
