"""Finalizer: validate cohort.json against admin.rs record_rehoming + store rules and the challenger's c1 read."""
import json, sys, hashlib, collections, pathlib
base = pathlib.Path(sys.argv[1])
raw = (base / "cohort.json").read_bytes()
c = json.loads(raw)
errs = []
# --- handler (crates/app/src/admin.rs:96-157) + serde shape (RehomingBody / RehomingIn) ---
if set(c) != {"cohort", "verdicts"}:
    errs.append(f"top-level keys {sorted(c)}")
if not isinstance(c.get("cohort"), str) or not c["cohort"]:
    errs.append("cohort empty / not a string")
vs = c.get("verdicts") or []
if not vs:
    errs.append("verdicts empty")
req = {"org_id": int, "notice_id": int, "section_id": str, "action": str, "rationale": str, "confidence": str}
opt = {"target_org_id": (int, type(None)), "target_name": (str, type(None))}
keys = collections.Counter()
for v in vs:
    k = f"{v.get('notice_id')}/{v.get('section_id')}"
    for f, t in req.items():
        if f not in v or not isinstance(v[f], t) or isinstance(v[f], bool):
            errs.append(f"{k}: field {f} missing or not {t.__name__}")
    for f, t in opt.items():
        if f in v and not isinstance(v[f], t):
            errs.append(f"{k}: {f} wrong type")
    extra = set(v) - set(req) - set(opt)
    if extra:
        errs.append(f"{k}: extra fields {extra} (serde ignores, but flag)")
    if v.get("confidence") not in ("high", "medium", "low"):
        errs.append(f"{k}: confidence")
    if v.get("action") not in ("rehome", "keep"):
        errs.append(f"{k}: action")
    if v.get("action") == "rehome" and v.get("target_org_id") is None and v.get("target_name") is None:
        errs.append(f"{k}: rehome without target")
    # apply_rehoming eligibility (canonical.rs:33847-33880): rehome+high+target_org_id
    if v.get("action") == "rehome" and (v.get("confidence") != "high" or v.get("target_org_id") is None):
        errs.append(f"{k}: rehome not eligible (needs high + target_org_id)")
    if v.get("action") == "keep" and v.get("target_org_id") is not None:
        errs.append(f"{k}: keep carries a target_org_id")
    if v.get("action") == "rehome" and v.get("target_org_id") == v.get("org_id"):
        errs.append(f"{k}: target == case org (no-op)")
    if not str(v.get("rationale", "")).strip():
        errs.append(f"{k}: empty rationale")
    if v.get("org_id") != 49:
        errs.append(f"{k}: org_id {v.get('org_id')} != 49")
    if not str(v.get("section_id", "")).startswith("ORG-"):
        errs.append(f"{k}: section id shape")
    keys[(v.get("notice_id"), v.get("section_id"))] += 1
dups = [k for k, n in keys.items() if n > 1]
if dups:
    errs.append(f"duplicate PK (notice, section): {dups}")
# --- against unit 1 (38 rows [mention_id, notice, section, name]) ---
u1 = json.loads((base.parent / "584-u1" / "org49_foreign.json").read_text())
u1k = {(r[1], r[2]) for r in u1}
if u1k != set(keys):
    errs.append(f"cohort keys != unit-1 keys: only-cohort {set(keys)-u1k} only-u1 {u1k-set(keys)}")
# --- against challenger's c1 live read (mentions on 49; targets exist, names) ---
c1 = json.loads((base / "challenge-raw/sql/c1_verify.out").read_text())
m = {(r[1], r[2]): r for r in c1["rows"] if r[0] == "m"}
o = {r[1]: r for r in c1["rows"] if r[0] == "o"}
for v in vs:
    k = (v["notice_id"], v["section_id"])
    if k not in m:
        errs.append(f"{k}: not in c1"); continue
    if m[k][3] != 49:
        errs.append(f"{k}: mention sits on {m[k][3]}, not 49 (would be a no-op)")
    if v["action"] == "rehome":
        t = v["target_org_id"]
        if t not in o:
            errs.append(f"{k}: target {t} not read in c1")
        elif o[t][4] != v["target_name"]:
            errs.append(f"{k}: target_name {v['target_name']!r} != live head {o[t][4]!r}")
lens = {f: max(len(v[f] or "") for v in vs) for f in ("section_id", "rationale", "target_name") if True}
cnt = collections.Counter(v["action"] for v in vs)
tgt = collections.Counter(v["target_org_id"] for v in vs if v["action"] == "rehome")
print(json.dumps({
    "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw), "cohort": c.get("cohort"),
    "verdicts": len(vs), "actions": dict(cnt), "confidence": dict(collections.Counter(v["confidence"] for v in vs)),
    "targets": len(tgt), "target_counts": {str(k): n for k, n in sorted(tgt.items(), key=lambda x: -x[1])},
    "max_len": lens, "errors": errs,
}, ensure_ascii=False, indent=1))
sys.exit(1 if errs else 0)
