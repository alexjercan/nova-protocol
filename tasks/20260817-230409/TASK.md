# Allow shared NOVA OS section bindings

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.11.0,fix,nova-os,input

## Goal

Let several sections on one ship use the same keyboard key or mouse button when
rebinding through the NOVA OS ship app.

## Accepted design

- Section-to-section duplicates are allowed, matching the editor.
- Reserved flight-control inputs remain blocked.
- Rebinding still replaces only the selected section's complete binding list.

## Done when

- Rebinding a section to another section's input succeeds.
- Both sections keep the shared input.
- Reserved flight-control rejection remains covered.
- NOVA OS documentation states the shared-binding policy.
