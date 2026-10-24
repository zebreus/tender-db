#!/usr/bin/env python3
"""519 M6: serial public-API GETs (https://tenders.zebreus.click/v1/...), at most one per 2 s, cached
under api/<tag>.json with the URL. usage: api.py TAG PATH_AND_QUERY [TAG PATH ...]"""
import json, os, subprocess, sys, time, urllib.parse
D = '/home/user/tender-db/.scratch/tender-db/519-m6'
STAMP = os.path.join(D, 'api', '.last')
args = sys.argv[1:]
for tag, path in zip(args[::2], args[1::2]):
    out = os.path.join(D, 'api', tag + '.json')
    if os.path.exists(out):
        print(tag, 'cached'); continue
    last = float(open(STAMP).read()) if os.path.exists(STAMP) else 0
    wait = 2.1 - (time.time() - last)
    if wait > 0: time.sleep(wait)
    url = 'https://tenders.zebreus.click' + path
    p = subprocess.run(['curl', '-sS', '--max-time', '30', '-w', '\n%{http_code}', url], capture_output=True, text=True)
    open(STAMP, 'w').write(str(time.time()))
    body, _, code = p.stdout.rpartition('\n')
    try: parsed = json.loads(body)
    except ValueError: parsed = body[:2000]
    json.dump({'url': url, 'http': code, 'fetched': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'body': parsed}, open(out, 'w'), indent=1)
    print(tag, code, url)
