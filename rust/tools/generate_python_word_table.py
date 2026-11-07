#!/usr/bin/env python3
"""Generate the pinned Unicode 15 Python `str.isalnum` table used by Rust."""

import sys
from pathlib import Path

if sys.version_info[:3] != (3, 12, 3):
    raise SystemExit(f"requires CPython 3.12.3; got {sys.version.split()[0]}")

import unicodedata

if unicodedata.unidata_version != "15.0.0":
    raise SystemExit(f"requires Unicode 15.0.0; got {unicodedata.unidata_version}")

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src/rules/weasel_words_very.rs"
START = "// Generated from Python 3.12.3's Unicode 15.0.0 `str.isalnum` property.\n"
END = "];\n"


def table() -> str:
    ranges: list[tuple[int, int]] = []
    for code in range(0x110000):
        if not chr(code).isalnum():
            continue
        if ranges and code == ranges[-1][1] + 1:
            ranges[-1] = (ranges[-1][0], code)
        else:
            ranges.append((code, code))
    return START + "// Keeping this table pinned avoids Rust toolchain Unicode-version drift.\n" + "// Derived Unicode data is covered by https://www.unicode.org/license.txt.\n" + "const PYTHON_ALNUM_RANGES: &[(u32, u32)] = &[\n" + "".join(
        f"    (0x{first:X}, 0x{last:X}),\n" for first, last in ranges
    ) + "];\n"


text = SOURCE.read_text(encoding="utf-8")
start = text.index(START)
end = text.index(END, start) + len(END)
current = text[start:end]
generated = table()
if "--check" in sys.argv:
    if current != generated:
        raise SystemExit("checked-in Python Unicode word table differs; rerun generator")
else:
    SOURCE.write_text(text[:start] + generated + text[end:], encoding="utf-8")
