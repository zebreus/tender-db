"""Export every ctx.pkl record as the tab-separated line format the Rust port check reads."""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import merge_rule as M
E = M.E
ctx = M.load_ctx()
from urllib.parse import quote
def s(x):
    """Percent-encoded (every byte outside [A-Za-z0-9_.~-]), so a key holding a tab survives; '-' is None."""
    if x is None: return "-"
    x = quote(str(x), safe="")
    return "%2D" if x == "-" else x
# port_in.tsv is generated (3.8 MB), not committed: regenerate it here, or name another path.
out = open(sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "port_in.tsv"), "w")
for tid in sorted(ctx):
    e = ctx[tid]; keys = e["keys"]
    versions = [E.version_from_rows(v.get("amounts"), v.get("lots"), v.get("results"), keys) for v in e["chain"]]
    versions.append(E.version_from_rows(e["head"]["amounts"], e["head"]["lots"], e["head"]["results"], keys))
    out.write(f"T\t{tid}\n")
    for c, r in sorted(M.rates_of(e).items()):
        out.write(f"R\t{c}\t{r!r}\n")
    for v in versions:
        out.write("V\n")
        for f in v.facts:
            out.write(f"F\t{s(f.field)}\t{f.cents}\t{s(f.currency)}\t{s(f.quality)}\n")
        for l in v.lots:
            out.write(f"L\t{s(l.key)}\t{s(l.kind)}\n")
            for f in l.facts:
                out.write(f"LF\t{s(f.field)}\t{f.cents}\t{s(f.currency)}\t{s(f.quality)}\n")
        for r in v.rounds:
            out.write(f"N\t{r.notice_id}\n")
            for x in r.lot_results:
                out.write(f"A\t{s(x.key if x.key is not None else 'RES')}\t{s(x.lot_key)}\t{s(x.cents)}\t{s(x.currency)}\n")
    out.write("E\n")
out.close()
print("ok", len(ctx))
