"""Write the packages that the self-driving workflow publishes into a folder,
so they can be checked by hand with `cloacinactl package validate` or built
with cargo.

    /usr/bin/python3 -I .angreal/test/bdd/suite/tests/render_targets.py OUT_DIR

Makes `OUT_DIR/standing-target-py`, `OUT_DIR/standing-target-rust` and
`OUT_DIR/standing-target-rust-broken` (the version with a build error).
"""

from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(os.path.dirname(HERE)))

from suite.tests.test_workload import load_fixture


def main(argv):
    if len(argv) != 1:
        print(__doc__)
        return 2
    driver = load_fixture("standing-driver-py", "standing_driver")
    wanted = (("standing-target-py", "py", False), ("standing-target-rust", "rust", False),
              ("standing-target-rust-broken", "rust", True))
    for folder, kind, broken in wanted:
        for rel, text in driver.target_files(kind, "0.1.0", broken=broken).items():
            path = os.path.join(argv[0], folder, *rel.split("/"))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(text)
        print(os.path.join(argv[0], folder))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
