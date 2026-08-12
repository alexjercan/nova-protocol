# Create a shared scenario authoring helper catalog

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, scenario, authoring

## Goal

Provide one discoverable module for generic Rust helpers that construct
scenario events, filters, actions, and related RON-facing configuration.

## Scope

- Inventory small constructors across authoring modules and examples.
- Separate generic constructors from scenario-specific story helpers.
- Move shared helpers out of Shakedown without creating a broad utility dump.
- Migrate examples that use equivalent local constructors to the common catalog.
- Preserve generated content and authored behavior.
