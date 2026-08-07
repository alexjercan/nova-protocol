# Conventions - ruled 2026-08-07

The Rust house style for nova-protocol, extracted from what the repo already
writes. **All 12 rules accepted by the owner on 2026-08-07.**

**This file is the evidence record and it stays in the task folder.** The
repo-root `CONVENTIONS.md` is a rewrite of it, not a copy: the rules, one
snippet each, the tool-trap table, and a `## Not yet true` section - 120-150
lines, matching `~/personal/scufris/CONVENTIONS.md` in shape. Everything below
that makes this file 648 lines long (violation counts, file lists, the rulings,
the rejected proposals, the lane placement) is why the extraction is trustworthy
and is not something a contributor needs at the root. It lands in lane L0,
before the benchmark baseline; the full spec is in `notes/17-lanes.md`.

Two rules were ruled against the direction this document proposed. Both are
marked **RULED** in place, with the original recommendation left visible:

| Rule | Proposed | Ruled |
| --- | --- | --- |
| 3 - prelude granularity | one prelude per crate | **per module** - "more copy-pastable" |
| 10 - `SystemSet` | declare one only where something orders against it | **declare one everywhere** - "predictable order" |

Both rulings make the rule *more* work, not less, and both change the lane plan.
See `## What the rulings cost` at the bottom.

Method: read the oldest untouched production files (2026-07-14 .. 2026-07-29),
name the recurring pattern, then count how many places in the current tree
violate it. Counts are measured against HEAD `4a8b55aa`, 2026-08-07.

**How to read a rule.** Each carries a source (the file that does it right), a
counter-example (a file that does not), a violation count, and a marker:

- **[enforceable]** - a clippy lint, rustfmt setting, or test can check it. The
  mechanism is named.
- **[judgment]** - a reviewer has to read it.

A rule with zero violations is not here. If nothing in the tree breaks it, it is
already universal and writing it down costs more than it buys.

---

## 1. Every module opens with a `//!` that says what it owns and when to touch it

**[judgment]** - 28 of 322 non-test files in `crates/` have no module doc at all.

Source - `crates/nova_gameplay/src/input/mod.rs:1-9`:

```rust
//! What decides where a ship goes and when it shoots. Three producers feed the
//! same section inputs: [`player`] (human keybinds, flight verbs, weapon fire),
//! [`ai`] (the enemy behavior state machine), and [`targeting`] (the player's
//! lock/radar system that also derives weapons-safety). [`reference`](mod@reference) exposes
//! the keybind table for the HUD. [`SpaceshipInputPlugin`] adds all three.
//!
//! Touch this module when adding a new way to command a ship. The intents these
//! produce are consumed by the section plugins ([`sections`](crate::sections))
//! and the flight controller ([`flight`](crate::flight)).
```

Two things make this the model, and neither is "describe the contents":

1. It names the module's **job in one sentence** before naming any type.
2. It has a **"touch this module when ..." line** - the routing information a
   cold reader actually needs, and the thing nothing else in the tree provides.

`crates/nova_gameplay/src/asset_ref.rs:1-13` is the other shape worth copying:
it states the **problem** ("in code those are `Handle<A>`; in a hand-authored
RON modding file they must be a *path*") before the solution.

Counter-example: `crates/nova_gameplay/src/hud/nova_os/shell.rs` and 27 others
open straight into `use`.

Note this rule is not covered by `#![warn(missing_docs)]`, which is already on
all 16 crates - `missing_docs` does not require a doc on a private module, and
a one-line restatement of the module name satisfies it.

Proposed shape, three sentences maximum:

```
//! <what this module owns, one sentence>
//!
//! <the constraint or problem that made it exist>
//! Touch this module when <the change that lands here>.
```

## 2. A prelude carries no prose

**[enforceable]** - a grep test. 69 violations.

69 of the 106 `pub mod prelude` blocks in `crates/` carry exactly this line and
nothing else:

```rust
/// Glob-import surface: `use nova_gameplay::asset_ref::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::AssetRef;
}
```

The sentence says nothing the two lines under it do not. It is present because
`#![warn(missing_docs)]` demands a doc on a public module, so it is boilerplate
written to satisfy a lint.

