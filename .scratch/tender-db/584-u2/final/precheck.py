"""precheck.py <`/root/aj.sh "/admin/case-reviews?table=rehoming&limit=5000"` output> <cohort.json>
Before (or right after) the POST: which OTHER pending verdicts will the dry plan also carry, and is any
cohort key already stamped (a stamped real move is never cleared by a re-POST: canonical.rs:33735-33739)."""
import json, sys
d = json.load(open(sys.argv[1]))
coh = json.load(open(sys.argv[2]))
name = coh['cohort']
keys = {(v['notice_id'], v['section_id']) for v in coh['verdicts']}
rows = d['rows']
if d.get('count', len(rows)) >= d.get('limit', 10**9):
    print(f"WARNING: listing hit its limit ({d.get('limit')}); raise it (max 5000) or the counts below are partial")
pend = [r for r in rows if r.get('applied_at') is None]
elig_other = [r for r in pend if r['cohort'] != name and r['action'] == 'rehome' and r['confidence'] == 'high'
              and r.get('target_org_id') is not None]
mine = [r for r in rows if (r['notice_id'], r['section_id']) in keys]
stamped = [r for r in mine if r.get('applied_at') is not None]
other_cohort = [r for r in mine if r['cohort'] != name]
print(f'table rows {len(rows)}; pending {len(pend)}; pending high rehomes with a target OUTSIDE {name}: {len(elig_other)}')
for r in elig_other: print('  will also be planned:', r['cohort'], r['notice_id'], r['section_id'], r['case_org_id'], '->', r['target_org_id'])
print(f'cohort keys present {len(mine)} of {len(keys)}; under another cohort name {len(other_cohort)}; already stamped {len(stamped)}')
for r in stamped: print('  STAMPED (will not be planned):', r['cohort'], r['notice_id'], r['section_id'], r['applied_action'])
pend_missing_target = [r for r in pend if r['action'] == 'rehome' and r['confidence'] == 'high' and r.get('target_org_id') is None]
print(f'expected dry counts: pending {len(pend) + (0 if mine else len(keys))} (if POSTed after this read), '
      f'eligible {len(elig_other) + 32}, missing_target >= {len(pend_missing_target)}')
