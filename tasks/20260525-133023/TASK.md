# Blast radius visual

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.4.0,torpedo

Shader or particle effect on detonation. Legacy #147.

Pulled into v0.4.0 (roadmap spike 20260708-161726): completes torpedo detonation
feedback. Prefer a shader/gizmo expanding-sphere over particles so this is NOT
blocked by the wasm particle issue (162908) - unlike the bay-particles task
(133024), which stays in v0.5.0 for that reason.
