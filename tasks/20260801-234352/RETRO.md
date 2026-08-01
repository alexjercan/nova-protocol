# Retro: Fix v0.9.0 web build: gate pause exit import

- TASK: 20260801-234352
- BRANCH: master
- REVIEW ROUNDS: 1

## What went well

The compiler error identified the mismatched cfg directly. A target-specific
check reproduced the release compile path without requiring the full site build.

## What went wrong

The menu split moved a target-gated function across modules without moving its
cfg boundary to the new import.

## What to improve next time

Run both native and wasm32 checks when splitting modules containing target-gated
items.

## Action items

No follow-up. The existing WASM release pipeline remains the regression guard.