The rule: **CONVENTIONS.md states once, globally, what a prelude is.** An
individual prelude gets a doc only when it has something specific to say. 37
already do, and `crates/nova_ui/src/lib.rs:24-31` shows what "specific" means:

```rust
/// Glob-import surface: `use nova_ui::prelude::*` brings the [`theme`] palette,
/// ... into scope, plus the [`status_bar`] names the composition root spawns.
///
/// The tween names are deliberately absent: one caller registers
/// [`TweenPlugin`](tween::TweenPlugin) by crate path, so a prelude entry would
/// carry no weight.
```

That second paragraph - what is deliberately **out** and why - is the only kind
of prelude doc worth keeping.

Mechanism: a test that greps `crates/` for the exact boilerplate sentence and
fails on any hit. Deleting the 69 requires `#[allow(missing_docs)]` on the
module or, better, keeping a one-line doc that names the contents rather than
restating the mechanism.

**Owner already ruled: delete the boilerplate.** This rule is the generalization
- no doc whose text is derivable from the declaration it sits above.

## 3. Every module that exports items carries a prelude

**RULED 2026-08-07: per module.** Owner: *"I prefer having prelude per module so
it's more copy-pastable."* The heading originally read "Preludes are per crate,
not per module" and proposed the opposite; the evidence that produced the
question is kept below.

**[enforceable]** - a test that every `pub mod` with public items has a
`pub mod prelude`. **80 modules currently lack one.**

The crate root prelude is a list of `<module>::prelude::*` lines, so adding a
public item is a one-line edit inside its own module and never touches the root.
`nova_gameplay/src/lib.rs:87-113` is the model:

```rust
pub mod prelude {
    // Re-export BY NAME, never by glob. A glob over a vendored engine prelude
    // ...
    pub use crate::{
        asset_ref::prelude::*,
        beacon::prelude::*,
        camera::prelude::*,
```

Gap by crate - public modules declared, against preludes present:

| Crate | `pub mod` | preludes | missing |
| --- | --- | --- | --- |
| nova_gameplay | 101 | 76 | 26 |
| nova_probe | 12 | 0 | 13 |
| nova_assets | 13 | 1 | 13 |
| nova_autopilot | 7 | 1 | 7 |
| nova_ui | 8 | 3 | 6 |
| nova_debug | 6 | 1 | 6 |
| nova_os | 4 | 1 | 4 |
| nova_scenario | 17 | 16 | 2 |
| nova_mod_format | 1 | 0 | 2 |
| 6 other crates | 0 | 1 each | 0 |
| **Total** | **170** | **106** | **80** |

The six crates at zero are correct as they are: `nova_core`, `nova_editor`,
`nova_menu`, `nova_modding`, `nova_info` and `nova_events` declare no public
submodules at all - they export a plugin and nothing else.

`nova_ui` is the counter-example that motivated the question: its root prelude
names 40-odd items by hand (`lib.rs:32-51`) while `font.rs` and 5 siblings have
no prelude, so every new public item there is a two-file edit. Under this rule it
converts to the glob form.

**The evidence that made this a question rather than an extraction** - two
architectures were in the tree and the ruling picks one:

- **Glob the module preludes.** 92 sites, `nova_gameplay` + `nova_scenario`.
  Cheap to extend. **This one wins.**
- **Name every item at the crate root.** 14 crates. One hand-maintained list per
  crate; `AGENTS.md:9` ("New public items require prelude exports") assumes it.

**Interaction with rule 2, and it is the one thing this ruling forces.** 80 new
preludes under `#![warn(missing_docs)]` means 80 new module docs, and the path
of least resistance is the boilerplate sentence rule 2 exists to delete. So the
prelude-doc form is settled here, once, for all 186:

```rust
/// The asteroid object config, its spawner and the plugin that registers it.
pub mod prelude {
```

One line, naming **what is in it**. Never the mechanism sentence. A prelude that
deliberately omits something says so in a second paragraph - see `nova_ui`'s
tween note, quoted in rule 2.

`pub mod prelude` count by crate:

| Crate | Preludes |
| --- | --- |
| nova_gameplay | 76 |
| nova_scenario | 16 |
| nova_ui | 3 |
| nova_events | 2 |
| 10 other crates | 1 each (the crate root) |
| nova_probe, nova_events_macros, nova_mod_format | 0 |

