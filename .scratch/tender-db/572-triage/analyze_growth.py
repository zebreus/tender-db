# usage: python3 -I analyze_growth.py — critic amendment S1: the derivations behind "9-10 class mentions a month
# (NHC about 5)" and "about 20% of new P@A mentions arrive on an unreviewed key (Good-Turing)".
# Inputs: q4_class_refs.summary.json (the 191 class mentions) and q10_class_dates.out.json (published_at of their
# 190 notices). Writes growth.summary.json.
import json, os, re
from collections import Counter, defaultdict
D = os.path.dirname(os.path.abspath(__file__))
M = json.load(open(D + '/q4_class_refs.summary.json'))
import datetime
j = json.load(open(D + '/q10_class_dates.out.json'))
# published_at is unix seconds (canonical.rs:216); render as UTC ISO dates
pub = {r[0]: datetime.datetime.fromtimestamp(r[1], datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ') for r in j['rows']}
assert len(pub) == 190 and all(m['notice'] in pub for m in M)
def month(n): return pub[n][:7]
def norm(s): return ' '.join(s.lower().split())
months = Counter(month(m['notice']) for m in M)
first, last = min(pub.values())[:10], max(pub.values())[:10]
# span in months from the first class notice to 2026-10-19 (the measure date)
y0, m0 = int(first[:4]), int(first[5:7]); span = (2026 - y0) * 12 + (10 - m0) + 19 / 31
by_agent = Counter(m['agent'] for m in M)
recent = [m for m in M if pub[m['notice']] >= '2025-10-19']        # the last 12 months
recent6 = [m for m in M if pub[m['notice']] >= '2026-04-19']       # the last 6 months
# Good-Turing on P@A keys (agent org, normalised printed name): the chance that the NEXT P@A mention is on a key
# never seen before is estimated by N1 / N, N1 = keys seen exactly once, N = P@A mentions.
pa = [m for m in M if m['direction'] == 'P@A' and m['own_roles'] == ['OPT-300-Procedure-Buyer']]
keys = Counter((m['org'], norm(m['name'])) for m in pa)
n1 = sum(1 for v in keys.values() if v == 1)
# the same estimate per procedure-free year halves: keys first seen in the last 12 months
first_seen = {}
for m in sorted(pa, key=lambda m: pub[m['notice']]):
    first_seen.setdefault((m['org'], norm(m['name'])), pub[m['notice']])
pa_recent = [m for m in pa if pub[m['notice']] >= '2025-10-19']
new_recent = sum(1 for m in pa_recent if first_seen[(m['org'], norm(m['name']))] == pub[m['notice']])
out = dict(
    first_class_notice=first, last_class_notice=last, span_months=round(span, 1),
    mentions=len(M), per_month_whole_span=round(len(M) / span, 1),
    per_month_last_12=round(len(recent) / 12, 1), per_month_last_6=round(len(recent6) / 6, 1),
    nhc_per_month_whole_span=round(by_agent['NHC'] / span, 1),
    nhc_per_month_last_12=round(sum(m['agent'] == 'NHC' for m in recent) / 12, 1),
    by_month=dict(sorted(months.items())),
    pa_buyer_mentions=len(pa), pa_keys=len(keys), pa_keys_seen_once=n1, good_turing_unseen=round(n1 / len(pa), 3),
    pa_mentions_last_12=len(pa_recent), pa_last_12_on_a_key_first_seen_then=new_recent,
)
json.dump(out, open(D + '/growth.summary.json', 'w'), indent=1)
print(json.dumps(out, indent=1))
