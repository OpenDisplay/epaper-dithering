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

TWO LAYERS
    Layer 1 (default, always runs): LOCAL cross-language parity -- Rust vs
    Python vs TypeScript agree with each other.

    Layer 2 (`--header <path>`, opt-in): parity against the CANONICAL source of
    truth, `enum ColorScheme` in `opendisplay-protocol/src/opendisplay_structs.h`.
    That header names `epaper-dithering packages/rust/core/src/palettes.rs` as
    its designated `@external` mirror, so the header wins any disagreement.
    It is opt-in because it needs the protocol repo checked out alongside this
    one, which is true in CI (a dedicated job clones it) but not necessarily on
    a contributor's machine. Layer 1 stays blocking and dependency-free.

    Layer 2 applies to ColorScheme only -- DitherMode is a dithering-library
    concept with no firmware wire representation, so the header has nothing to
    say about it.

    The header enum is a superset of the mirrors: it also carries the
    non-epaper RGB565/RGB888/RGB16BPC schemes (values 100-102), which this
    dithering library has no palette for. `HEADER_ONLY_MEMBERS` lists those as
    a deliberate, reviewed exemption; every other header member must be
    mirrored, and every mirrored member must exist in the header with the same
    value.

THE SEVEN INDEPENDENT PARSERS
    parse_rust_colorscheme    packages/rust/core/src/palettes.rs
    parse_rust_dithermode     packages/rust/core/src/enums.rs
    parse_python_colorscheme  packages/python/src/epaper_dithering/palettes.py
    parse_python_dithermode   packages/python/src/epaper_dithering/enums.py
    parse_ts_colorscheme      packages/javascript/src/palettes.ts
    parse_ts_dithermode       packages/javascript/src/enums.ts
    parse_header_colorscheme  <opendisplay-protocol>/src/opendisplay_structs.h

NAME NORMALISATION
    Member names are not spelled identically across languages: Rust uses
    PascalCase (`Grayscale16`, `JarvisJudiceNinke`), Python/TypeScript use
    SCREAMING_SNAKE_CASE (`GRAYSCALE_16`, `JARVIS_JUDICE_NINKE`). `normalize_name`
    converts either convention to a canonical SCREAMING_SNAKE_CASE key so
    names can be compared for real equality rather than incidentally matching
    string equality. See `_NORMALIZE_NAME_EXAMPLES` for the exact behaviour;
    `--self-test` runs them as an assertion.

EXIT CODES
    0  all mirrors agree (names and values) for both enums, and -- with
       --header -- agree with the canonical C header
    1  a divergence was found (missing/extra member or value mismatch)

Stdlib only; no third-party dependencies. Python 3.9+.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
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

# Header members with no counterpart in this library, by design: they are
# non-epaper framebuffer formats (no ink palette to dither to), so the mirrors
# deliberately do not carry them. Any OTHER header member missing from the
# mirrors is a real divergence. Keep this list short and reviewed.
#
# A `frozenset`, not a dict: only membership (name) is checked below -- there
# is no mirror value to compare these header-only members against, so a dict
# would misleadingly imply the 100/101/102 values are pinned by this script.
HEADER_ONLY_MEMBERS = frozenset({"RGB565", "RGB888", "RGB16BPC"})

# Mirror members with no counterpart in the header, by design: GRAYSCALE_8 is
# a library-local dithering target (e.g. the Inkplate 10, issue #19) at value
# 9, which is NOT a firmware wire value -- the header defines ColorScheme
# values 0-8 and 100-102 only, with no 9.
#
# NAME-ONLY EXEMPTION -- READ BEFORE TOUCHING:
# This must exempt GRAYSCALE_8 from the "missing from header" check ONLY. It
# must NOT suppress a "mirror value not in header" / "header defines this
# value under another name" conflict. If the protocol ever assigns value 9 to
# something else, that is a real collision with this library's squatted
# GRAYSCALE_8=9 and the gate MUST fail loudly, not go quiet because the name
# "GRAYSCALE_8" happens to appear in an exemption list. See
# `check_colorscheme_against_header` -- GRAYSCALE_8 is looked up by name in
# `header`, not skipped by value, so a header member at value 9 under a
# different name still fails as an ordinary mirror-only divergence (that
# other name is missing from the mirror, and/or GRAYSCALE_8=9 is "not in the
# header" under ITS name -- either way `ok` goes False). Proven by
# `_run_self_test`'s `value_nine_collision_still_fails` case.
MIRROR_ONLY_MEMBERS = frozenset({"GRAYSCALE_8"})