Two architectures are in the tree and neither is wrong on its own terms:

- **Crate root globs module preludes.** `nova_gameplay/src/lib.rs:87-113` is
  `asset_ref::prelude::*, beacon::prelude::*, camera::prelude::*, ...`. Adding a
  public item to a module is a one-line edit inside that module. This is what
  drives the 76.
- **Crate root names items directly.** `nova_ui/src/lib.rs:32-51` lists every
  re-export by name, and `nova_ui/src/font.rs` has no prelude of its own. This
  is what `AGENTS.md:9` ("New public items require prelude exports") assumes.

`nova_gameplay/src/lib.rs:79-83` already carries an explicit warning that the
glob form is dangerous:

```rust
    // Re-export BY NAME, never by glob. A glob over a vendored engine prelude
```

Cost of picking the glob form: 92 module preludes, of which 69 carry the
boilerplate doc of rule 2. Cost of picking the by-name form: one large,
hand-maintained list per crate, and 92 module-prelude deletions.

**Owner must rule.** Whichever wins, the rule is "one shape, workspace-wide",
because a cold reader currently cannot predict which crate has which.

## 4. Import through the prelude, not a deep path

**[enforceable]** - a grep test. 90 intra-crate violations; `nova_ui`'s own
prelude has zero in-crate consumers.

`AGENTS.md:103` already says "Imports through crate `prelude`; avoid deep public
paths". Measured:

Counted as `use` **statements** in `crates/`, non-test:

| Form | Count |
| --- | --- |
| `use crate::prelude::...` | 76 |
| `use crate::<module>::...` (deep, intra-crate) | **105** |
| `use nova_x::prelude::*` (cross-crate) | 127 |
| `use nova_x::<module>::...` (deep, cross-crate) | 36 |

So the rule holds 78% of the time across crates and **42% of the time inside a
crate**. `nova_ui` is the sharpest counter-example: it has **0**
`use crate::prelude` anywhere, so its own prelude is exercised only by
downstream crates.

This is the one rule whose violation count makes it a refactor rather than a
cleanup, and it is inseparable from rule 3: a module with no prelude leaves its
consumers no other path than the deep one. Of the 105 violations, **69 import a
module that has no prelude at all** and only 36 bypass a prelude that exists. So
two thirds of this rule is not a violation of it - it is rule 3's missing 80
seen from the consumer side, and it disappears as a side effect of adding the
preludes. Fix the two together, per module. The 36 genuine bypasses are the only
part that is independent work.

**Tool warning:** `clippy::wildcard_imports` (pedantic) flags exactly the form
this rule requires. It must stay out of any lint configuration, or the linter
will silently push the codebase away from its own convention. This is the same
trap `~/personal/scufris/CONVENTIONS.md` names for ruff's `UP` ruleset.

## 5. A doc comment never cites a task artifact

**[enforceable]** - a grep test. 26 violations.

Task folders are not shipped and are not read from the source tree. A doc that
says "see this task's DECISION.md" is unresolvable to every future reader.

Counter-examples, all real:

```rust
// crates/nova_gameplay/src/hud/nova_os/mod.rs:18
//! overlay uses (see this task's DECISION.md - the NOVA OS is a THIRD variant of

// crates/nova_gameplay/src/hud/nova_os_ship/mod.rs:29
//! handler without touching the callers (DECISION fork 4).

// crates/nova_scenario/src/objects/binding_input.rs:1
//! Authoring surface for input bindings (task: scenario config serde).

// crates/nova_probe/src/fixtures.rs:6-7
//! became the third caller and paid for the extraction (task 20260804-094006,
//! DECISION.md D1).
```

Full list, by shape: 9 `DECISION.md` / `DECISION fork`, 8 bare task ids
(`task 20260805-091151`, `task 214617`, `task craft-ships-into-base`), 9 mixed.
`nova_gameplay/src/hud/nova_os*` holds 11 of the 26.

The rule: **state the constraint, not where it was decided.** If the rationale
matters, inline it; if it does not, delete the reference. A task id belongs in
the git message and the task record.

