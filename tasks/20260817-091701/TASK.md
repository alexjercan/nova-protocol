# CI actually runs the mesh generator checks

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.11.0,ci,art

## The gap

GREEBLES.md section 4.1 and assets/base/gltf/greebles/README.md claim the
mesh generator's --check "fails CI on a stale commit". It does not: ci.yaml
runs no python step. Nothing today stops a landed recipe/model byte
mismatch (a hand-edited glb, a stale regeneration).

## The fix

One CI step (the default-features job is the natural host) running both
generators' checks:

- python3 scripts/gen-greebles.py --check
- python3 scripts/gen-thruster-shells.py --check

Both are deterministic byte-compares and run in seconds. Runner has
python3; no new toolchain.

## Done when

- CI fails on a deliberately staled glb (prove once in a scratch commit or
  locally with the exact CI command)
- the docs' claim is true
