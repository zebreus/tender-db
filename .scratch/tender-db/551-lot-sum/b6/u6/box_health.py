"""Issue 551 B6 kit: `SELECT 1` through the shared box state (clears the flag a failed read set; never a retry of it).

    python3 -I -B box_health.py
"""
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kitlib as K  # noqa: E402

def main():
    b = K.shared_box().health()
    print('health', 'OK' if b is not None else 'NO ANSWER (flag stays set; wait, then try again)')
    return 0 if b is not None else 2


if __name__ == '__main__':
    sys.exit(K.run_main(main))
