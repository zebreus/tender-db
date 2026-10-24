#!/usr/bin/env python3
"""W7 (issue 551) helper: every field-id-shaped string literal in the fold's PRODUCTION code
(crates/ingest/src/project.rs minus its #[cfg(test)] items, and the production halves of the
project/ submodules), with line number and the enclosing const/fn. Read-only.
Usage: python3 -I scan_fold_literals.py <repo-root>
"""
import re, sys
ROOT = sys.argv[1]
ID = re.compile(r'"((?:BT|OPT|OPP|OPA|DE1|UBL|SDK01|TED|TXT)-[^"\s]*)"')
ITEM = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static|fn|async fn|struct|enum|impl)\s+([A-Za-z_0-9<>]+)')

def prod_lines(path, end=None, skip_items=()):
    lines = open(path).read().split("\n")
    if end:
        lines = lines[: end - 1]
    out = []
    skip = False
    depth = 0
    cur = "?"
    for i, l in enumerate(lines, 1):
        m = ITEM.match(l)
        if m and not l.startswith(" " * 8):
            cur = m.group(1)
        out.append((i, l, cur))
    return out

def tests_mod_line(path):
    lines = open(path).read().split("\n")
    for i, l in enumerate(lines, 1):
        if l.startswith("mod tests {") and lines[i - 2].strip() == "#[cfg(test)]":
            return i - 1
    return None

FILES = [
    ("crates/ingest/src/project.rs", tests_mod_line(f"{ROOT}/crates/ingest/src/project.rs")),
    ("crates/ingest/src/project/guid_twin.rs", 495),
    ("crates/ingest/src/project/key_census.rs", 489),
    ("crates/ingest/src/project/role_census.rs", 1285),
    ("crates/ingest/src/project/escapes.rs", 161),
    ("crates/ingest/src/project/title_key_nfkd.rs", None),
]
# #[cfg(test)] items inside project.rs's production range (start line of the item, end line)
TEST_ITEMS = []
src = open(f"{ROOT}/crates/ingest/src/project.rs").read().split("\n")
for i, l in enumerate(src[: FILES[0][1] - 1], 1):
    if l.strip() == "#[cfg(test)]":
        nxt = src[i]  # line i+1 (0-based i)
        if nxt.startswith("mod "):
            continue
        # find the end: first line that is exactly "];" or "}" at column 0 after it
        j = i + 1
        while j <= len(src) and src[j - 1] not in ("];", "}"):
            j += 1
        TEST_ITEMS.append((i + 1, j))
for path, end in FILES:
    for i, l, cur in prod_lines(f"{ROOT}/{path}", end):
        if path.endswith("project.rs") and any(a <= i <= b for a, b in TEST_ITEMS):
            continue
        s = l.strip()
        if s.startswith("//"):
            continue
        code = l.split("//")[0] if "//" in l and '"' not in l.split("//")[0][-1:] else l
        for m in ID.finditer(code):
            print(f"{path.split('/')[-1]}:{i}\t{cur}\t{m.group(1)}")
print("# test items skipped in project.rs:", TEST_ITEMS, file=sys.stderr)
