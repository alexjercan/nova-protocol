# L5 - Delete the dead and lying surface

**Baseline: BLOCKS - lands AFTER it.** Deletion count is success criterion #2;
lines deleted before the baseline never enter the ledger. This is the exact
opposite of L0, and the two are easily confused because both are cheap
doc-ish work.

Findings: **F45, F46, F47, F48, F49, F50, F51, F52, F54, F55**, plus the
CONVENTIONS.md prose sweep (rules 1, 2, 5, 7, 9 and 19 orphaned preludes).

**Depends on:** L2.

## Deletions, largest first

### F45 - the whole `Tween` subsystem, 421 lines + 11 tests

```rust
DELETE  crates/nova_ui/src/tween.rs            (421 lines, 11 tests)
DELETE  crates/nova_ui/src/lib.rs              the `pub mod tween;` line + prelude re-exports
CHANGE  crates/nova_gameplay/src/hud/mod.rs:301  drop `.add_plugins(TweenPlugin)`
```

Zero consumers workspace-wide - verified 2026-08-07, no `Tween<T>` is spawned
outside the module's own tests. `TweenPlugin` runs four empty queries every
frame. Latent defect if it ever gained a consumer: `tween.rs:243` inserts
`TweenFinished` and nothing removes it, so it latches.

**Do this first inside the lane.** It makes F55 cheaper (a two-plugin merge
instead of three) and retires `TweenSystems` before L9 counts rule 10's sets.

### F46, F51 - `nova_ui/src/status_bar.rs`

```rust
// DELETE  status_bar.rs:131-136 + the init_resource at :153
#[derive(Resource, Default, Clone)]
pub struct StatusBarStore {
    pub store: HashMap<Entity, Arc<dyn StatusValue>>,
}
//   Declared, init_resource'd, and NEVER read or written. Those two lines are
//   the only hits workspace-wide. The per-entity staging it documents is
//   actually done by StatusBarItemValue (:129).
```

```rust
// CHANGE  status_bar.rs:238 insert_status_bar_item        (F51)
//   The entity the caller spawns with `status_bar_item` is never parented and
//   never rendered - the observer copies its data into a brand-new child of
//   the root, leaving the caller's entity a permanent orphan with no Node.
//   nova_core/src/lib.rs:290,297 spawns two.
//   FIX: build the item UI as children OF the caller's entity and reparent it
//   under the root, so the returned handle is the live entity. Any future
//   "remove this metric" code operating on it is a silent no-op today.
```

**F41 is in the same file and is behavior-only, so it sits in L4.** Read the
file once; hold this commit for the baseline. See `lane04.md`.

### F47 - the advertised headless mode does not exist

```rust
// crates/nova_gameplay/src/plugin.rs:40
pub struct NovaGameplayPlugin { pub render: bool, ... }
//   documented as gating "meshes, HUD, particles", forwarded to ONE plugin
//   (:109). Hanabi (:77), skybox (:85), post (:86) and the entire HUD (:111)
//   are unconditional.
```

**RULED 2026-08-07: make it real.** Gate `:77` (hanabi), `:85` (skybox), `:86`
(post) and `:111` (the HUD) on `render`, so the documented headless mode exists
and HUD-free tests become possible. Deleting the field was the alternative and
was rejected.

This is the one item in this lane that **adds** capability rather than removing
surface, and it changes what a probe run builds - so `probe run --all` before
and after is the verification, not the compiler.

### F48 - a system that can never run

```rust
DELETE  crates/nova_gameplay/src/objectives.rs:123  rebuild_lines
//   ObjectivesPanelMarker appears ONLY inside objectives.rs (bundle, Single
//   query, its own unit test) - verified, 4 hits, all in that file. The live
//   objectives HUD is a separate panel
//   (nova_scenario/src/loader/lifecycle.rs:49-63).
//   ObjectivesPlugin's only system is a permanent no-op - delete the plugin too.
```

### F49, F50 - filters and parameters that lie

