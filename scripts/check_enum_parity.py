#!/usr/bin/env python3
"""Cross-language parity gate for the hand-mirrored `ColorScheme` / `DitherMode` enums.

`ColorScheme` and `DitherMode` are declared independently in Rust, Python, and
TypeScript (there is no code generator). `ColorScheme`'s integer values are a
firmware wire contract, so a silent mismatch between the mirrors is a real bug,
not just a lint nit. This script parses all three copies of each enum with six
INDEPENDENT regex parsers -- one per file, deliberately not routed through a
shared model -- and asserts that every language agrees on the same set of
member names (after normalising naming convention) and the same value for
each name.

Independent parsers are the point: a shared parsing/model layer used by all
six sources would hide exactly the class of bug this gate exists to catch --
e.g. a single shared "enum extraction" helper with a wrong regex would make
every mirror look consistent with every other mirror while all of them
silently drifted from the truth. Compare
`opendisplay-protocol/tools/validate_mirrors.py`, which documents the same
principle for that repo's generated mirrors.

SCOPE (this layer only)
    This script checks LOCAL cross-language parity: Rust vs Python vs
    TypeScript agree with each other. It does NOT check either mirror against
    the canonical OpenDisplay firmware C header -- that is a separate,
    heavier gate (planned) that requires the header to be vendored/available
    and is added alongside the enum-value fixes it depends on. Keeping that
    layer out lets this gate be blocking immediately, since it passes against
    the mirrors as they stand today.

    Extension point for that future layer: add a `parse_header_*` function
    per enum (mirroring the shape of the `parse_rust_*` / `parse_python_*` /
    `parse_ts_*` functions below), a `--header <path>` CLI flag, and fold its
    result into `validate_enum`'s `sources` dict under a `"header"` key. No
    other function needs to change.

THE SIX INDEPENDENT PARSERS
    parse_rust_colorscheme    packages/rust/core/src/palettes.rs
    parse_rust_dithermode     packages/rust/core/src/enums.rs
    parse_python_colorscheme  packages/python/src/epaper_dithering/palettes.py
    parse_python_dithermode   packages/python/src/epaper_dithering/enums.py
    parse_ts_colorscheme      packages/javascript/src/palettes.ts
    parse_ts_dithermode       packages/javascript/src/enums.ts

NAME NORMALISATION
    Member names are not spelled identically across languages: Rust uses
    PascalCase (`Grayscale16`, `JarvisJudiceNinke`), Python/TypeScript use
    SCREAMING_SNAKE_CASE (`GRAYSCALE_16`, `JARVIS_JUDICE_NINKE`). `normalize_name`
    converts either convention to a canonical SCREAMING_SNAKE_CASE key so
    names can be compared for real equality rather than incidentally matching
    string equality. See `_NORMALIZE_NAME_EXAMPLES` for the exact behaviour;
    `--self-test` runs them as an assertion.

EXIT CODES
    0  all mirrors agree (names and values) for both enums
    1  a divergence was found (missing/extra member or value mismatch)

Stdlib only; no third-party dependencies. Python 3.9+.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Dict, Tuple

ROOT = Path(__file__).resolve().parent.parent

RUST_PALETTES = ROOT / "packages/rust/core/src/palettes.rs"
RUST_ENUMS = ROOT / "packages/rust/core/src/enums.rs"
PYTHON_PALETTES = ROOT / "packages/python/src/epaper_dithering/palettes.py"
PYTHON_ENUMS = ROOT / "packages/python/src/epaper_dithering/enums.py"
TS_PALETTES = ROOT / "packages/javascript/src/palettes.ts"
TS_ENUMS = ROOT / "packages/javascript/src/enums.ts"

# name -> firmware integer value
EnumMembers = Dict[str, int]


# --- name normalisation ------------------------------------------------------

def normalize_name(name: str) -> str:
    """Normalise a Rust PascalCase or Python/TS SCREAMING_SNAKE_CASE enum
    member name to a canonical SCREAMING_SNAKE_CASE key.

    Rust:   Grayscale16        -> GRAYSCALE_16
            JarvisJudiceNinke  -> JARVIS_JUDICE_NINKE
            Bwgbry             -> BWGBRY   (no internal case/digit boundary)
    Python/TS members are already SCREAMING_SNAKE_CASE and pass through
    unchanged (mixed-case boundaries are idempotent: GRAYSCALE_16 has no
    lower->upper or letter->digit boundary left to insert an underscore at).
    """
    # lower->upper boundary (camel/Pascal hump), e.g. Floyd|Steinberg
    s = re.sub(r"(?<=[a-z])(?=[A-Z])", "_", name)
    # letter->digit boundary, e.g. Grayscale|16
    s = re.sub(r"(?<=[A-Za-z])(?=[0-9])", "_", s)
    return s.upper()


# Worked examples used by --self-test to pin normalize_name's behaviour.
_NORMALIZE_NAME_EXAMPLES: Tuple[Tuple[str, str], ...] = (
    ("Mono", "MONO"),
    ("MONO", "MONO"),
    ("Bwr", "BWR"),
    ("Bwgbry", "BWGBRY"),
    ("Grayscale4", "GRAYSCALE_4"),
    ("GRAYSCALE_4", "GRAYSCALE_4"),
    ("Grayscale16", "GRAYSCALE_16"),
    ("GRAYSCALE_16", "GRAYSCALE_16"),
    ("None", "NONE"),
    ("FloydSteinberg", "FLOYD_STEINBERG"),
    ("FLOYD_STEINBERG", "FLOYD_STEINBERG"),
    ("SierraLite", "SIERRA_LITE"),
    ("JarvisJudiceNinke", "JARVIS_JUDICE_NINKE"),
    ("JARVIS_JUDICE_NINKE", "JARVIS_JUDICE_NINKE"),
)


def _run_self_test() -> None:
    for raw, expected in _NORMALIZE_NAME_EXAMPLES:
        actual = normalize_name(raw)
        assert actual == expected, f"normalize_name({raw!r}) = {actual!r}, expected {expected!r}"
    print(f"[self-test] normalize_name: {len(_NORMALIZE_NAME_EXAMPLES)}/{len(_NORMALIZE_NAME_EXAMPLES)} OK")


# --- six independent parsers -------------------------------------------------
#
# Each function is a standalone regex walk over its own source file. They are
# NOT refactored into a shared "parse a C-like enum body" helper on purpose:
# a bug in a shared helper would silently pass every mirror it touches, which
# is exactly the failure mode this gate exists to catch.

def parse_rust_colorscheme(path: Path = RUST_PALETTES) -> EnumMembers:
    """Parse `pub enum ColorScheme { Name = N, ... }` from palettes.rs."""
    text = path.read_text()
    m = re.search(r"pub enum ColorScheme\s*\{(.*?)\n\}", text, re.S)
    if not m:
        raise ValueError(f"{path}: could not find `pub enum ColorScheme {{ ... }}`")
    members: EnumMembers = {}
    for line in m.group(1).splitlines():
        vm = re.match(r"\s*(\w+)\s*=\s*(\d+)\s*,?\s*$", line)
        if vm:
            members[vm.group(1)] = int(vm.group(2))
    if not members:
        raise ValueError(f"{path}: found ColorScheme enum body but parsed zero members")
    return members


def parse_rust_dithermode(path: Path = RUST_ENUMS) -> EnumMembers:
    """Parse `pub enum DitherMode { ... Name = N, ... }` from enums.rs.

    Doc-comment lines and the `#[default]` attribute line interleaved between
    variants are simply not `Name = N` and are skipped by the per-line match.
    """
    text = path.read_text()
    m = re.search(r"pub enum DitherMode\s*\{(.*?)\n\}", text, re.S)
    if not m:
        raise ValueError(f"{path}: could not find `pub enum DitherMode {{ ... }}`")
    members: EnumMembers = {}
    for line in m.group(1).splitlines():
        vm = re.match(r"\s*(\w+)\s*=\s*(\d+)\s*,?\s*$", line)
        if vm:
            members[vm.group(1)] = int(vm.group(2))
    if not members:
        raise ValueError(f"{path}: found DitherMode enum body but parsed zero members")
    return members


def parse_python_colorscheme(path: Path = PYTHON_PALETTES) -> EnumMembers:
    """Parse `class ColorScheme(Enum): NAME = (N, ColorPalette(...)), ...`.

    Members here are non-standard: each is a tuple `(int, ColorPalette)`
    whose first element becomes `_value_` in `__init__`. The int we want is
    the first element of that tuple literal, i.e. the first line after
    `NAME = (`.
    """
    text = path.read_text()
    m = re.search(r"class ColorScheme\(Enum\):(.*?)\n    def __init__", text, re.S)
    if not m:
        raise ValueError(f"{path}: could not find `class ColorScheme(Enum): ... def __init__`")
    body = m.group(1)
    members: EnumMembers = {}
    for vm in re.finditer(r"^ {4}([A-Z][A-Z0-9_]*)\s*=\s*\(\s*\n\s*(\d+)\s*,", body, re.M):
        members[vm.group(1)] = int(vm.group(2))
    if not members:
        raise ValueError(f"{path}: found ColorScheme class body but parsed zero members")
    return members


def parse_python_dithermode(path: Path = PYTHON_ENUMS) -> EnumMembers:
    """Parse `class DitherMode(IntEnum): NAME = N` from enums.py."""
    text = path.read_text()
    m = re.search(r"class DitherMode\(IntEnum\):(.*)", text, re.S)
    if not m:
        raise ValueError(f"{path}: could not find `class DitherMode(IntEnum): ...`")
    body = m.group(1)
    members: EnumMembers = {}
    for vm in re.finditer(r"^ {4}([A-Z][A-Z0-9_]*)\s*=\s*(\d+)\s*$", body, re.M):
        members[vm.group(1)] = int(vm.group(2))
    if not members:
        raise ValueError(f"{path}: found DitherMode class body but parsed zero members")
    return members


def parse_ts_colorscheme(path: Path = TS_PALETTES) -> EnumMembers:
    """Parse `export enum ColorScheme { NAME = N, ... }` from palettes.ts."""
    text = path.read_text()
    m = re.search(r"export enum ColorScheme\s*\{(.*?)\n\}", text, re.S)
    if not m:
        raise ValueError(f"{path}: could not find `export enum ColorScheme {{ ... }}`")
    members: EnumMembers = {}
    for vm in re.finditer(r"([A-Z][A-Z0-9_]*)\s*=\s*(\d+)\s*,", m.group(1)):
        members[vm.group(1)] = int(vm.group(2))
    if not members:
        raise ValueError(f"{path}: found ColorScheme enum body but parsed zero members")
    return members


def parse_ts_dithermode(path: Path = TS_ENUMS) -> EnumMembers:
    """Parse `export enum DitherMode { NAME = N, ... }` from enums.ts."""
    text = path.read_text()
    m = re.search(r"export enum DitherMode\s*\{(.*?)\n\}", text, re.S)
    if not m:
        raise ValueError(f"{path}: could not find `export enum DitherMode {{ ... }}`")
    members: EnumMembers = {}
    for vm in re.finditer(r"([A-Z][A-Z0-9_]*)\s*=\s*(\d+)\s*,", m.group(1)):
        members[vm.group(1)] = int(vm.group(2))
    if not members:
        raise ValueError(f"{path}: found DitherMode enum body but parsed zero members")
    return members


# --- comparison / reporting --------------------------------------------------

def validate_enum(enum_label: str, sources: Dict[str, Tuple[Path, EnumMembers]]) -> bool:
    """Compare normalised name -> value maps across languages.

    `sources` maps a short language label ("rust", "python", "typescript") to
    (source_path, raw_name -> value). Reports missing/extra members per
    language relative to the union of all normalised names, and value
    mismatches for names present in more than one language.
    """
    # normalized name -> {language: (raw_name, value)}
    normalized: Dict[str, Dict[str, Tuple[str, int]]] = {}
    for lang, (_, members) in sources.items():
        for raw_name, value in members.items():
            key = normalize_name(raw_name)
            normalized.setdefault(key, {})[lang] = (raw_name, value)

    all_langs = set(sources)
    ok = True

    print(f"[{enum_label}] members found: " +
          ", ".join(f"{lang}={len(members)}" for lang, (_, members) in sources.items()))

    for key in sorted(normalized):
        present = normalized[key]
        missing_langs = all_langs - set(present)
        if missing_langs:
            ok = False
            for lang in sorted(missing_langs):
                other_desc = ", ".join(
                    f"{other_lang}:{sources[other_lang][0].name} has {raw!r}={val}"
                    for other_lang, (raw, val) in sorted(present.items())
                )
                print(
                    f"  DIVERGENCE [{enum_label}] member '{key}' is MISSING from "
                    f"{lang}:{sources[lang][0].name} but present elsewhere ({other_desc})"
                )
            continue

        values = {lang: val for lang, (_, val) in present.items()}
        distinct_values = set(values.values())
        if len(distinct_values) > 1:
            ok = False
            detail = ", ".join(
                f"{lang}:{sources[lang][0].name} {raw!r}={val}"
                for lang, (raw, val) in sorted(present.items())
            )
            print(f"  DIVERGENCE [{enum_label}] member '{key}' VALUE MISMATCH: {detail}")

    if ok:
        print(f"  {enum_label}: OK ({len(normalized)} members agree across {', '.join(sorted(all_langs))})")
    return ok


def check_colorscheme() -> bool:
    sources = {
        "rust": (RUST_PALETTES, parse_rust_colorscheme()),
        "python": (PYTHON_PALETTES, parse_python_colorscheme()),
        "typescript": (TS_PALETTES, parse_ts_colorscheme()),
    }
    return validate_enum("ColorScheme", sources)


def check_dithermode() -> bool:
    sources = {
        "rust": (RUST_ENUMS, parse_rust_dithermode()),
        "python": (PYTHON_ENUMS, parse_python_dithermode()),
        "typescript": (TS_ENUMS, parse_ts_dithermode()),
    }
    return validate_enum("DitherMode", sources)


# --- entrypoint ---------------------------------------------------------------

def main(argv=None) -> int:
    p = argparse.ArgumentParser(
        prog="check_enum_parity.py",
        description=(
            "Cross-language parity gate: asserts ColorScheme and DitherMode "
            "agree (names + values) across the Rust, Python, and TypeScript "
            "mirrors. Local parity only -- see module docstring for scope."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument("--colorscheme", action="store_true", help="check ColorScheme only")
    p.add_argument("--dithermode", action="store_true", help="check DitherMode only")
    p.add_argument("--self-test", action="store_true", help="run normalize_name's worked examples and exit")
    args = p.parse_args(argv)

    if args.self_test:
        _run_self_test()
        return 0

    do_colorscheme = args.colorscheme or not args.dithermode
    do_dithermode = args.dithermode or not args.colorscheme

    ok = True
    if do_colorscheme:
        ok &= check_colorscheme()
    if do_dithermode:
        ok &= check_dithermode()

    print("\nVERDICT:", "ENUMS CONSISTENT ACROSS LANGUAGES" if ok else "DIVERGENCE FOUND")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
