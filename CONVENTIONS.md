# Conventions

The Rust house style for nova-protocol, extracted from what the repo already
writes. Binding on every crate. `AGENTS.md` owns what is not style.

## 1. Open every module with a `//!` saying what it owns and when to touch it

```rust
//! What decides where a ship goes and when it shoots. Three producers feed the
//! same section inputs: [`player`], [`ai`] and [`targeting`].
//!
//! Touch this module when adding a new way to command a ship.
```

The job in one sentence, the constraint that made the module exist, then a
"touch this module when ..." line. Three sentences maximum. Intra-doc links for
reachable types, wiki links for concepts. `missing_docs` asks for none of it.

## 2. Write no prose a declaration already carries

```rust
// WRONG - says nothing the declaration under it does not
/// Glob-import surface: `use nova_gameplay::asset_ref::prelude::*` re-exports
/// the public API of this module.
pub mod prelude {
```

A doc exists to say what the code cannot. A prelude gets one line naming what is
in it, plus a second paragraph only when something is deliberately left out
(`nova_ui/src/lib.rs` explains why the tween names are absent).

## 3. Give every module that exports items a prelude

```rust
/// The asteroid object config, its spawner and the plugin that registers it.
pub mod prelude {
    pub use super::{AsteroidConfig, AsteroidPlugin};
}
```

The crate root is then a list of `<module>::prelude::*` lines
(`nova_gameplay/src/lib.rs`), so a new public item is a one-line edit inside its
own module. Re-export BY NAME where a glob would pull in an engine prelude.

## 4. Import through the prelude, not a deep path

```rust
use crate::prelude::*;              // yes
use crate::terminal::shell::*;      // no
```

A module's prelude is its public boundary; a deep path reaches past it and
freezes somebody else's internal layout into your file. Inside a crate too.

## 5. Never cite a task artifact from a doc comment

```rust
//! handler without touching the callers (DECISION fork 4).  // WRONG
```

Task folders are not shipped and are not read from the source tree. State the
constraint inline, or delete the reference. `TODO(<task-id>)` is exempt: a live
tracker link, not a citation.

## 6. Write comments that survive the next refactor

```rust
// Registered ONCE so the MessageReader cursor persists across runs
// NOTE: rodio does not accept a non-positive playback rate.
```

Both are constraints: break them and the code breaks. A comment that records
history, restates a declaration or duplicates a manual is invalidated by the next
edit without anyone touching it. This is the test rules 2 and 5 are cases of. No
purge is owed: the tree is 83% why-comments with zero commented-out code.

## 7. Say why a hand-written trait impl is not a derive

```rust
/// `Clone`/`Debug`/`PartialEq`/`Eq` are implemented by hand rather than derived:
/// a `#[derive]` would wrongly add an `A: Clone` (etc.) bound and exclude asset
/// types like `EffectAsset` that are not themselves `Debug`.
```

Twenty lines of mechanical `match` follow. Without that paragraph the next reader
deletes them, adds the derive, and the build breaks somewhere else.

## 8. Suppress a lint with `#[expect]` and a reason, never a bare `#[allow]`

```rust
#[expect(clippy::type_complexity, reason = "one query per chip part")]
```

`#[expect]` overrides the workspace `allow` at the site, turning the suppression
into a self-auditing claim: simplify the signature and
`unfulfilled_lint_expectations` reports it stale, for free. It found 12 dead
suppressions the moment it was switched on. `#[allow(missing_docs)]` at
`nova_assets/src/portal/mod.rs` is the one exception.

## 9. Name `SystemSet` types `<Subsystem>Systems`

```rust
pub struct SpaceshipInputSystems;  // not HudSituationSensing, not CameraAuthority
```

28 of 30 already do, so `grep 'Systems$'` is a reliable search key for the
scheduling structure. All 98 `impl Plugin for` types already end in `Plugin`.

## 10. State every ordering constraint; declare a `SystemSet` when another plugin needs the handle

