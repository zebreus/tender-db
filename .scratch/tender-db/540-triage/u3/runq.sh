#!/bin/bash
# usage: runq.sh <tag> <lo> <hi> ; writes s<tag>_win.sql, s<tag>.json, s<tag>.err; prints EXIT and secs
tag=$1; lo=$2; hi=$3
d=/home/user/tender-db/.scratch/tender-db/540-triage/u3
sed -e "s/@LO@/$lo/; s/@HI@/$hi/" $d/${TEMPLATE:-win_template.sql} > $d/s${tag}_win.sql
scp -q $d/s${tag}_win.sql root@zebreus.click:/root/qtri/u540_s${tag}.sql || { echo SCPFAIL; exit 1; }
start=$(date +%s.%N)
ssh root@zebreus.click "/root/sq.sh < /root/qtri/u540_s${tag}.sql" > $d/s${tag}.json 2> $d/s${tag}.err
echo "EXIT=$?"
end=$(date +%s.%N); echo "secs=$(echo "$end-$start"|bc)"
cat $d/s${tag}.json; echo; cat $d/s${tag}.err
