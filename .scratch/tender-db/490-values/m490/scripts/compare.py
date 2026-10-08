"""Compare a naive SQL per-lot max with the REST lot value, per window, and attribute causes."""
import json, sys, os
from collections import Counter, defaultdict

IMPLAUSIBLE = 10_000_000_000_000
SCALE_MIN_EUR = 100_000_000_000
FLOOR = 1_000
MIN_K = 3

def repdigit_len(n):
    rep = n % 10; d = 0
    while n > 0:
        if n % 10 != rep: return 0
        n //= 10; d += 1
    return d

def sentinel_kind(c):
    if c < 0: return "negative"
    if c == 0: return "zero"
    if c in (1, 100): return "one-unit"
    if repdigit_len(c) >= 9:
        return "repdigit-nines" if c % 10 == 9 else "repdigit-other"
    if c % 100 != 0: return None
    if repdigit_len(c // 100) >= 9:
        return "repdigit-nines" if (c // 100) % 10 == 9 else "repdigit-other"
    return None

class Scale:
    def __init__(self):
        self.figures = set(); self.head = {}
    def add_partner(self, cur, c):
        if c > FLOOR: self.figures.add((cur, c))
    def add_head(self, field, cur, c):
        k = (cur, c)
        if k in self.head:
            if self.head[k] is not None and self.head[k] != field: self.head[k] = None
        else: self.head[k] = field
    def partner(self, cur, c):
        if c <= 0: return None
        for k in range(MIN_K, 19):
            p = 10 ** k
            if not (c // p > FLOOR): break
            if c % p == 0 and (cur, c // p) in self.figures: return c // p
        return None
    def refuses(self, field, cur, c, eur):
        if eur < SCALE_MIN_EUR or self.partner(cur, c) is None: return False
        h = self.head.get((cur, c), "MISSING")
        if h == "MISSING": corroborated = False
        elif h is None: corroborated = True
        else: corroborated = h != field
        return not corroborated

def main(name, d):
    s = json.load(open(os.path.join(d, f"{name}.sql.json")))
    r = json.load(open(os.path.join(d, f"{name}.rest.json")))
    rest = {it["id"]: it for it in r["items"]}
    cur = {row[0]: row[1] for row in s["tenders"]["rows"]}
    src = {row[0]: row[2] for row in s["tenders"]["rows"]}
    lot_key = {row[0]: row[2] for row in s["lots"]["rows"]}
    cur_lots = [(row[0], row[2]) for row in s["tvl"]["rows"] if cur.get(row[0]) == row[1]]
    amts = s["amounts"]["rows"]  # tender_id, seq, lot_id, field, cents, currency, eur_cents, quality
    by_ver = defaultdict(list)
    for a in amts: by_ver[(a[0], a[1])].append(a)
    lres = defaultdict(list)
    for x in s["lot_results"]["rows"]: lres[x[0]].append(x)  # tender_id, seq, cents, cur

    def scale_for(t, seq):
        rule = Scale()
        for a in amts:
            if a[0] == t and a[1] <= seq:
                rule.add_partner(a[5], a[4])
                if a[1] == seq: rule.add_head(a[3], a[5], a[4])
        for x in lres[t]:
            if x[1] <= seq and x[2] is not None and x[3] is not None:
                rule.add_partner(x[3], x[2])
        return rule

    stats = Counter(); causes_c = Counter(); causes_e = Counter(); examples = defaultdict(list)
    replic_mismatch = []
    tenders_c = defaultdict(set); tenders_e = defaultdict(set); tenders_ln = defaultdict(set); causes_ln = Counter()
    watch = set(int(x) for x in os.environ.get("WATCH", "").split(",") if x)
    for (t, lot) in sorted(cur_lots):
        seq = cur[t]
        stats["lots"] += 1
        it = rest.get(lot)
        if it is None:
            stats["missing_from_rest"] += 1; continue
        if it["version"] != seq:
            stats["version_skew"] += 1; continue
        stats["compared"] += 1
        rv = it["value"]
        rc = rv["cents"] if rv else None
        rcur = rv["currency"] if rv else None
        rows = [a for a in by_ver[(t, seq)] if a[2] == lot]
        tender_scoped = [a for a in by_ver[(t, seq)] if a[2] is None]
        if not rows:
            stats["no_lot_amount"] += 1
            if tender_scoped: stats["no_lot_amount_but_tender_amount"] += 1
            if rv is not None: stats["REST_value_without_lot_amount(fallback?)"] += 1
            continue
        stats["with_lot_amount"] += 1
        # replicate REST
        cands = []
        refused = {}
        for a in rows:
            if a[7] is not None: refused[id(a)] = "withheld"; continue
            sk = sentinel_kind(a[4])
            if sk: refused[id(a)] = "sentinel:" + sk; continue
            if a[6] is not None and a[6] > IMPLAUSIBLE: refused[id(a)] = "ceiling>100bn"; continue
            cands.append(a)
        if any(a[6] is not None and a[6] >= SCALE_MIN_EUR for a in cands):
            rule = scale_for(t, seq)
            keep = []
            for a in cands:
                if a[6] is not None and rule.refuses(a[3], a[5], a[4], a[6]):
                    refused[id(a)] = "scale-rule"
                else: keep.append(a)
            cands = keep
        best = None
        for a in cands:
            if best is None or a[4] > best[4]: best = a
        mine = (best[4], best[5]) if best else None
        if mine != ((rc, rcur) if rv else None):
            replic_mismatch.append((t, lot, mine, rv))
        # naive MAX(cents)
        ncents = max(a[4] for a in rows)
        top_c = [a for a in rows if a[4] == ncents]
        if ncents != rc:
            stats["disagree_maxcents"] += 1
            cause = refused.get(id(top_c[0]), "admitted?!")
            causes_c[cause] += 1
            tenders_c[cause].add(t)
            examples["maxcents:" + cause].append((t, lot_key.get(lot), lot, src[t], [(a[3], a[4], a[5], a[6], a[7]) for a in rows], rv))
        elif rv and any(a[5] != rcur for a in top_c):
            stats["maxcents_same_cents_other_currency"] += 1
        # less-naive MAX(eur_cents): quality IS NULL AND cents > 0
        ln = [a for a in rows if a[7] is None and a[4] > 0 and a[6] is not None]
        lneur = max(a[6] for a in ln) if ln else None
        if len({a[5] for a in rows}) > 1: stats["lots_mixed_currency"] += 1
        # naive MAX(eur_cents)
        eurs = [a[6] for a in rows if a[6] is not None]
        neur = max(eurs) if eurs else None
        rest_eur = None
        if best is not None and mine == ((rc, rcur) if rv else None):
            rest_eur = best[6]
        rest_eur_for_ln = rest_eur
        if lneur != rest_eur_for_ln:
            stats["disagree_lessnaive_eur(quality IS NULL AND cents>0)"] += 1
            top = [a for a in ln if a[6] == lneur][0] if lneur is not None else None
            c2 = refused.get(id(top), "other") if top is not None else "naive NULL"
            if top is not None and id(top) not in refused: c2 = "currency mix / unconvertible"
            causes_ln[c2] += 1; tenders_ln[c2].add(t)
        if t in watch:
            print("WATCH", t, lot_key.get(lot), lot, [(a[3], a[4], a[5], a[6], a[7]) for a in rows], "REST", rv, "naive_c", ncents, "naive_eur", neur)
        if neur != rest_eur:
            stats["disagree_maxeur"] += 1
            if neur is None:
                cause = "naive NULL (no convertible row); REST serves unconverted " + str(rcur)
            else:
                top = [a for a in rows if a[6] == neur][0]
                if id(top) in refused:
                    cause = refused[id(top)]
                elif rest_eur is None and rv is None:
                    cause = "admitted?!"
                elif rest_eur is None:
                    cause = "REST pick has no EUR (unconvertible currency)"
                else:
                    cause = "currency mix: REST ranks by published cents"
            causes_e[cause] += 1
            tenders_e[cause].add(t)
            examples["maxeur:" + cause].append((t, lot_key.get(lot), lot, src[t], [(a[3], a[4], a[5], a[6], a[7]) for a in rows], rv))
    print(f"== {name} window {s['window']}")
    for k, v in stats.items(): print(f"  {k}: {v}")
    print("  causes MAX(cents):", dict(causes_c))
    print("  causes MAX(eur_cents):", dict(causes_e))
    print("  distinct tenders MAX(cents):", {k: len(v) for k, v in tenders_c.items()})
    print("  distinct tenders MAX(eur):", {k: len(v) for k, v in tenders_e.items()})
    print("  causes less-naive:", dict(causes_ln), {k: len(v) for k, v in tenders_ln.items()})
    print("  replication mismatches (my REST rebuild != served):", len(replic_mismatch))
    for m in replic_mismatch[:10]: print("    ", m)
    for k, v in examples.items():
        print(f"  ex {k} ({len(v)}):")
        for e in v[:4]: print("    ", e)

main(sys.argv[1], sys.argv[2])
