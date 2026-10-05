#!/usr/bin/env python3
"""Validate the curated probe suites against the Cargo example catalog.

The full shards contain distinct, known examples and no empty shard. The PR
smoke set is non-empty, duplicate-free, and contained in those shards. Other
cataloged examples may run in neither suite; WFC arena is one such example.

Both lists are read from the workflow sources, without copies here. Unexpected
source shapes fail loudly. The independent `nova_probe_cli` `catalog_drift.rs`
check owns examples/ on disk <-> Cargo.toml, including examples omitted from
these runtime suites. The default all-target build still compiles them.

Run it from anywhere; it reads the repository it lives in.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CI = ROOT / ".github/workflows/ci.yaml"
FULL = ROOT / ".github/workflows/probe-full.yaml"
CARGO = ROOT / "Cargo.toml"

# The full matrix shape this parser is pinned to, in probe-full.yaml's own
# indentation:
#           - shard: boot-ui
#             examples: >-
#               system_menu_boot system_session_loop
SHARD = re.compile(r" {10}- shard: (\S+)")
SHARD_EXAMPLES = " " * 12 + "examples: >-"
SHARD_MEMBER = re.compile(r" {14}\S.*")

# The smoke shape, a job-level env block in ci.yaml:
#       SMOKE_EXAMPLES: >-
#         system_menu_boot system_session_loop
SMOKE_EXAMPLES = " " * 6 + "SMOKE_EXAMPLES: >-"
SMOKE_MEMBER = re.compile(r" {8}\S.*")

# A line that MEANT to be one of the headers above but does not match it - a
# re-indent, a stray space, a rename. Without these the parser keeps reading
# under the previous header and folds the orphaned block into it, which is the
# one way it could stay green while describing the wrong thing.
NEAR_SHARD = re.compile(r"\s*-\s*shard\s*:")
NEAR_SHARD_EXAMPLES = re.compile(r"\s*examples\s*:")
NEAR_SMOKE = re.compile(r"\s*SMOKE_EXAMPLES\s*:")


def fail(message: str) -> None:
    print(f"::error::{message}")
    sys.exit(1)


def folded_block(
    path: Path, header: str, member: re.Pattern[str], near: re.Pattern[str]
) -> list[str]:
    """The whitespace-separated names of one `>-` folded block, or []."""
    names: list[str] = []
    collecting = False
    for line in path.read_text().splitlines():
        if line == header:
            collecting = True
        elif near.fullmatch(line.rstrip(">- ")):
            fail(f"{path} line `{line.strip()}` is not the shape this parser reads: {header!r}")
        elif collecting:
            if not member.fullmatch(line):
                break
            names += line.split()
    return names


def full_shards() -> dict[str, list[str]]:
    """The full suite's shard -> example names, read off its matrix."""
    shards: dict[str, list[str]] = {}
    current: str | None = None
    collecting = False
    for line in FULL.read_text().splitlines():
        if match := SHARD.fullmatch(line):
            current, collecting = match.group(1), False
            if current in shards:
                fail(f"duplicate probe shard `{current}` in {FULL}")
            shards[current] = []
        elif NEAR_SHARD.match(line):
            fail(f"{FULL} line `{line.strip()}` is not the shard shape this parser reads")
        elif line == SHARD_EXAMPLES:
            if current is None:
                fail(f"{FULL} has an orphan examples block without a shard")
            collecting = True
        elif NEAR_SHARD_EXAMPLES.fullmatch(line.rstrip(">- ")):
            fail(f"{FULL} line `{line.strip()}` is not the shape this parser reads")
        elif collecting:
            if SHARD_MEMBER.fullmatch(line):
                shards[current].append(line)
            else:
                collecting = False
    return {name: " ".join(lines).split() for name, lines in shards.items()}


def catalog() -> list[str]:
    """Every `[[example]]` name, in Cargo.toml order."""
    return re.findall(r'^\[\[example\]\]\nname = "([^"]+)"', CARGO.read_text(), re.M)


def main() -> None:
    shards = full_shards()
    smoke = folded_block(CI, SMOKE_EXAMPLES, SMOKE_MEMBER, NEAR_SMOKE)
    names = catalog()
    if not shards:
        fail(f"no `- shard:` entries found in {FULL} - has the matrix shape changed?")
    if not smoke:
        fail(f"no `SMOKE_EXAMPLES: >-` block found in {CI} - has the job shape changed?")
    if not names:
        fail(f"no `[[example]]` blocks found in {CARGO}")

    owner: dict[str, str] = {}
    for shard, members in shards.items():
        if not members:
            fail(f"probe shard `{shard}` names no examples")
        for name in members:
            if name in owner:
                fail(f"`{name}` is in both `{owner[name]}` and `{shard}`")
            owner[name] = shard

    if unknown := sorted(set(owner) - set(names)):
        fail(f"{FULL} names examples that are not in Cargo.toml: {' '.join(unknown)}")
    if repeated := sorted({name for name in smoke if smoke.count(name) > 1}):
        fail(f"the smoke set names these twice: {' '.join(repeated)}")
    if unknown := sorted(set(smoke) - set(names)):
        fail(f"the smoke set names examples that are not in Cargo.toml: {' '.join(unknown)}")
    if orphan := sorted(set(smoke) - set(owner)):
        fail(f"these smoke examples are in no full shard: {' '.join(orphan)} - add each in {FULL}")

    print(f"{len(owner)} curated examples over {len(shards)} full probe shards ({len(names)} cataloged):")
    for shard, members in shards.items():
        print(f"  {shard:<16} {len(members):>3}  {' '.join(members)}")
    print(f"smoke set ({len(smoke)}): {' '.join(smoke)}")


if __name__ == "__main__":
    main()