`TODO(<task-id>)` markers are **exempt** - the id there is a live tracker link,
not a citation, and there are only 3 workspace-wide.

## 6. A comment must survive the next refactor

**[judgment]** - the test, not a countable rule. It subsumes rule 5.

**Read `notes/07-comments-and-docs.md` before adding any comment rule.** The
premise "there are useless comments all over the code" was measured twice and
rejected: 83% of a 70-block random sample are why-comments, there is **zero**
commented-out code, there are 3 TODO markers in 155,587 lines, and a strict
what-comment purge yields about 439 lines - 0.3% of the tree. **Do not propose
a purge.** These are the style to preserve:

```rust
// crates/nova_gameplay/src/hud/lock_crosshairs.rs:624
// Registered ONCE so the MessageReader cursor persists across runs

// crates/nova_gameplay/src/audio/sfx.rs:124
// NOTE: rodio does not accept a non-positive playback rate.
```

The real defect is **volume and staleness**. A comment fails this test when it
describes a state of the world rather than a constraint on the code:

| Failure | Instance |
| --- | --- |
| Cites a task artifact | 26 sites - rule 5 |
| Records history for its own sake | `nova_ui/src/theme.rs:130-138` |
| Duplicates a manual | `nova_probe/src/lib.rs` - 100 comment lines in 168, duplicating `.claude/skills/probe/SKILL.md` |
| Restates the declaration | the 69 prelude lines of rule 2 |

Everything in that table is a claim the next refactor invalidates without
touching the comment. Every exemplar above it is a claim the refactor must
preserve or the code breaks.

## 7. A hand-written trait impl states why it is not a derive

**[judgment]** - 6 violations of 53 hand-written impls.

Source - `crates/nova_gameplay/src/asset_ref.rs:25-28`:

```rust
/// `Clone`/`Debug`/`PartialEq`/`Eq` are implemented by hand rather than derived:
/// both variants (`String` and `Handle<A>`) satisfy those traits for every asset
/// type, but a `#[derive]` would wrongly add an `A: Clone` (etc.) bound and
/// exclude asset types like `EffectAsset` that are not themselves `Debug`.
```

Twenty lines of mechanical `match` follow. Without that paragraph the next
reader deletes them and adds `#[derive(Clone, Debug, PartialEq, Eq)]`, and the
build breaks somewhere else entirely.

Measured: 53 hand-written `Clone`/`Debug`/`Default`/`PartialEq`/`Eq`/`Hash`
impls in `crates/`; **44 carry an explanatory comment within 8 lines.** The 6
genuine bare ones:

`nova_autopilot/src/autopilot.rs:109`, `nova_autopilot/src/completion.rs:93`,
`nova_gameplay/src/settings.rs:239`,
`nova_gameplay/src/sections/controller_section.rs:137`,
`nova_menu/src/settings_store.rs:53`, `nova_os/src/terminal/state.rs:177`.

(Three further bare sites in `asset_ref.rs` itself are covered by the type doc
above and are not violations.)

83% adherence makes this the most settled unwritten rule in the tree. Writing it
down costs six comments.

## 8. A lint suppression is an `#[expect]` with a reason

**[enforceable]** - `unfulfilled_lint_expectations`, on by default. 38
violations.

The rule: **`#[expect(lint, reason = "...")]`, never bare `#[allow(lint)]`.**
Current state is 54 `#[allow(...)]` against 6 `#[expect(...)]`, and 38 of the
allows are the same lint:

```rust
// crates/nova_gameplay/src/hud/keybind_dock.rs:569 - the model
#[expect(clippy::type_complexity, reason = "one query per chip part")]
```

Two facts make this cheap and worth doing, both verified 2026-08-07:

**a. The 38 `#[allow(clippy::type_complexity)]` attributes are already dead.**

```toml
# Cargo.toml:314-316
[workspace.lints.clippy]
type_complexity = "allow"
too_many_arguments = "allow"
```

All 17 manifests carry `[lints] workspace = true`, so the lint is allowed
workspace-wide and those 38 attributes suppress something that cannot fire.
They are pure noise today.

