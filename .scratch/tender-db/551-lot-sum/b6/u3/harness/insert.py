"""Issue 551 B6 U3: insert or remove the TEMPORARY harness in crates/ (exact, asserted edits; never git checkout, so a
co-worker's uncommitted hunk in another file is never touched).

    python3 -I -B insert.py insert     add the two `mod u3_harness;` blocks and copy the two harness files in
    python3 -I -B insert.py remove     take both blocks out again and delete the two copied files

Each edit asserts its anchor occurs exactly once; `remove` asserts the inserted block is present exactly once and the
copied file is byte-identical to the record copy before deleting it.
"""
import os
import shutil
import sys

REPO = '/home/user/tender-db'
HERE = os.path.dirname(os.path.abspath(__file__))
EDITS = [
    (os.path.join(REPO, 'crates/store/src/canonical.rs'),
     '#[cfg(test)]\n#[path = "canonical_551_v2_fixtures.rs"]\nmod lot_sum_551_v2_fixtures;\n',
     '#[cfg(test)]\n#[path = "canonical_u3_harness.rs"]\nmod u3_harness;\n',
     'after'),
    (os.path.join(REPO, 'crates/ingest/src/project.rs'),
     '\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    // ------------------------------------------------ issue 539: DÖE published_at\n',
     '#[cfg(test)]\n#[path = "project_u3_harness.rs"]\nmod u3_harness;\n',
     'before'),
]
FILES = [
    (os.path.join(HERE, 'canonical_u3_harness.rs'), os.path.join(REPO, 'crates/store/src/canonical_u3_harness.rs')),
    (os.path.join(HERE, 'project_u3_harness.rs'), os.path.join(REPO, 'crates/ingest/src/project_u3_harness.rs')),
]


def joined(anchor, block, where):
    # 'after': anchor then block; 'before': the block goes right after the anchor's leading newline
    return anchor + block if where == 'after' else '\n' + block + anchor[1:]


def insert():
    for path, anchor, block, where in EDITS:
        s = open(path, encoding='utf-8').read()
        assert s.count(anchor) == 1, (path, 'anchor count', s.count(anchor))
        assert block not in s, (path, 'already inserted')
        s = s.replace(anchor, joined(anchor, block, where))
        open(path, 'w', encoding='utf-8').write(s)
    for src, dst in FILES:
        assert not os.path.exists(dst), dst
        shutil.copyfile(src, dst)
    print('inserted')


def remove():
    for path, anchor, block, where in EDITS:
        s = open(path, encoding='utf-8').read()
        j = joined(anchor, block, where)
        assert s.count(j) == 1, (path, 'inserted block count', s.count(j))
        s = s.replace(j, anchor)
        open(path, 'w', encoding='utf-8').write(s)
    for src, dst in FILES:
        assert open(src, 'rb').read() == open(dst, 'rb').read(), (dst, 'differs from the record copy')
        os.remove(dst)
    print('removed')


if __name__ == '__main__':
    {'insert': insert, 'remove': remove}[sys.argv[1]]()
