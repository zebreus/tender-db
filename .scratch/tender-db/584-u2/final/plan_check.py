"""plan_check.py <rehoming-plan report as saved from `/root/aj.sh /admin/reports/rehoming-plan`> <cohort.json>
Compares the dry plan (stored by apply-rehoming dry, supervisor.rs ApplyRehoming arm) to the cohort's rehome
tuples (notice, section, from=org_id, to=target_org_id), the same tuples the wet run's T4 ladder compares.
Exit 0 only when plan == cohort exactly (no extra, no missing, same destinations)."""
import json, sys
rep = json.load(open(sys.argv[1]))
body = json.loads(rep['body']) if isinstance(rep.get('body'), str) else rep  # the report wraps the plan as a JSON string
coh = json.load(open(sys.argv[2]))
want = {(v['notice_id'], v['section_id'], v['org_id'], v['target_org_id'])
        for v in coh['verdicts'] if v['action'] == 'rehome' and v['confidence'] == 'high' and v['target_org_id'] is not None}
got = [(m['notice'], m['section'], m['from'], m['to']) for m in body['plan']]
gs = set(got)
extra, missing = sorted(gs - want), sorted(want - gs)
print(json.dumps({k: body.get(k) for k in ('pending', 'eligible', 'moves', 'noop', 'missing_target')}),
      '| computed_at', rep.get('computed_at'), 'age_seconds', rep.get('age_seconds'))
print(f'cohort rehome tuples {len(want)}; plan tuples {len(got)} (distinct {len(gs)}); extra {len(extra)}; missing {len(missing)}')
for t in extra: print('  EXTRA (another cohort, or a changed destination):', t)
for t in missing: print('  MISSING (no-op, missing target, or already applied):', t)
ok = not extra and not missing and len(got) == len(gs) == len(want)
print('PLAN == COHORT' if ok else 'PLAN != COHORT -- do not run the wet job on this plan')
sys.exit(0 if ok else 1)