**b. `#[expect]` overrides the workspace `allow` at the site.** This is what
makes the conversion worth more than a deletion, and it is observed, not
assumed: the four existing `#[expect(clippy::type_complexity, ...)]` sites
(`keybind_dock.rs:569,737,790`, `input/player/hints.rs:200`) coexist with the
workspace `allow`, and clippy reports **0 warnings** at
`--workspace --all-targets --features debug`. If `#[expect]` did not re-enable
the lint locally, every one of them would be an unfulfilled expectation and
would warn.

So converting 38 no-op allows into 38 expectations turns each one into a
self-auditing claim: the moment a refactor simplifies the signature, rustc
reports the suppression as stale, for free, with no change to the workspace
lint table. Two are already stale by eye (`hud/ammo_readout.rs:325`, `:510`).

The remaining 16 `#[allow(...)]` get the same treatment or a justifying reason,
with one legitimate exception: `#[allow(missing_docs)]` at
`nova_assets/src/portal/mod.rs:108` is a deliberate escape hatch, and
`#[expect]` there would be fulfilled or not depending on unrelated edits.

**This supersedes the version of this recommendation in `notes/07` and
`notes/13`.** Both were right about the conversion; neither knew the workspace
`allow` was already in place, so both undercounted how little the change costs.

## 9. `SystemSet` types are named `<Subsystem>Systems`

**[enforceable]** - a grep over `derive(SystemSet)` sites. 2 violations of 30.

```rust
// crates/nova_gameplay/src/input/mod.rs:29-32
/// System set holding all input production (player, AI, targeting), ordered
/// first among the gameplay sets so downstream sections/flight read fresh intent.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipInputSystems;
```

28 of 30 follow it: `NovaFlightSystems`, `IntegritySystems`,
`ChaseCameraSystems`, `TweenSystems`, ... The two that do not are
`HudSituationSensing` and `CameraAuthority` (which additionally collides in
spirit with `CameraAuthorityPlugin`).

Two violations makes this a cleanup, not a refactor. Include it only if you want
the name to be a reliable search key - `grep 'Systems$'` is currently 93%
accurate.

Note the sibling rule needs **no** entry: **all 98 `impl Plugin for` types end
in `Plugin`, with zero exceptions.** Already universal, so it does not go in.

## 10. Every subsystem plugin declares a `SystemSet` and orders it

**RULED 2026-08-07: declare one everywhere.** Owner: *"let's use all system sets
just to have a predictable order."* The heading originally read "A `SystemSet`
exists to be ordered against" and proposed declaring one only where something
outside needs to order against it. The ruling goes further: the set is the
default, and the ordering is the point.

**[enforceable]** - a test asserting every `impl Plugin` type has a matching
`SystemSet` reachable from a `configure_sets` call. **68 plugins have no set,
and 16 of the 30 existing sets are never ordered.**

```rust
// crates/nova_gameplay/src/input/mod.rs:29-32 - the model
/// System set holding all input production (player, AI, targeting), ordered
/// first among the gameplay sets so downstream sections/flight read fresh intent.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipInputSystems;
```

Current state:

| | Count |
| --- | --- |
| `impl Plugin for` types | 98 |
| `SystemSet` types | 30 |
| `configure_sets` calls | 21 |
| Sets ever passed to `configure_sets` | **14** |
| **Sets declared but never ordered** | **16** |

The 16 that tag systems without ordering anything:
`DirectionalSphereOrbitSystems`, `HudSituationSensing`, `IntegritySystems`,
`NovaOsMapSystems`, `NovaOsShipSystems`, `ObjectivesPluginSystems`,
`PointRotationSystems`, `SmoothLookRotationSystems`, `SpaceshipTargetingSystems`,
`SphereOrbitSystems`, `SphereRandomOrbitSystems`, `StatusBarPluginSystems`,
`TempEntitySystems`, `TurretSectionAimSystems`, `TweenSystems`,
`WASDCameraControllerSystems`.

(`NovaOsMapSystems` and `NovaOsShipSystems` are finding **F53** in
`16-findings-master.md`. The measurement above shows F53 is not two sites - it
is 16.)

Counter-example on the other side, `crates/nova_gameplay/src/plugin.rs:73-101` -
13 leaf plugins added in sequence with no set and no declared order, each with a
comment that restates its own name:

