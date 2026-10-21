# companion to shadow.py: EXTRA/direct (non-alias) entries that shadow an inventory-declared field
import runpy, sys, re
from collections import defaultdict
sys.argv = ["shadow.py", sys.argv[1]]
import io, contextlib
buf = io.StringIO()
with contextlib.redirect_stdout(buf):
    g = runpy.run_path("shadow.py")
rep = defaultdict(set)
alias_decl = defaultdict(set)
for ver, fn in g["ACCEPTED"]:
    import json
    fields = json.load(open(f"{sys.argv[1]}/crates/ingest/sdk/{fn}"))["fields"]
    root = g["build"](fields, ver)
    for fid, fprov, pf, gid, gprov, pg in g["shadows"](root):
        if gprov == "declared" and fprov in ("extra", "direct"):
            rep[(fid, fprov, gid, pg)].add(ver)
        if gprov == "declared" and fprov.startswith("alias"):
            alias_decl[(fid, gid, pg)].add(ver)
print("== EXTRA/direct entry shadows an inventory-declared field ==")
for (fid, fprov, gid, pg), vers in sorted(rep.items()):
    print(f"- {fid} ({fprov}) over {gid} at {pg} :: {','.join(sorted(vers))}")
print(len(rep), "rows")
print("\n== alias graft shadows an inventory-declared field: per version count ==")
per = defaultdict(int)
for k, vers in alias_decl.items():
    for v in vers:
        per[v] += 1
for v, n in sorted(per.items()):
    print(v, n)
print(len(alias_decl), "distinct (winner, loser, path) rows")
