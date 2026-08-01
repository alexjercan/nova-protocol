# Retro: Release v0.9.1

- TASK: 20260802-000300
- BRANCH: master
- REVIEW ROUNDS: 1

## What went well

- Exact three-file release commit kept the tag boundary auditable.
- Targeted wasm and website checks caught the release surfaces directly.

## What went wrong

- Flow gating interrupted a release task whose publication work was still open.
- Watching the superseded CI run added noise; the owner removed that requirement.

## What to improve next time

- Define release tasks around the desired tag boundary and explicit publication
  policy before starting flow.

## Action items

- None. The owner removed unwanted monitoring and asset-verification scope.