```rust
// A constraint with one holder states itself on the system.
app.add_systems(Update, draw_juice_flashes.after(TransformSystems::Propagate));

/// A constraint with an outside holder needs a name to order against.
/// Ordered first among the gameplay sets so downstream sections/flight read
/// fresh intent.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipInputSystems;
```

Both halves are load-bearing. An order your systems depend on but never state is
a bug that hides until the scheduler's tie-break changes, so writing it down is
what makes a refactor that moves systems provably safe. But a set nothing is
ordered against records nothing: it is a handle with no holder, and the compiler
cannot tell you when it stops describing its plugin. So the set is the
consequence of a cross-plugin edge, never the goal. `CameraAuthoritySystems`,
`SpaceshipSectionSystems`, `NovaHudSystems` and `TurretSectionAimSystems` all
exist because another plugin had to order against them; every one of this
repo's good sets was born that way.

Two violations, both greppable: a plugin whose systems depend on order it does
not state, and a set with no `.after`/`.before` referencing it from outside its
own plugin.

## 11. Put tests inline in `#[cfg(test)] mod tests`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn path_ref_round_trips_through_ron_as_a_bare_string() { /* ... */ }
```

202 files do this; 4 large modules escalated to a sibling `src/**/tests/`
directory once the test module outgrew the code. Both are unit tests inside
`src/`; `crates/*/tests/` stays integration tests. Names read as sentences.

## 12. Take formatting from `rustfmt.toml`

```toml
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

`cargo fmt --check` is in the pre-commit hook, at zero violations. Those two keys
are **nightly-only** and are silently ignored on stable, so stable `cargo fmt`
will not reproduce the tree's layout and will not say why. `rust-toolchain.toml`
pins nightly for this reason.

## Tools that would undo these conventions

Plain clippy is clean at `--workspace --all-targets --features debug`, which is
what lets CI run it with `-D warnings`. Do **not** enable `clippy::pedantic` or
`clippy::nursery` wholesale: two of their lints are 66% of that output and both
argue against this file.

| Lint | Would break | Hits |
| --- | --- | --- |
| `clippy::wildcard_imports` (pedantic) | rule 4 - flags every `use ...::prelude::*` | the whole prelude architecture |
| `clippy::redundant_pub_crate` (nursery) | tells you to weaken deliberate `pub(crate)` | 1,270 |
| `clippy::needless_pass_by_value` (pedantic) | fires on every Bevy system parameter taken by value | 1,366 |
| `clippy::missing_docs_in_private_items` | re-creates rule 2's boilerplate, for private items | not measured |

## Not yet true

Open sites, re-measured 2026-08-08 after the three seam cuts. Each is scheduled
work in the `nova_*` refactor epic, not licence to fix inside an unrelated diff.
**This section is deleted when it empties; its emptiness is the proof the rules
above are real.**

| Rule | Open sites | Where | Closed by |
| --- | --- | --- | --- |
| 3 - a prelude per exporting module | 15 | `nova_assets` 9, `nova_scenario` 2, `nova_ui` 2, `nova_gameplay::test_support`, `nova_probe_cli::native` | L7, L10 |
| 4 - import through the prelude | 25 | `nova_ship` 8 (all `#[cfg(test)]`), `nova_scenario` 7, `nova_probe_cli` 4, `nova_ui` 4, `nova_assets` 1, `nova_editor` 1 | with rule 3 |
| 1 - module doc | 1 | | L10 |

Rules 3 and 4 are one edit per module, not two: two thirds of rule 4's
violations import a module that has no prelude at all.

Rule 3 is measured over modules that are a BOUNDARY. A `pub mod` whose parent's
prelude already re-exports its items is not one, and the fix there is to make the
module private, not to give it a second prelude nobody imports through - that is
how 14 of `nova_ship`'s sites closed.

Rule 10 is absent from this table because it closed at zero, not because it was
skipped. It was rewritten (owner ruling, 2026-08-08) after measurement showed
its old wording had 22 open sites and that all 22 wanted no set: 16 leaf plugins
nothing can order against, and 6 sets whose only consumers sit in a different
schedule. Those 6 were deleted, joining `WASDCameraControllerSystems` which L9.6
had already removed on the same reasoning.