```rust
// crates/nova_gameplay/src/sections/torpedo_section/bay.rs:112   (F49)
Without<SectionInactiveMarker>
//   can never exclude anything: integrity/glue.rs:49 is the only writer and is
//   guarded by With<SectionMarker>, which the spawner does not have. Disable a
//   torpedo bay in place and its cooldown keeps ticking to ready.
//   FIX: make the filter real (tag the spawner) or delete it. It currently
//   READS AS A LIVE SAFETY GATE and does nothing.

// crates/nova_ui/src/widget/panel.rs:112                          (F50)
pub fn panel_head(title: &str, tag: Option<&str>, _skin: UiSkin) -> impl Bundle
//                                                 ^^^^^ discarded
//   Switching to Hardware repaints the panel to grey but leaves the header a
//   green CRT band - BorderColor::all(theme::PHOSPHOR..) at :121 is hardcoded.
//   FIX: honor the skin. Deleting the parameter is the wrong fix; every call
//   site believes it does something.
```

**The rule this proves:** an unused parameter or an inert filter must be
**removed, never renamed to `_`** - the signature is what the caller believes.
Route to `../conventions-prompt.md`.

### F52, F54, F55

```rust
// crates/nova_debug/Cargo.toml:18 + root Cargo.toml:224        (F52)
//   nova_debug hard-forces nova_gameplay/debug and the root dev-depends on it
//   unconditionally, so EVERY cargo test and example build compiles gameplay
//   with debug on regardless of flags. nova_info additionally declares a
//   `debug = []` feature with ZERO cfg sites - delete that one outright.
//   Whoever did F79 in L0 already learned what --features debug builds;
//   read their notes rather than repeating the investigation.

// crates/nova_debug/src/lib.rs:124, inspector.rs:180, wireframe.rs:66  (F54)
DELETE two of the three private toggle_debug_mode fns
//   All three registered, all toggling the same DebugEnabled on the same F11.
//   Works only because three flips of a bool is still a flip - lib.rs:110
//   comments "they stay in phase". A FOURTH sub-plugin silently breaks the key.

// crates/nova_ui/src/widget/register + WidgetObserversRegistered        (F55)
NEW     pub struct NovaUiPlugin;   // one plugin, replacing:
DELETE  the first-caller-wins `register` fn + WidgetObserversRegistered resource
CHANGE  status_bar.rs:147 StatusBarPlugin  -> folded in
//   (tween.rs:198 TweenPlugin is already gone via F45)
```

## The CONVENTIONS.md prose sweep - 131 sites

Five accepted rules are pure prose-and-rename work with no behavioral risk, and
they share this lane's constraint exactly: all block the baseline, all land
after it.

| Rule | Work | Sites |
| --- | --- | --- |
| 2 | delete the prelude boilerplate doc line | **69** |
| 1 | write the missing module docs (`//!` + a "touch this module when ..." line) | 28 |
| 5 | rewrite docs citing a task artifact (`DECISION.md`, bare task ids) | 26 |
| 7 | one comment per bare hand-written trait impl, saying why it is not a derive | 6 |
| 9 | rename `HudSituationSensing` -> `HudSituationSensingSystems`, `CameraAuthority` -> `CameraAuthoritySystems` | 2 |

Rule 2's count **corrects the 91 figure**: of 106 prelude docs, **69** are the
exact boilerplate sentence and **37** say something specific
(`nova_ui/src/lib.rs:24-31` is the model). Delete 69, keep 37.

**Count rule 1 separately from the deletions.** It *adds* 28 module docs and
nets against success criterion #2. Report the two numbers, not one.

### The 19 orphaned preludes (rules 3 and 4)

The 80 missing module preludes are paid for by whichever structural lane opens
the crate. These four crates have no structural lane:

```
NEW  crates/nova_autopilot/src/<module>/prelude.rs   x7
NEW  crates/nova_debug/src/<module>/prelude.rs       x6
NEW  crates/nova_os/src/<module>/prelude.rs          x4
NEW  crates/nova_mod_format/src/<module>/prelude.rs  x2
```

Each is one prelude module plus a one-line doc naming its contents - **never
the boilerplate sentence rule 2 deletes.**

## Verified by

The compiler for the deletions. `probe run --all` for F47, because making the
headless mode real changes what a run builds. A double-registration check in
the menu and editor apps for F55.
