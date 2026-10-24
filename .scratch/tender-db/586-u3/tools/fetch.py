"""fetch.py: serial GETs (via tools/get.sh, >= 2.2 s apart) for every pick in picks.json: the tender, the organization,
and the notice content of the party's mention notice; plus the extra Verify organizations. Skips files already fetched."""
import json, os, subprocess
HERE = '/home/user/tender-db/.scratch/tender-db/586-u3'
GET = os.path.join(HERE, 'tools', 'get.sh')
picks = json.load(open(os.path.join(HERE, 'picks.json')))
jobs = []
EXTRA_ORGS = [31975096, 31975008, 31975059, 31975006, 31975019, 31975180, 31975009, 31975004, 31975568, 31975060, 31975949]
for o in EXTRA_ORGS:
    jobs.append((f'/v1/organizations/{o}', f'raw/org/{o}.json'))
for part in ('minted', 'aliased'):
    for p in picks[part]['picks']:
        jobs.append((f"/v1/tenders/{p['tender_id']}", f"raw/tender/{p['tender_id']}.json"))
        jobs.append((f"/v1/organizations/{p['org_id']}", f"raw/org/{p['org_id']}.json"))
        jobs.append((f"/v1/notices/{p['notice_id']}/content", f"raw/content/{p['notice_id']}.json"))
done = set()
for path, out in jobs:
    full = os.path.join(HERE, out)
    if out in done or (os.path.exists(full) and os.path.getsize(full) > 0):
        done.add(out); continue
    r = subprocess.run([GET, path, full], capture_output=True, text=True)
    print(path, r.stdout.strip(), r.stderr.strip()[:200], flush=True)
    done.add(out)
print('DONE', len(done))
