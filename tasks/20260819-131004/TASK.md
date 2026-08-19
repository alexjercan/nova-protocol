# Decide where a shared id lives, then stop re-typing them

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,done

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
- A rename of a well-known id stops compiling. A `const` a rename breaks at
  BUILD time is louder than any test, and it needs no test to stay true, so no
  detection test is added for this.
- `CONVENTIONS.md` Nova rule 5 gains the answer, so the next person does not
  re-type the literal.

## The answer

**A shared id is a `const`, and it lives in the LOWEST crate every consumer
already depends on.** No registry, no newtype, no drift test. A consumer that
cannot reach the owner does not get a new dependency edge - the constant moves
DOWN to the floor both sides can already see, and the owner imports it back up.

Entry bar, so the floor does not become a dumping ground: **an id moves down
only when a crate that cannot reach its owner must name it in Rust.** An id
with no cross-crate Rust consumer stays where it is authored. This is why ship
ids (`CARGOA_SHIP_ID`) and style ids (`INDUSTRIAL_STYLE_ID`) stay `nova_authoring`'s
- nothing outside that crate names one - while five section ids move.

Why not the alternatives:

- A new leaf crate is unnecessary. Every cluster already has a floor:
  `nova_events` for scenario-object type names, `nova_mod_format` for the mod
  id, `nova_ship` for section prototype ids. A new crate would buy nothing and
  cost an edge everywhere.
- A registry and newtypes both add machinery to convert a silent runtime miss
  into a loud runtime miss. A `const` converts it into a COMPILE error, which
  is strictly earlier and strictly cheaper.
- A drift test that reads the content RON keeps the duplication and adds a
  test to guard it. The duplication is the defect.

### Placement

| Id family | Const | Lands in | Why that floor |
|---|---|---|---|
| Scenario-object type names (`asteroid`, `spaceship`, `beacon`, `anchor`, `light`, `salvage_crate`) | `ASTEROID_TYPE_NAME`, ... | `nova_events` (crate root, exported by its prelude) | `nova_events` already owns `EntityTypeName` and the reflect field-name consts. `nova_os_ui`, `nova_gameplay`, `nova_ship` and `nova_scenario` all depend on it; none but `nova_scenario` may depend on `nova_scenario`. |
| Base mod id (`base`) | `BASE_MOD_ID` | `nova_mod_format` (crate root; re-exported by `nova_modding`) | It is the id of a `ModEntry`, whose schema is this crate. `nova_assets`, `nova_menu`, `nova_authoring` and `nova_modding` all depend on it directly. |
| Base section prototype ids (5) | `REINFORCED_HULL_SECTION_ID`, `LIGHT_HULL_SECTION_ID`, `BASIC_CONTROLLER_SECTION_ID`, `BASIC_THRUSTER_SECTION_ID`, `PDC_KINETIC_TURRET_SECTION_ID` | `nova_ship::sections::catalog_ids` | The id is a key into `GameSections`, which is `nova_ship`'s. `nova_editor` (the consumer) and `nova_authoring` (the author) both depend on `nova_ship`, and neither can see the other. |

Zero new dependency edges. Zero manifest changes.

### Exempted, with reasons

- **Base art paths** (`nova_assets/src/collections.rs:90-108`,
  `nova_editor/src/scenario.rs:174,184,276-277,477-478`,
  `nova_ship/src/sections/torpedo_section/bay.rs:373`). Same fault, but the
  task record already assigns it elsewhere: the damage there is the missing
  loading-screen failure state, and the fix is a bundle-derived list, not a
  constant.
- **`examples/`** (~35 files naming section ids). Covered: an unknown prototype
  makes the run miss its beats and the probe gate goes red. Production has no
  such gate, which is exactly what makes production the defect.
- **`crates/*/tests/`** integration tests. They build bundle trees where the id
  is also RON text, and they read the real shipped content, so they already
  fail when it drifts.
- **`nova_perf_web/src/main.rs:30`** - `"broadside"` as the default `scenario`
  query param of a dev-tool binary. Not a contract: an unknown id reports and
  the tool is a dev tool.
- **`nova_scenario/src/lint/ship.rs:670,674`** - `"base"` is the `source` LABEL
  argument, not a mod id lookup. Any string works.
- **`nova_editor/src/snap.rs:315`**, `nova_ship/src/sections/clearance.rs:268`,
  `nova_authoring/.../standard.rs:259,357` - `"base"` here is a `LinkPoint` id
  (the face a part bolts down through), not the mod id. The audit
  mis-classified `snap.rs:315`.
- **Self-contained test fixtures** (`nova_editor/src/gallery/`,
  `nova_menu/src/tests/mods.rs`, `nova_assets/src/mod_refs.rs` tests). A fixture
  that defines and consumes its own ids in one place is not a contract with
  anything. The `mod_refs` ones also carry the id inside `dep://base/...` ref
  text, so half of each pair could not be a const anyway.

## What landed

Four commits, no new tests, no new crate, no manifest change.

- `nova_events` gains six `*_TYPE_NAME` consts; the `nova_scenario` object
  modules that declared them now import them. `nova_os_ui`'s map, and the
  `nova_gameplay` / `nova_ship` tests that could not reach `nova_scenario`, all
  stop re-typing.
- `nova_mod_format::BASE_MOD_ID`, re-exported through `nova_modding`'s prelude.
  Every production special-case in `nova_assets`, `nova_menu` and
  `nova_authoring` uses it.
- `nova_ship::sections::catalog_ids` gains five prototype ids. `nova_editor`'s
  New Ship buttons and sandbox scenario use them, and `nova_authoring`'s
  builders now AUTHOR from the same consts, so there is one literal per id in
  the workspace.
- `CONVENTIONS.md` Nova 5, `docs/guide-extend-scenarios.md` and
  `docs/scenario-system.md` record where a new id goes.

Proof: `cargo check --workspace --all-targets` clean, `cargo fmt --check`
clean, the `--lib` suite of all ten touched crates green, and
`content -- gen` leaves `assets/` byte-identical - which is what shows no id
VALUE moved. `content -- lint`: 0 errors, 0 warnings.

No `CHANGELOG.md` entry: nothing a player or a modder can observe changed.
