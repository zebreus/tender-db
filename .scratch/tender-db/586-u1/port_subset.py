"""586 unit 2: the stratified subset of fixtures/vectors.jsonl the Rust port check embeds.

    python3 -I port_subset.py fixtures/vectors.jsonl crates/ingest/tests/fixtures/touchpoint_586/vectors.jsonl

vectors.jsonl (all 3,751 saved pairs, 452 KB) is over the 200 KB the build embeds, so the committed fixture holds:
every row final.py aliases (tier A or C, 867), every row its gate drops (92), and 200 of the remaining minted rows,
the first 200 by sha256 of the salt `586-u2-port` and the row's JSON. Rows keep vectors.jsonl's order (by org id).
The full file stays checkable: `touchpoint_names_586`'s ignored test reads it from here.
"""
import hashlib
import json
import sys

SALT = '586-u2-port'


def main(src, dst):
    lines = [line for line in open(src, encoding='utf-8') if line.strip()]
    rows = [json.loads(line) for line in lines]
    rest = [i for i, r in enumerate(rows) if r[2] is None and not r[3]]
    salted = sorted(rest, key=lambda i: hashlib.sha256((SALT + lines[i].rstrip('\n')).encode('utf-8')).hexdigest())
    keep = {i for i, r in enumerate(rows) if r[2] is not None or r[3]} | set(salted[:200])
    with open(dst, 'w', encoding='utf-8') as f:
        for i in sorted(keep):
            f.write(lines[i] if lines[i].endswith('\n') else lines[i] + '\n')
    aliased = sum(1 for i in keep if rows[i][2] is not None)
    gated = sum(1 for i in keep if rows[i][3])
    print(f'{len(keep)} rows: {aliased} aliased, {gated} gated, {len(keep) - aliased - gated} salted minted')


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
