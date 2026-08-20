# Conventions

How Nova Protocol code and its documentation are written. `AGENTS.md` owns
workflow and process. Read both before writing anything.

Every rule here is a mistake somebody actually made. Nothing is recorded because
it is good practice in general - if a rule is just how Rust or Bevy works, it
does not belong in this file.

## Rust

1. Use `#[expect(<lint>, reason = "...")]`, not bare `#[allow]`.
   `#[allow(missing_docs)]` in `nova_assets/src/portal/mod.rs` is the exception.
2. Do not enable `clippy::pedantic`, `clippy::nursery`,
   `clippy::wildcard_imports`, `clippy::redundant_pub_crate`,
   `clippy::needless_pass_by_value`, or `clippy::missing_docs_in_private_items`
   across the workspace.
3. Use the pinned nightly toolchain and `rustfmt.toml`. Stable Rust ignores the
   nightly `imports_granularity` and `group_imports` settings.
4. Put unit tests in inline `#[cfg(test)] mod tests`. Move a large test module
   to a sibling `src/**/tests/` directory. Keep `crates/*/tests/` for integration
   tests.
5. Name tests as sentences that state the behavior.

## Modules and preludes

1. Give each module that exports items a `prelude`. Re-export items by name
   when a glob can include an engine prelude.
2. Import through preludes, including inside the same crate. Do not import from
   another module's internal path.
3. Export each module prelude from the crate root.

```rust
//! Ship command selection shared by player, AI, and targeting systems.
//!
//! Change this module when adding a way to command a ship.

/// The asteroid config, spawner, and plugin.
pub mod prelude {
    pub use super::{AsteroidConfig, AsteroidPlugin};
}

use crate::prelude::*;
```

## Bevy

1. Name plugin types `<Subsystem>Plugin`.
2. Name system sets `<Subsystem>Systems`.
3. State each scheduling dependency with `.before(...)` or `.after(...)`.
4. Create a `SystemSet` only when another plugin needs an ordering handle.

```rust
app.add_systems(Update, draw_juice_flashes.after(TransformSystems::Propagate));

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipInputSystems;
```

## Nova

1. A bug becomes a RANGE. Reproduce it as an `examples/systems/` range before
   the fix, so the fix is what turns the range green. Every claim a range makes
   is named: a `nova_probe::probe_marker` reading `outcome: <slug>` beside the
   assert it belongs to, and that slug on the roster in
   `crates/nova_probe_cli/tests/catalog_drift.rs` - deleting an assertion then
   fails a test instead of leaving a green run that proves less. `systems/`
   still owns correctness, and a range that only measures is not a range.
2. An example is filed by WHO it is for, in one of THREE directories.
   `playable/` is for a human: they load it and do the thing it demonstrates,
   through an affordance wired outside the `NOVA_AUTOPILOT` gate. `systems/` is
   for the probe. `screenshots/` is for the website. The test for `playable/`
   is whether a human loading it would expect to DO something - if the name
   promises a verb, it owes the verb; the test between the other two is whether
   a run produces an assertion or a picture. A free-fly camera is not an
   affordance: the scenario loader hands one to every cameraless scene, so
   "you can look at it" files under `screenshots/`.
3. Autopilot is a SECOND driver, never a disqualifier. A `playable/` example
   keeps its script and its captures - that is what keeps it on the probe gate
   and in the docs pipeline. An example that silently does nothing when a human
   loads it is the defect: it either grows the affordance and moves, or it says
   in its `about` line that it is autopilot-only.
4. Gameplay randomness comes from SEEDED `bevy_rand`, never `rand::rng()`. The
   thread RNG makes a run unreproducible, which silently voids every probe
   assertion built on a seeded layout. Section death drew from it for a whole
   epic before anyone noticed.
5. Prototype, scenario, style and asset ids are runtime STRINGS. Nothing
   type-checks them, so renaming one compiles clean and fails at load. Grep
   every id you rename - examples included - and RUN what you changed.
   An id a SECOND crate names is a `const`, and it lives in the LOWEST crate
   both already depend on. Never reach it by adding a dependency edge - move
   the constant DOWN and let the owner import it back up. Today that is
   `nova_events` for `EntityTypeName` values, `nova_mod_format` for
   `BASE_MOD_ID`, and `nova_ship::sections::catalog_ids` for base section
   prototypes. An id with no cross-crate Rust consumer stays a literal beside
   the builder that authors it - that entry bar is what keeps those three from
   becoming dumping grounds. `examples/` and `crates/*/tests/` keep literals:
   a probe run and `content lint` already go red when one drifts, which is the
   detection shipped code does not have.
