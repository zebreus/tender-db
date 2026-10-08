"""Diff two data-quality section-16 extracts (the >= EUR 10 bn head band) by tender id.

    python3 section16_diff.py <before.txt> <after.txt>

Prints the tenders that left the band, the ones that joined, and the ones whose head notice,
published figure or EUR value moved. The EUR column moves with the rate date for any tender whose
version was re-derived, so a change there alone is listed separately from a head/published change."""
import re, sys

ROW = re.compile(r"^\s+(\d+)\s{2,}(.+?)\s{2,}(\S+)\s+([\d,]+\.\d\d)\s+([\d,]+\.\d\d)\s")


def rows(path):
    out, on = {}, False
    for line in open(path, encoding="utf-8"):
        if line.startswith("== 16."):
            on = True
            continue
        if on and line.startswith("== "):
            break
        m = ROW.match(line) if on else None
        if m:
            out[int(m.group(1))] = (m.group(2).strip(), m.group(3), m.group(4), m.group(5))
    return out


before, after = rows(sys.argv[1]), rows(sys.argv[2])
left = sorted(set(before) - set(after))
joined = sorted(set(after) - set(before))
moved = sorted(t for t in set(before) & set(after) if before[t][1:3] != after[t][1:3])
eur_only = sorted(t for t in set(before) & set(after) if before[t][1:3] == after[t][1:3] and before[t][3] != after[t][3])
print(f"before {len(before)} rows, after {len(after)} rows")
print(f"left the band ({len(left)}):")
for t in left:
    print("  -", t, before[t])
print(f"joined the band ({len(joined)}):")
for t in joined:
    print("  +", t, after[t])
print(f"head notice or published figure moved ({len(moved)}):")
for t in moved:
    print("  ~", t, before[t], "->", after[t])
print(f"EUR value only moved ({len(eur_only)})")
