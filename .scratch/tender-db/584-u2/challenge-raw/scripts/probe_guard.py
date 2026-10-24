import sys, runpy, io, contextlib
sys.argv = ['guard_sim.py', '/home/user/tender-db/.scratch/tender-db/584-u1/org49_mentions.json']
buf = io.StringIO()
with contextlib.redirect_stdout(buf):
    g = runpy.run_path('/home/user/tender-db/.scratch/tender-db/584-u2/scripts/guard_sim.py')
D = g['tests']['D C + vocabulary (families >= 1% of mentions)']
for n in ['KRAJOWA IZBA GOSPODARCZA', 'Krajowa Izba Kontroli', 'Urząd Miasta Szubin', 'Urząd Gminy Wojcieszków',
          'Gmina Skaryszew', 'Ministerstwo Finansów', 'Platforma eZamówienia', 'STANIMEX SPÓŁKA JAWNA S. STANISZEWSKI, M. STANISZEWSKI',
          'METALZBYT-HURT Sp. z o.o.', 'Ever Cleaning Sp. z o.o.', 'Bank Gospodarstwa Krajowego', 'Krajowy Ośrodek Wsparcia Rolnictwa',
          'Publiczna Szkoła Podstawowa nr 1', 'Krajowa Spółka Cukrowa S.A.']:
    print('PASS (binds KIO)' if D(n) else 'refused        ', '|', n)