6. An example builds its app with `AppBuilder`, never `App::new()` plus
   `DefaultPlugins` by hand. `with_game_plugins` is the seam for whatever the
   range adds; `build()` supplies assets, the loading screen, gameplay, ship and
   scenario with the render choice threaded through all of them.

   Hand-assembly does not merely diverge from the shipped app - it SILENTLY OPTS
   OUT of everything the builder later learns. `system_blast_penetration` and
   `system_section_severing` each hand-rolled a `WindowPlugin` for a custom
   window title. When `NOVA_NORENDER` landed on `AppBuilder::new`, both kept
   opening a window and running on the GPU inside a `--norender` sweep, reported
   nothing wrong, and were caught only by an audit that ran every range by hand.

   Deliberately NOT enforced by a test. A range is the game, assembled the way
   the game assembles itself; a lint would police the symptom and teach nobody
   the reason.

## Comments and rustdoc

1. Start each module with a `//!` document of at most three sentences:
   ownership, the key constraint, and when to change it. Use intra-doc links
   for types and wiki links for concepts.
2. Document information that declarations do not show. Do not restate code.
3. Explain constraints and non-obvious choices. Do not record history.
4. Do not cite task artifacts in docs. `TODO(<task-id>)` is allowed for live
   tracker work.
5. A constant's doc states its VALUE only when the doc explains the value. A
   number written into prose beside the number itself goes stale silently, and
   both then have to be believed.

## Documentation

**The reader's baseline is the LAST RELEASE, not the last commit.** Everything a
cycle adds and then changes or removes before shipping never happened. This is
Comments 3 applied to what readers see, and it is the rule this project breaks
most often.

1. Remove the docs for a removed thing. Do not leave a page saying it is gone,
   deprecated, or replaced - that is history. A migration note is for a format
   that SHIPPED.
2. Do not claim a rename, a removal, or a break without finding the old name in
   a RELEASED section first. "Scorch becomes Cracks" is noise if `Scorch` never
   shipped.
3. Ship code and the docs it invalidates in the SAME task.
   `docs/keeping-docs-in-sync.md` is the routing map: it says which of the four
   surfaces - `CHANGELOG.md`, `/wiki/`, `/create/`, the dev book - each code
   area owes.
4. Aim each surface at its own reader and do not paste one paragraph into three.
   `/wiki/` is what a player experiences, `/create/` is the authored contract
   and must be exact, the dev book is the mechanism.

   The test is REACHABILITY, not topic: **can this page's reader reach the thing
   it describes?** A player cannot run `cargo run --example wfc_arena`, so a
   wiki section about that bench is misfiled however player-shaped it reads.
   Both violations found on 2026-08-20 passed a topic check and failed this one
   - an arena bench on `/wiki/nova-os/`, and the exhaustive event/filter/action
   catalog folded into `/wiki/scenarios/` when `/create/` owns it.

   Duplication is the symptom, not the defect. The defect is that the copy
   drifts and there is no way to tell which one is true - and the exact copy
   goes stale on the surface whose reader can least afford it, because
   `/create/` must be exact and a wiki fold is nobody's job to update.
5. A page describing something about to be rewritten gets a HOLE, not a
   placeholder. Strike what is false, say the mechanism is undocumented, and
   schedule it.

## Changelog

Same baseline as Documentation: entries describe the delta from the last
RELEASE.

1. One entry per change, 200 characters HARD MAX, measured on the whole entry
   with wrapped lines joined. Detail goes to `web/src/news/<version>.md` or the
   task folder.
2. A thing added and then revised several times in one cycle gets ONE entry,
   describing where it ended up.
3. A bug introduced and fixed inside one cycle gets NO entry. It never existed
   for a reader.
4. Re-read the whole `[Unreleased]` block as ONE document before landing a
   branch that edited it more than once. Incremental entries are each true when
   written and collectively false at merge - a squash hides this, and it is the
   most common way the file goes wrong.
5. Group by subsystem, not by Added/Changed/Fixed. Tag a format break
   **(breaking)**.

## Web

1. A widget carries static fallback prose inside its `data-widget` block.
   No-JS readers, search engines and an unregistered key all get the fallback,
   so it has to say something on its own.
2. Every game number on a doc page is lifted from the Rust source, with the
   `file:line` it was verified against on its comment. Never tune a number to
   make the page read better.
3. Run `npm run ci` in `web/` before landing web changes. It is format, lint,
   test and build; `npm ci` first in a fresh worktree, which has no
   `node_modules`.
