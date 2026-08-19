# Decide where a shared id lives, then stop re-typing them

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.11.0,architecture,spike

Epic: `20260818-220812`. Found by the design audit; evidence and the full site
list are in `tasks/20260818-220812/AUDIT.md`.

**This needs a design answer, not a patch.** Do not start by fixing sites.

## The fault

`CONVENTIONS.md` Nova rule 5 already states it: prototype, scenario, style and
asset ids are runtime STRINGS that nothing type-checks, so renaming one compiles
clean and fails at load. The audit found the rule is broken systematically, and
always for the same reason: **the crate that needs the id cannot reach the crate
that owns it, so somebody re-types the literal.**

Confirmed sites:

- `crates/nova_os_ui/src/map/contacts.rs:259,366` - `"asteroid"` re-typed
  because `nova_os_ui` cannot depend on `nova_scenario`. Rename the type name
  and the NOVA OS map silently empties through a bare `continue` - no log, no
  warning. **And the tests re-type the same literal, so they stay green.** That
  last part is what makes it dangerous: the test cannot catch the rename it
  exists to catch.
- Editor section-prototype ids (`crates/nova_editor/src/placement.rs:213,236`,
  `scenario.rs:312-391`) - rename makes the New Ship button a silent no-op.
- `"base"` as the base-mod id, re-typed across five files
  (`nova_menu/src/mods.rs:828`, `nova_editor/src/snap.rs:315`, and others).
- `crates/nova_assets/src/collections.rs:88-108` - seven art paths duplicated
  from `assets/base/base.bundle.ron`. Being fixed separately for its OWN reason
  (a missing failure state hangs the loading screen), but it is the same fault.

## Why it keeps happening

The failure is always SILENT or warn-only. Nothing has ever gone bang, so
nothing has ever been noticed. Combined with tests re-typing the same literals,
the codebase has an entire class of breakage with no detection at all.

It is also not laziness: in every case the crate graph genuinely forbids the
dependency that would let the id be a constant. `nova_os_ui` depending on
`nova_scenario` would be a worse problem than the one it solves.

## The question to answer

**Where does an id that several crates need actually live?** Options, none
chosen:

- A small leaf crate every layer may depend on, holding the well-known ids as
  constants. Cheap, but it becomes a dumping ground unless the entry bar is
  written down.
- Ids stay strings, but every consumer resolves through a registry that FAILS
  LOUDLY on a miss rather than falling through a `continue`. Keeps the graph as
  it is; turns silence into a log or a panic.
- Newtypes over the strings, defined once, so at least the TYPE is checked even
  though the value is not.
- Accept the duplication, and make the DETECTION good: a test that reads the
  content RON and asserts every re-typed literal in the tree still resolves.
  Cheapest to build, and it directly kills the "tests re-type the literal"
  problem.

The last one deserves serious weight. It does not fight the crate graph, and the
real defect is not that the string is duplicated - it is that nothing notices
when the duplicate goes stale.

## Do not

- Do not add a dependency that inverts the documented crate graph. The audit
  confirmed `docs/architecture.md` matches reality today, including plugin
  order. That is worth keeping.
- Do not fix individual sites before the answer exists, or the next one lands
  the same way.

## Done when

- The approach is decided and recorded HERE, with its reasoning, before any
  code moves.
- Every confirmed site above is either fixed under it or explicitly exempted.
- A rename of a well-known id fails something LOUD - a test, a lint or a startup
  error - rather than emptying a screen in silence.
- `CONVENTIONS.md` Nova rule 5 gains the answer, so the next person does not
  re-type the literal.
