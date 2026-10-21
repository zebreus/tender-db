"""Issue 551 B6 U6 step 0 (critic A5): refresh the port's rates AFTER the 10-21 chain's fetch-rates, keeping the old set.

    python3 -I -B rates_refresh.py --after-chain-fetch-rates-done [--zip FILE] [--expect-latest YYYY-MM-DD]

  - new set dir: scratchpad m551/rates-<UTC yyyymmddThhmmssZ>/ (the old m551/rates is never touched);
  - the two Eurostat files (ecu-ert_h_eur_d.csv, ecu-ert_bil_eur_d.csv: pre-1999 series, frozen) are COPIED from the old
    set byte for byte (sha256 asserted equal);
  - eurofxref-hist.zip is downloaded from the ECB into a fresh empty quarantine dir (untrusted download; nothing in it is
    executed), checked (a zip of exactly one CSV whose header starts with Date and lists USD/GBP/PLN/HUF), then moved in.
    --zip FILE uses an already-downloaded file instead (same checks; offline tests use this);
  - it reads the newest fixing date in the CSV. The store's fetch-rates this morning loaded the ECB file up to
    YESTERDAY's fixing (operations.md: "up to yesterday's fixing"); --expect-latest (default: yesterday, UTC) must equal
    it. A newer fixing (the ECB publishes ~14:00Z) means the port has a rate the store does not: recorded as a
    deviation (exit 2) -- re-run before 14:00Z, or accept it explicitly with --accept-newer (affects only a head
    published today, and only its S_eur);
  - writes u6/rates.json: old dir + sha256 of its three files, new dir + sha256 of its three files, the newest fixing,
    the download URL/time/bytes. RULE_G_RATES_DIR=<new dir> is what predict.py passes to the port.
Exit 0 ok; 2 check failed (nothing pointed at the new set); 3 refused (flag missing).
"""
import argparse
import datetime
import os
import shutil
import subprocess
import sys
import time
import zipfile

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kitlib as K  # noqa: E402

OLD = os.path.join(K.SP, 'm551', 'rates')
ECB = 'https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist.zip'
ERT = ('ecu-ert_h_eur_d.csv', 'ecu-ert_bil_eur_d.csv')
OLD_SHA = {'eurofxref-hist.zip': '8e925cef1572f8df5c8743229a761b029180f3b5cd4aa2213592192964ebd793',
           'ecu-ert_h_eur_d.csv': '47e941270ac4a0f0f22416bd4b41c9b427d43c3a4f430996f8457dca8f134e5c',
           'ecu-ert_bil_eur_d.csv': '48e2c9f17ae7219b2b145a66ace399aba7c306c88721b7ce8f7c76f4758e131b'}


