#!/usr/bin/env python3
"""The probe shards must partition the example catalog exactly.

`.github/workflows/ci.yaml`'s probe job no longer shards by directory: each
shard NAMES the examples it runs. That list is what makes a red shard say
"gunnery is red" instead of "screenshots is red", and it is also the one way a
new example can go missing - an example that joins the catalog and no shard
never runs, and every shard still reports green over the names it knows.

So this asserts the partition: every `[[example]]` in `Cargo.toml` appears in
exactly one shard, and no shard names anything else. `catalog_drift.rs` owns
the other half (disk <-> Cargo.toml); the two together mean every example file
on disk runs in exactly one shard.

Run it from anywhere; it reads the repository it lives in.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CI = ROOT / ".github/workflows/ci.yaml"
CARGO = ROOT / "Cargo.toml"

# The matrix shape this parser is pinned to, in ci.yaml's own indentation:
#           - shard: editor
#             examples: >-
#               screenshot_editor screenshot_planet_editor
SHARD = re.compile(r" {10}- shard: (\S+)")
EXAMPLES = " " * 12 + "examples: >-"
MEMBER = re.compile(r" {14}\S.*")


def shards_from_ci() -> dict[str, list[str]]:
    """The probe matrix's shard -> example names, read off the workflow."""
    shards: dict[str, list[str]] = {}
    current: str | None = None
    collecting = False
    for line in CI.read_text().splitlines():
        if match := SHARD.fullmatch(line):
            current, collecting = match.group(1), False
            if current in shards:
                fail(f"duplicate probe shard `{current}` in the matrix")
            shards[current] = []
        elif current and line == EXAMPLES:
            collecting = True
        elif collecting:
            if MEMBER.fullmatch(line):
                shards[current].append(line.strip())
            else:
                collecting = False
    return {name: " ".join(lines).split() for name, lines in shards.items()}


def catalog() -> list[str]:
    """Every `[[example]]` name, in Cargo.toml order."""
    return re.findall(r'^\[\[example\]\]\nname = "([^"]+)"', CARGO.read_text(), re.M)


def fail(message: str) -> None:
    print(f"::error::{message}")
    sys.exit(1)


def main() -> None:
    shards = shards_from_ci()
    names = catalog()
    if not shards:
        fail(f"no `- shard:` entries found in {CI} - has the matrix shape changed?")
    if not names:
        fail(f"no `[[example]]` blocks found in {CARGO}")

    owner: dict[str, str] = {}
    for shard, members in shards.items():
        if not members:
            fail(f"probe shard `{shard}` names no examples")
        for member in members:
            if member in owner:
                fail(f"`{member}` is in both `{owner[member]}` and `{shard}`")
            owner[member] = shard

    if unknown := sorted(set(owner) - set(names)):
        fail(f"probe shards name examples that are not in Cargo.toml: {' '.join(unknown)}")
    if missing := sorted(set(names) - set(owner)):
        fail(
            f"these examples are in no probe shard and would never run: "
            f"{' '.join(missing)} - add each to a shard in {CI}"
        )

    print(f"{len(names)} examples over {len(shards)} probe shards:")
    for shard, members in shards.items():
        print(f"  {shard:<12} {len(members):>3}  {' '.join(members)}")


if __name__ == "__main__":
    main()