# The header spells the grayscale schemes GRAY4/GRAY16 where the mirrors spell
# them GRAYSCALE_4/GRAYSCALE_16. That is a naming difference only -- the values
# are the contract -- so the header parser maps them explicitly rather than
# letting `normalize_name` silently fail to reconcile them.
HEADER_NAME_ALIASES = {
    "GRAY4": "GRAYSCALE_4",
    "GRAY16": "GRAYSCALE_16",
}


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
    ("SevenColor", "SEVEN_COLOR"),
    ("SEVEN_COLOR", "SEVEN_COLOR"),
    ("BwgbrySplit", "BWGBRY_SPLIT"),
    ("BWGBRY_SPLIT", "BWGBRY_SPLIT"),
)

# Every real member name in play, grouped by the key it MUST normalise to.
# `_run_self_test` asserts that no two names from DIFFERENT groups collide.
#
# Why this matters: `validate_enum` keys members by their normalised name, so
# two genuinely distinct members that normalise to the same key would silently
# merge into one entry -- their value mismatch would never be reported, and a
# member missing from one language would look present. Forward-direction
# examples alone cannot catch that; this is the injectivity check.
_NORMALIZE_NAME_COLLISION_GROUPS: Tuple[Tuple[str, ...], ...] = (
    ("Mono", "MONO"),
    ("Bwr", "BWR"),
    ("Bwy", "BWY"),
    ("Bwry", "BWRY"),
    ("Bwgbry", "BWGBRY"),
    ("BwgbrySplit", "BWGBRY_SPLIT"),
    ("SevenColor", "SEVEN_COLOR"),
    ("Grayscale4", "GRAYSCALE_4"),
    ("Grayscale16", "GRAYSCALE_16"),
    ("None", "NONE"),
    ("Ordered", "ORDERED"),
    ("FloydSteinberg", "FLOYD_STEINBERG"),
    ("Burkes", "BURKES"),
    ("Atkinson", "ATKINSON"),
    ("Stucki", "STUCKI"),
    ("Sierra", "SIERRA"),
    ("SierraLite", "SIERRA_LITE"),
    ("JarvisJudiceNinke", "JARVIS_JUDICE_NINKE"),
    ("Rgb565", "RGB565"),
    ("Rgb888", "RGB888"),
    ("Rgb16Bpc", "RGB16BPC"),
)


