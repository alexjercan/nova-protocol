# Spike+refactor: drive controller verb availability from modification components, not config flags

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.6.0,modding,refactor,spike

Follow-up from 20260714-113411 (component-based section modifications). SPIKE first
(feasibility + simpler?), then the refactor if RECOMMENDED. Do not implement before
the spike concludes.

## Idea (user)

The section-modification work (113411) makes `DisableVerb(Orbit)` a component added to
a controller section, applied by an observer that flips a boolean field on
`ControllerVerbs` (built from `ControllerSectionConfig.verbs`). That is a DUAL source
of truth: the config flags AND the modification component both describe verb
availability. The idea: DROP the flags from the controller config/`ControllerVerbs`
and make the verb-availability system query the modification component directly - one
component-driven model. A section grants a verb by default; a `DisableVerb(v)`
component (a modification) removes it, and the availability gate reads component
presence instead of a struct field.

## Questions the spike must answer

- Is it actually simpler, or does it just move the state? (One mechanism vs two, but
  the gate now does component lookups per verb.)
- Feasibility: who reads `ControllerVerbs` today (the flight/verb-availability gate,
  the `SetControllerVerb` scenario action, HUD hints)? Trace every reader and writer
  before committing - the `SetControllerVerb` runtime action currently flips the flags
  at runtime, so a component model must still support runtime enable/disable.
- What does "default granted" mean, and how does the gate express "verb V is available
  unless a `DisableVerb(V)` is present"? Per-verb marker components vs one component
  holding a set.
- Does this generalize the modification model (health/mass overrides as the same
  pattern) or is verbs a special case?
- Migration: `ControllerSectionConfig.verbs` and the built-in catalog's verb settings.

## Constraints

- Pure-data/wasm-safe; the runtime `SetControllerVerb` scenario action must keep
  working (runtime toggling).
- Dovetails with 113411's `SectionModification` component model - reuse it, don't fork.

Spike: (this task's own SPIKE.md once explored). Seeds the refactor task(s).