```rust
        // Point Rotation Plugin to convert linear movement to a target rotation
        app.add_plugins(crate::transform::prelude::PointRotationPlugin);
        // for debug to have a random orbiting object
        app.add_plugins(crate::transform::prelude::SphereRandomOrbitPlugin);
        // Rotation Plugin for the turret facing direction
        app.add_plugins(crate::transform::prelude::SmoothLookRotationPlugin);
        // Sphere Orbit Plugin
        app.add_plugins(crate::transform::prelude::SphereOrbitPlugin);
```

Today their relative order is whatever the scheduler picks. Under this rule it
is written down, which is what makes a refactor that moves them provably safe.

**Scope warning.** As ruled, this is not a cleanup - it is 68 new sets and, more
expensively, 68 ordering decisions, most of which nobody has ever had to make.
Where it lands is in `## What the rulings cost`.

**Original text, kept for the record:** *"a subsystem plugin declares a
`SystemSet` when anything outside it needs to order against it, and nothing else
needs one."* Rejected.

`AGENTS.md:101` says "One plugin per subsystem; group systems with `SystemSet`".
The first half holds; the second is where the ratio breaks down.

Counter-example, `crates/nova_gameplay/src/plugin.rs:73-101` - 13 leaf plugins
added in sequence, each with a comment that restates its own name:

```rust
        // Point Rotation Plugin to convert linear movement to a target rotation
        app.add_plugins(crate::transform::prelude::PointRotationPlugin);
        // for debug to have a random orbiting object
        app.add_plugins(crate::transform::prelude::SphereRandomOrbitPlugin);
        // Rotation Plugin for the turret facing direction
        app.add_plugins(crate::transform::prelude::SmoothLookRotationPlugin);
        // Sphere Orbit Plugin
        app.add_plugins(crate::transform::prelude::SphereOrbitPlugin);
```

Compare `input/mod.rs:38-46`: three `add_plugins` lines, no comments, and a
`SpaceshipInputSystems` set that states the ordering contract in its doc.

Proposed rule: **a subsystem plugin declares a `SystemSet` when anything outside
it needs to order against it, and nothing else needs one.** The four-line block
above is a plugin-shaped module list, and the fix is a `transform` bundle plugin
- but that is arrangement, so it belongs to the refactor task, not here.

Accept this only as a going-forward rule. It has no bounded violation count.

## 11. Tests live inline in `#[cfg(test)] mod tests`

**[judgment]** - 202 files inline; 4 modules use a `tests/` directory.

The overwhelming default is an inline `#[cfg(test)] mod tests` at the bottom of
the file - `asset_ref.rs:133-162` is the model, and its three test names read as
sentences (`path_ref_round_trips_through_ron_as_a_bare_string`).

The four exceptions are all large modules that outgrew it:

```
crates/nova_menu/src/tests
crates/nova_gameplay/src/flight/tests
crates/nova_gameplay/src/hud/nova_os/tests
crates/nova_assets/src/scenario/shakedown/tests
```

That is a coherent escalation, not a disagreement: inline until the test module
is bigger than the code, then a sibling `tests/` directory. Both are unit tests
inside `src/`; `crates/*/tests/` remains integration tests, per `AGENTS.md:80`.

**This may be a fact rather than a rule.** Nothing is currently wrong. Include
it only to spare the next contributor the decision.

## 12. Formatting is `rustfmt.toml`, and it is not negotiable

**[enforceable]** - `cargo fmt --check`, already wired into the pre-commit hook
via `scripts/setup-hooks.sh`. 0 violations.