def _run_value_nine_collision_self_test() -> None:
    """Prove that exempting GRAYSCALE_8 by NAME from the "missing from header"
    check does NOT also swallow a real value-9 collision.

    `GRAYSCALE_8` squats library-local value 9, which the canonical header
    currently has no opinion on (it defines 0-8 and 100-102). If the protocol
    ever assigns 9 to something else, this library's GRAYSCALE_8=9 collides
    with it, and `check_colorscheme_against_header` MUST fail -- silently
    tolerating that would mean the gate stopped meaning anything for the one
    value this library invented.

    This builds a synthetic header identical to the real `enum ColorScheme`
    body but with one extra member, `OD_COLOR_SCHEME_EXPERIMENTAL = 9`, i.e.
    value 9 claimed under a name that is NOT "GRAYSCALE_8". `MIRROR_ONLY_MEMBERS`
    only ever exempts by name, so this must NOT be absorbed by the
    `GRAYSCALE_8` exemption -- it must surface as an ordinary "header member
    missing from the mirror" divergence (`EXPERIMENTAL` has no mirror
    counterpart under any name), and the check must return False.
    """
    synthetic_header = """
enum ColorScheme {
    OD_COLOR_SCHEME_MONO         = 0,
    OD_COLOR_SCHEME_BWR          = 1,
    OD_COLOR_SCHEME_BWY          = 2,
    OD_COLOR_SCHEME_BWRY         = 3,
    OD_COLOR_SCHEME_BWGBRY       = 4,
    OD_COLOR_SCHEME_GRAY4        = 5,
    OD_COLOR_SCHEME_GRAY16       = 6,
    OD_COLOR_SCHEME_SEVEN_COLOR  = 7,
    OD_COLOR_SCHEME_BWGBRY_SPLIT = 8,
    OD_COLOR_SCHEME_EXPERIMENTAL = 9,
    OD_COLOR_SCHEME_RGB565       = 100,
    OD_COLOR_SCHEME_RGB888       = 101,
    OD_COLOR_SCHEME_RGB16BPC     = 102
};
"""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".h", delete=False) as f:
        f.write(synthetic_header)
        temp_path = Path(f.name)
    try:
        result = check_colorscheme_against_header(temp_path)
    finally:
        temp_path.unlink()

    assert result is False, (
        "[self-test] value_nine_collision_still_fails: expected "
        "check_colorscheme_against_header to return False when the header "
        "claims value 9 under a name other than GRAYSCALE_8, but it returned "
        "True -- the GRAYSCALE_8 name-exemption is silently swallowing a real "
        "value-9 collision"
    )
    print("[self-test] value_nine_collision_still_fails: OK (header value 9 under "
          "a different name still fails the gate)")


def _run_self_test() -> None:
    for raw, expected in _NORMALIZE_NAME_EXAMPLES:
        actual = normalize_name(raw)
        assert actual == expected, f"normalize_name({raw!r}) = {actual!r}, expected {expected!r}"
    print(f"[self-test] normalize_name: {len(_NORMALIZE_NAME_EXAMPLES)}/{len(_NORMALIZE_NAME_EXAMPLES)} OK")

    # Injectivity: names within a group must agree; names across groups must not.
    seen: Dict[str, Tuple[str, ...]] = {}
    for group in _NORMALIZE_NAME_COLLISION_GROUPS:
        keys = {normalize_name(name) for name in group}
        assert len(keys) == 1, (
            f"[self-test] spellings of the same member disagree: "
            f"{ {n: normalize_name(n) for n in group} }"
        )
        key = keys.pop()
        clash = seen.get(key)
        assert clash is None, (
            f"[self-test] COLLISION: distinct members {clash} and {group} both "
            f"normalise to {key!r} -- validate_enum would silently merge them"
        )
        seen[key] = group
    total = sum(len(g) for g in _NORMALIZE_NAME_COLLISION_GROUPS)
    print(f"[self-test] normalize_name injectivity: {total} names -> "
          f"{len(seen)} distinct keys, no collisions")

    _run_value_nine_collision_self_test()


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


def _strip_c_comments(text: str) -> str:
    """Remove `/* ... */` (including `/**< ... */` doc comments) and `//` line
    comments from a C source fragment.

    The enum body scanned by `parse_header_colorscheme` carries per-member
    `/**< @doc ... */` comments, and this repo's `@changed` doc notes have
    already shown up spelling other enumerator symbols inline (e.g. the
    historical `COLOR_SCHEME_GRAY8=7` note on `OD_COLOR_SCHEME_SEVEN_COLOR`).
    Today that note omits the `OD_` prefix so the member regex can't match it,
    but that's luck, not a guarantee -- a future comment that spells a full
    `OD_COLOR_SCHEME_...=N` symbol would otherwise be parsed as a phantom
    member. Stripping comments first removes that whole class of false match.
    """
    without_block_comments = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    without_line_comments = re.sub(r"//.*", "", without_block_comments)
    return without_line_comments