def check_zip(path):
    """-> (newest fixing 'YYYY-MM-DD', n_rows). Raises ValueError on anything that is not the ECB history file."""
    with zipfile.ZipFile(path) as z:
        names = z.namelist()
        if len(names) != 1 or not names[0].lower().endswith('.csv') or '/' in names[0] or '..' in names[0]:
            raise ValueError(f'unexpected members {names}')
        info = z.getinfo(names[0])
        if info.file_size > 50 * 1024 * 1024:
            raise ValueError(f'member too large: {info.file_size}')
        csv = z.read(names[0]).decode('utf-8')
    lines = [ln for ln in csv.splitlines() if ln.strip()]
    head = [c.strip() for c in lines[0].split(',')]
    if head[0] != 'Date' or not {'USD', 'GBP', 'PLN', 'HUF'} <= set(head):
        raise ValueError(f'unexpected header {head[:8]}')
    dates = []
    for ln in lines[1:]:
        d = ln.split(',', 1)[0].strip()
        datetime.date.fromisoformat(d)
        dates.append(d)
    return max(dates), len(dates)


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--after-chain-fetch-rates-done', action='store_true',
                    help='confirm the 10-21 chain (fetch-rates, rederive-eur-recent, project) shows done on /health')
    ap.add_argument('--zip')
    ap.add_argument('--expect-latest')
    ap.add_argument('--accept-newer', action='store_true')
    ap.add_argument('--out', default=os.path.join(K.U6, 'rates.json'))
    ap.add_argument('--new-root', default=os.path.join(K.SP, 'm551'))
    a = ap.parse_args(argv)
    if not a.after_chain_fetch_rates_done:
        print('REFUSED: pass --after-chain-fetch-rates-done once the chain has finished (A5: refresh AFTER fetch-rates)')
        return 3
    old_sha = {f: K.sha_file(os.path.join(OLD, f)) for f in sorted(os.listdir(OLD))}
    if old_sha != OLD_SHA:
        print(f'FAIL: the old set changed: {old_sha}')
        return 2
    stamp = time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())
    new = os.path.join(a.new_root, f'rates-{stamp}')
    quarantine = os.path.join(K.SP, 'b6u6kit', f'ecb-dl-{stamp}')
    os.makedirs(quarantine, exist_ok=False)
    dl = {'url': None, 'at': K.now_iso()}
    zpath = os.path.join(quarantine, 'eurofxref-hist.zip')
    if a.zip:
        shutil.copyfile(a.zip, zpath)
        dl['source_file'] = os.path.abspath(a.zip)
    else:
        dl['url'] = ECB
        p = subprocess.run(['curl', '-sS', '--fail', '--max-time', '120', '-o', zpath, ECB], capture_output=True,
                           text=True)
        if p.returncode != 0:
            print(f'FAIL: download rc={p.returncode} {p.stderr[:300]}')
            return 2
    try:
        latest, n = check_zip(zpath)
    except (ValueError, zipfile.BadZipFile, UnicodeDecodeError) as e:
        print(f'FAIL: the downloaded file is not the ECB history zip: {e}')
        return 2
    dl.update(bytes=os.path.getsize(zpath), sha256=K.sha_file(zpath), csv_rows=n, newest_fixing=latest)
    expect = a.expect_latest or (datetime.datetime.now(datetime.timezone.utc).date()
                                 - datetime.timedelta(days=1)).isoformat()
    status = 0
    note = None
    if latest < expect:
        print(f'FAIL: newest fixing {latest} is older than {expect} (the store loaded up to {expect} this morning)')
        return 2
    if latest > expect:
        note = (f'newest fixing {latest} is NEWER than the store\'s {expect}: a head published on {latest} gets a rate '
                'the store does not have (S_eur only)')
        if not a.accept_newer:
            print('DEVIATION: ' + note + ' -- re-run before ~14:00Z or pass --accept-newer')
            status = 2
    os.makedirs(new, exist_ok=False)
    for f in ERT:
        shutil.copyfile(os.path.join(OLD, f), os.path.join(new, f))
    shutil.move(zpath, os.path.join(new, 'eurofxref-hist.zip'))
    os.rmdir(quarantine)
    new_sha = {f: K.sha_file(os.path.join(new, f)) for f in sorted(os.listdir(new))}
    assert sorted(new_sha) == sorted(OLD_SHA), new_sha
    for f in ERT:
        assert new_sha[f] == OLD_SHA[f], (f, new_sha[f])
    rec = {'what': 'issue 551 B6 U6 step 0 (A5): the port rates set refreshed after the 10-21 chain fetch-rates',
           'old': {'dir': OLD, 'sha256': old_sha}, 'new': {'dir': new, 'sha256': new_sha}, 'download': dl,
           'store_loaded_up_to': expect, 'deviation': note, 'accepted_newer': bool(note and a.accept_newer),
           'RULE_G_RATES_DIR': new}
    if status == 0:
        K.jdump(a.out, rec)
        print(f'OK new rates set {new}')
    else:
        K.jdump(a.out + '.deviation.json', rec)
    for f, h in sorted(new_sha.items()):
        print(f'  {f} {h}')
    print(f'  newest fixing {latest} ({n} rows)')
    return status


if __name__ == '__main__':
    sys.exit(K.run_main(main))
