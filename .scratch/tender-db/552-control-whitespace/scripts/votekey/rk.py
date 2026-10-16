"""Thin wrapper over the standalone Rust binary `vk` (verbatim head_vote_key + helpers)."""
import subprocess, os
HERE = os.path.dirname(os.path.abspath(__file__))
_cache = {}
def rust(strings):
    todo = [s for s in dict.fromkeys(strings) if s not in _cache]
    if todo:
        inp = '\n'.join(s.encode('utf-8').hex() for s in todo) + '\n'
        out = subprocess.run([os.path.join(HERE, 'vk')], input=inp, capture_output=True, text=True, check=True).stdout
        for s, line in zip(todo, out.splitlines()):
            f = line.split('\t')
            _cache[s] = dict(key=bytes.fromhex(f[0]).decode(), ctrl=f[1] == 'true', cctrl=f[2] == 'true',
                             mojibake=f[3] == 'true', clen=int(f[4]), lower=f[5] == 'true', consortium=f[6] == 'true',
                             collapsed=bytes.fromhex(f[7]).decode())
    return [_cache[s] for s in strings]
def vkey(s):
    return rust([s])[0]['key']
if __name__ == '__main__':
    for s in ['WECO TMC S.R.L.', 'WECO TMC SRL', 'The Acme Ltd', 'Szkoła Podstawowa nr 13 z Oddziałami', 'J & M Smith Ltd',
              'Sp. z o.o.', 'Acme\nGmbH', "Ministero dell'Interno", 'ΔΗΜΟΣ ΑΒΔΗΡΩΝ']:
        print(repr(s), '->', repr(vkey(s)))
