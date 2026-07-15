# Mod dependencies: auto-install/auto-enable deps, topological merge order

- STATUS: OPEN
- PRIORITY: 8
- TAGS: modding

Spike: tasks/20260714-202515/SPIKE.md (option AC)
Depends on: 20260715-142916 (Explore tab - install path exists). Backlog until
a mod actually declares a dependency.

Goal: make the `dependencies: [ids]` field (schema landed with the bundle-meta
task, validated by the portal generator) actually resolve. Installing a mod
from Explore pulls its missing deps from the same catalog first; enabling a mod
auto-enables its deps (Factorio behavior; disabling a dep warns about
dependents); merge order becomes dependency-respecting topological order with
catalog order as the tiebreak. No version constraints yet (ids only) - semver
ranges are a future task if real demand appears. UI: the details panel's
dependency list links/marks missing vs installed deps.