```toml
edition = "2021"
reorder_imports = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

Zero violations, so by this document's own rule 2 it should not be here. It is,
for one reason: **`imports_granularity` and `group_imports` are nightly-only
rustfmt options.** They are silently ignored on stable, so a contributor running
stable `cargo fmt` will not produce the layout in the tree and will not be told
why. That is worth one sentence in CONVENTIONS.md.

---

## Tools that would undo these conventions

The scufris file ends by naming the ruleset that would silently reverse its own
convention. The Rust equivalents, measured in `notes/09-clippy-and-lints.md`:

| Lint / setting | Would break | Hits if enabled |
| --- | --- | --- |
| `clippy::wildcard_imports` (pedantic) | rule 4 - it flags every `use ...::prelude::*` | the whole prelude architecture |
| `clippy::redundant_pub_crate` (nursery) | tells you to weaken deliberate `pub(crate)` | 1,270 |
| `clippy::needless_pass_by_value` (pedantic) | fires on every Bevy system parameter taken by value | 1,366 |
| `clippy::missing_docs_in_private_items` | would re-create rule 2's boilerplate, for private items this time | not measured |

Those two nursery/pedantic lints alone are 66% of pedantic output. **Do not
enable `clippy::pedantic` or `clippy::nursery` wholesale.** Plain clippy is
already clean at `--workspace --all-targets --features debug` (0 warnings), so
`-D warnings` in CI is free today - see `notes/09`.

## What the rulings cost

All 12 rules are accepted, so every violation count above is now scheduled work.
It does not form a lane of its own. Two of these rules are large enough to
change the shape of a lane that already exists, and the rest are small enough
that they should ride along with whoever is already reading the file.

The deciding question for each is the same one `17-lanes.md` uses everywhere -
**does it move a file, rename a symbol, or edit a doc?** If yes it blocks the
benchmark baseline and lands after it; if no it is free.

| Rule | Work | Sites | Baseline | Lands in |
| --- | --- | --- | --- | --- |
| 8 | `#[allow]` -> `#[expect(reason)]` | 38 | before | **L0** - already scheduled as F80 |
| 12 | note nightly-only rustfmt keys | 0 | before | **L0** - one sentence |
| 11 | no work, documents current practice | 0 | - | **L0** - the doc itself |
| 2 | delete the prelude boilerplate doc | 69 | after | **L5** - already scheduled |
| 5 | rewrite docs citing task artifacts | 26 | after | **L5** |
| 7 | one comment per bare hand-written impl | 6 | after | **L5** |
| 9 | rename 2 `SystemSet` types | 2 | after | **L5** |
| 1 | write the 28 missing module docs | 28 | after | **L5** |
| 4 | route deep imports through the prelude | 36 (+69 via rule 3) | after | **L8 / L9 / L10** - per crate |
| 3 | add the 80 missing module preludes | 80 | after | **L8 / L9 / L10** - per crate |
| 6 | the test the other doc rules are judged by | - | - | not a task |
| 10 | 68 new `SystemSet`s, 16 unordered sets | 84 | after | **L9** - see below |

Three things follow, and they are the reason this is not one "chores" lane:

**Rule 10 is not a chore, and it must not be done before the split.** As ruled,
it is 68 new sets plus ordering decisions for all of them. Sixty-eight ordering
decisions made across `nova_gameplay` as it stands today are sixty-eight
decisions re-made the moment L9 cuts it into CORE / FLIGHT / HUD / NOVAOS -
because "what runs before what, across this boundary" is *precisely* the
question the seam forces. Doing it first means doing it twice. It belongs inside
L9, per seam, as the artifact that proves the seam is real. The 16 already-
declared-but-unordered sets are the natural first slice, and `NovaOsShipSystems`
/ `NovaOsMapSystems` are already in L9's cluster note as F53.

**Rules 3 and 4 are the same edit.** Adding a module's prelude and switching its
consumers off the deep path is one pass over one module, not two. They are also
the same edit as the visibility audit L9 already absorbs ("splitting four ways
forces each seam to decide what crosses its boundary") - deciding what goes in a
module's prelude *is* deciding what crosses its boundary. `nova_probe`'s share
(13 preludes, 184 deep-path imports, the worst in the workspace) is already
named in L8. Do not schedule a workspace-wide prelude pass; let each structural
lane pay for its own crates.

**The rest is one afternoon, and it goes in L5.** Rules 1, 2, 5, 7 and 9 are 131
sites of pure prose-and-rename work with no behavioral risk. They share L5's
constraint exactly (all block the baseline, all land after it) and L5 is already
the lane holding the 69 boilerplate lines. The one thing to preserve is L5's
purpose: deletion count is success criterion #2, so rule 1 (which *adds* 28
module docs) should be counted separately from the deletions rather than
allowed to net against them.
