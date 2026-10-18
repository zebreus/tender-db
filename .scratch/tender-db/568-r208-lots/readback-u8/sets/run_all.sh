#!/bin/bash
S=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8
H=/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8
R=/home/user/tender-db/.scratch/tender-db/568-r208-lots
cd $S/sets/tools
U7="--reuse-notices $S/sets/u7notices --reuse-notices $S/fresh"
OOS="--reuse-notices $R/u8-oos/cache_o1 --reuse-notices $R/u8-oos/cache_o2 --reuse-notices $S/oos/cache_o1 --reuse-notices $S/oos/cache_o2"
P570=""
for d in $(ls -d $S/c570/*_skeptic/live | sort); do P570="$P570 --reuse-notices $d"; done
for i in 1 2 3 4 5 6 7 8 9 10 11 12; do P570="$P570 --reuse-notices $S/c570/cache_k$i"; done
P570="$P570 --reuse-notices $S/c567"
run() { name=$1; shift; python3 -I -B $H/check_live.py --cache $S/sets/live --gap 1.6 "$@" --out $S/sets/out/$name.json $S/sets/ids/$name.json > $S/sets/out/$name.log 2>&1; echo "$name GATE-EXIT=$?" >> $S/sets/out/exits.txt; }
run s150 $U7
run f30 $U7
run m77 $U7
run oos $OOS $U7
run p570 $P570 $U7
echo ALLDONE >> $S/sets/out/exits.txt
