# Skin debug dump and wfc edge-case tests

- STATUS: IN_PROGRESS
- PRIORITY: 68
- TAGS: v0.11.0, ship, render, debug, harness

## Goal

A text dump of a ship's structure and its derived skin, so a skin defect can be
read rather than eyeballed.

The skin is a PURE FUNCTION of structure - no RNG, hashed off cell position. So
a dumped structure is a COMPLETE repro: given it, the derivation is reproducible
offline and pinnable in a test. That property is what makes this worth building
and it does not hold for most subsystems.

Owner: there are still "invalid / weird looking moments (e.g hulls that are bare
+ raises next to turrets etc)".

## Contents

Per cell: whether it is clad, the eight boundary samples, the canonical shape id,
the relief class, the zone facts the scatter reads (run length, enclosure,
facing), and which fixtures landed on it and why.

Extends the ship record from the state-snapshot capability rather than adding a
second serializer.

## Also in this task

MORE edge-case tests on the wfc effort, which the dump is what makes writable.
Owner asked for this directly.

## Depends on

Ships as a content kind - the dump needs a ship it can read from a file for the
input half. The dump itself can be built against a live ship first.

## Definition of done

- a dump of a real ship, read and attached
- edge-case tests that would have caught defects found by render this sprint
- shape and placement statistics emitted, because the shell-shape task needs them

## Lane

Not started. Queued behind `ship-content` (task 20260816-112330).
