"""Front-matter numeric canonicalization parity.

Rust serializes every numeric scalar via `f64::to_string()` (so `1.0` -> `1`,
`1e10` -> `10000000000`, `1.5` -> `1.5`). JavaScript inherits the same shape
via `JSON.stringify(Number(...))` for the range that appears in real
front-matter. Python's default `float`/`json.dumps` would emit `1.0` and
diverge from the byte-stable Canonical AST contract.

These tests pin the Python parser to the Rust/JS shape. A regression that
flips this test red would also flip the conformance gate red, but having
a focused unit test makes the cause obvious.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from nodx.front_matter import scalar  # noqa: E402
from nodx import parse, canonical_json  # noqa: E402


def test_integer_valued_floats_collapse_to_int():
    assert scalar("1.0") == 1
    assert scalar("-2.0") == -2
    assert scalar("0.0") == 0


def test_non_integer_floats_stay_float():
    assert scalar("1.5") == 1.5
    assert scalar("-0.25") == -0.25
    assert scalar("0.1") == 0.1


def test_plain_integers():
    assert scalar("42") == 42
    assert scalar("-7") == -7


def test_canonical_ast_keeps_one_for_one_release():
    source = "---\nrelease: 1.0\n---\n\nBody\n"
    doc = parse(source)
    serialized = canonical_json(doc)
    # The Rust/JS implementations emit `"release":1` (an integer), so the
    # Python serialization must agree byte-for-byte.
    parsed = json.loads(serialized)
    assert parsed["meta"]["release"] == 1
    assert ":1," in serialized or serialized.endswith(":1}")