def parse_header_colorscheme(path: Path) -> EnumMembers:
    """Parse `enum ColorScheme { OD_COLOR_SCHEME_NAME = N, ... }` from the
    canonical `opendisplay_structs.h`.

    Deliberately a seventh INDEPENDENT parser rather than a generalisation of
    the six above: the header is the source of truth the mirrors are checked
    against, so sharing parsing machinery with a mirror parser would let one
    bug make the truth and a mirror agree with each other while both are wrong.

    Members are returned under their short names (the `OD_COLOR_SCHEME_` prefix
    stripped, `HEADER_NAME_ALIASES` applied) so `normalize_name` can line them
    up with the mirrors.
    """
    text = path.read_text()
    m = re.search(r"\benum ColorScheme\s*\{(.*?)\n\};", text, re.S)
    if not m:
        raise ValueError(f"{path}: could not find `enum ColorScheme {{ ... }};`")
    body = _strip_c_comments(m.group(1))
    members: EnumMembers = {}
    for vm in re.finditer(r"OD_COLOR_SCHEME_([A-Z0-9_]+)\s*=\s*(\d+)", body):
        raw = vm.group(1)
        members[HEADER_NAME_ALIASES.get(raw, raw)] = int(vm.group(2))
    if not members:
        raise ValueError(f"{path}: found ColorScheme enum body but parsed zero members")
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


def check_colorscheme_against_header(header_path: Path) -> bool:
    """Layer 2: compare the Rust mirror against the canonical C header.

    The Rust file is the header's designated `@external` mirror, and layer 1
    already proves Python and TypeScript match Rust, so checking Rust against
    the header transitively pins all three.

    Divergences reported:
      * a member present in both with different values (the dangerous case --
        it silently ships the wrong bytes to a panel);
      * a header member absent from the mirror and not in `HEADER_ONLY_MEMBERS`;
      * a mirror member absent from the header (an invented wire value).
    """
    header = {normalize_name(n): v for n, v in parse_header_colorscheme(header_path).items()}
    mirror = {normalize_name(n): v for n, v in parse_rust_colorscheme().items()}
    exempt = {normalize_name(n) for n in HEADER_ONLY_MEMBERS}

    print(f"[ColorScheme vs header] header={len(header)} members "
          f"({len(exempt)} non-epaper, exempt), rust={len(mirror)} members")
    print(f"  header: {header_path}")

    ok = True
    for name in sorted(header):
        if name in mirror:
            if header[name] != mirror[name]:
                ok = False
                print(f"  DIVERGENCE [ColorScheme vs header] '{name}' VALUE MISMATCH: "
                      f"header={header[name]}, rust:{RUST_PALETTES.name}={mirror[name]}")
        elif name not in exempt:
            ok = False
            print(f"  DIVERGENCE [ColorScheme vs header] '{name}'={header[name]} is in the "
                  f"canonical header but MISSING from rust:{RUST_PALETTES.name}")

    mirror_exempt = {normalize_name(n) for n in MIRROR_ONLY_MEMBERS}
    for name in sorted(mirror):
        if name not in header and name not in mirror_exempt:
            ok = False
            print(f"  DIVERGENCE [ColorScheme vs header] '{name}'={mirror[name]} exists in "
                  f"rust:{RUST_PALETTES.name} but NOT in the canonical header "
                  f"(invented wire value)")

    if ok:
        print("  ColorScheme vs header: OK "
              f"({len(header) - len(exempt)} shared members agree with the canonical header)")
    return ok


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
    p.add_argument(
        "--header",
        metavar="PATH",
        type=Path,
        default=None,
        help=(
            "also check ColorScheme against the canonical `enum ColorScheme` in "
            "opendisplay-protocol/src/opendisplay_structs.h (layer 2)"
        ),
    )
    args = p.parse_args(argv)

    if args.self_test:
        _run_self_test()
        return 0

    do_colorscheme = args.colorscheme or not args.dithermode
    do_dithermode = args.dithermode or not args.colorscheme

    ok = True
    if do_colorscheme:
        ok &= check_colorscheme()
        if args.header is not None:
            if not args.header.is_file():
                print(f"ERROR: --header path does not exist: {args.header}")
                return 1
            print()
            ok &= check_colorscheme_against_header(args.header)
    if do_dithermode:
        ok &= check_dithermode()

    verdict = "ENUMS CONSISTENT ACROSS LANGUAGES"
    if args.header is not None and do_colorscheme:
        verdict += " AND WITH THE CANONICAL HEADER"
    print("\nVERDICT:", verdict if ok else "DIVERGENCE FOUND")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
