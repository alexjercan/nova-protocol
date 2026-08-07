# L4 - Reconciler discipline and terminal input

**Baseline: NEUTRAL.** Behavior-only. **Must precede L9's NOVAOS seam move**,
or these defects get fixed after 14.3k lines have shifted and every citation in
`../notes/10-review-hud-nova-os.md` has to be re-derived.

Findings: **F15, F34** (missing Control guards), **F18** (the `f32::MAX`
sentinel), **F19, F20, F75** (stale `Local<T>`), **F39, F40, F41, F42**
(unguarded per-frame writes), **F16** (aborted explosion), **F21** (audio loops
through scenario load), **F23** (torpedo anchor), **F33, F73, F74** (terminal).

**Depends on:** L1.

## The reference implementation

`crates/nova_gameplay/src/hud/keybind_dock.rs` is the most careful reconciler
in the tree: `set_if_neq` throughout, guarded `Node` writes, `Added<DockChip>`
overrides, real `.after()` edges, and `:537` carries the explanatory comment.
**Every fix in this lane is "make the site look like keybind_dock".**

## Cluster 1 - five defects, three files, one pass

F18, F19, F39, F40 and F42 live in `hud/nova_os/shell.rs`, `input.rs` and
`crt.rs`. Fixed separately that is three files read five times. **Largest
single-lane saving in the epic after F38.**

### F19 - stale `Local<usize>` across a respawn

```rust
// crates/nova_gameplay/src/hud/nova_os/shell.rs:363  (today)
mut last_len: Local<usize>,
...
let len = terminal.scrollback().len();
if len > *last_len { scroll.0.y = f32::MAX; }
*last_len = len;
//   remove_nova_os (spawn.rs:710) calls terminal.reset_session() back to 6
//   welcome rows while last_len stays at the old session's 200, so auto-scroll
//   stays dead for the next ~190 rows after a respawn.
```

```rust
// CHANGE  shell.rs:344 rebuild_terminal_ui - add the override that its two
//         siblings in this same file already carry
//         (reconcile_nova_os_header :288, rebuild_nova_os_footer_hints :320)
    q_added: Query<(), Added<NovaOsTerminalRootMarker>>,
//   just-spawned => treat last_len as 0 regardless of what it holds.
//   Recurrence of the `mode-keyed-reconciler-just-spawned-override` memory.
```

### F18 - the sentinel that is never cleared

```rust
// crates/nova_gameplay/src/hud/nova_os/shell.rs:379
scroll.0.y = f32::MAX;
//   Bevy writes the CLAMPED value to ComputedNode via bypass_change_detection
//   and never back to ScrollPosition (layout/mod.rs:365-369). (f32::MAX + -page)
//   is still f32::MAX in f32, so PageUp after a command needs two presses.
```

```rust
// CHANGE  clamp at the point of writing instead of pinning with a sentinel
scroll.0.y = max_nova_os_scroll_y(computed_node);
//   L7 gives this function its unit fix (F17); the sentinel removal is
//   independent of that and lands here.
```

### F40 - 4,800 entities respawned per 12 keystrokes

```rust
// crates/nova_gameplay/src/hud/nova_os/shell.rs:344
//   rebuild_terminal_ui despawns and respawns EVERY scrollback row whenever
//   NovaOsTerminal changes, and every edit goes through ResMut, so DerefMut
//   marks it changed on each keystroke - including caret movement, which
//   changes nothing on screen. Nothing trims scrollback.
```

Three changes, in order of payoff:

1. Rebuild rows only when `scrollback().len()` or the tail content changed -
   caret movement must not reach the row loop.
2. Bound `scrollback` (`nova_os::Terminal` owns it; see F74's cap).
3. Prompt/hint/ghost writes at `:385-400` go through `set_if_neq`.

### F39, F42 - unguarded `DerefMut` on `Node` and colors

```rust
// crates/nova_gameplay/src/hud/nova_os/crt.rs:219   (F39)
//   reconcile_nova_os_target writes node.width/node.height unconditionally.
//   The only gate is resource_exists::<NovaOsRtt>, which lives from ship spawn
//   to despawn - so it runs EVERY FRAME while the player is flying and the
//   monitor is hidden, over a subtree of hundreds of Text children.

// crates/nova_gameplay/src/hud/nova_os/shell.rs:442                  (F42)
pub(crate) fn position_nova_os_block_caret(
    q_before: Query<&ComputedNode, With<NovaOsTerminalPromptMarker>>,
    mut q_caret: Query<&mut Node, With<NovaOsTerminalCaretMarker>>,
)   //   node.left written every frame while open

// crates/nova_gameplay/src/hud/nova_os_ship/scene.rs:750,772          (F42)
//   unconditional TextColor / BorderColor / BackgroundColor writes in a
//   function that already guards its Text write two lines above -
//   inconsistent rather than deliberate.
```

Fix shape everywhere:

```rust
// NEW  a shared helper if one does not already exist near keybind_dock
fn set_val_if_neq(field: &mut Val, next: Val) -> bool
//   or just `if *field != next { *field = next; }` inline - the point is that
//   the Mut<Node> deref only happens when the value actually differs.
```

## Cluster 2 - F19, F20, F75 make the CONVENTIONS rule citable

Three sites of one pattern. Fixing them together lets the rule cite both the
violation count **and** the fix, in-repo. Fixing one and writing the rule from
it is weaker evidence.

```rust
// crates/nova_gameplay/src/audio/cues.rs:99   (F20)
//   play_safety_engaged_cue's Local<bool> is process-global and survives the
//   death of the ship it tracked, contradicting its own doc at :93. Die while
//   WeaponsHot(true): the new ship's default false matches Changed<WeaponsHot>
//   on frame 1, so a safety-engage click plays with nothing disarmed.
//   FIX: key on the ship Entity, or an Added<> override.

// crates/nova_gameplay/src/audio/cues.rs:147  (F75)
//   play_dry_fire_cue's Local<HashMap<Entity, bool>> is never pruned for
//   despawned turrets. Memory only - entity generations mean no stale latch.
//   FIX: the sibling prune already exists - mixing.rs:195 prune_sfx_throttle.
fn prune_dry_fire_state(...)   // NEW, modelled on prune_sfx_throttle
```

## Cluster 3 - F15 and F34, the same bypassed guard

```rust
// crates/nova_gameplay/src/hud/nova_os/input.rs:267   (F15)
Key::Character(_) | Key::Space => { ... terminal.insert_text(text) ... }
//   NO Control guard, so Ctrl+C, Ctrl+U, Ctrl+W, Ctrl+A, Ctrl+K all insert a
//   literal character at the nova> prompt. handle_nova_os_app_keyboard
//   (:355,:374) deliberately skips Control-held events; the prompt handler
//   chained immediately before it does not.
//   MOST LIKELY FINDING IN THE SET TO BE HIT BY A REAL PLAYER.

// crates/nova_gameplay/src/hud/nova_os_ship/scene.rs:397  (F34)
//   ship_input reads raw ButtonInput<KeyCode>, bypassing the app router's
//   Control guard, so Ctrl+[ both exits the app AND cycles selection back.
```

```rust
// CHANGE  input.rs:267 - skip when Control is held, matching the sibling
+ if event.modifiers.control { /* fall through to the app router */ }
// NEW - and, since three handlers now need the same question:
fn control_held(event: &KeyboardInput) -> bool
```

Deciding whether to *implement* the chords (Ctrl+U kill-line, Ctrl+W kill-word)
is a separate question. **Not inserting a literal character is the fix**;
implementing the chords is optional scope.

## Terminal - F33, F73, F74

```rust
// crates/nova_os/src/terminal/view.rs:222      (F33)
//   prompt_completion_ghost strips the prefix off the UNTRIMMED prompt while
//   refresh_parse (edit.rs:338) uses the trimmed one, so a leading space turns
//   the prompt green with no ghost rendered. Found independently by two
//   reviewers. FIX: one shared accessor for "the prompt as parsed".

// crates/nova_os/src/terminal/edit.rs:293      (F73)
//   completion_matches iterates a std::collections::HashMap and appends
//   without dedup, so Tab-cycle order varies between processes.
//   FIX: collect + sort + dedup before returning.

// crates/nova_os/src/terminal/edit.rs:109      (F74)
//   History is unbounded and never deduped; only reset_session clears it.
//   200 submits of `log` means 200 Up presses to reach anything else.
+ const MAX_HISTORY: usize = 200;   // + skip a submit equal to the last entry
```

## The remaining three

```rust
// crates/nova_gameplay/src/mesh/explode.rs:130 and :144   (F16)
let Some(mesh) = meshes.get(&**mesh3d) else { error!(..); return; };
let Some(fragments) = explode_mesh(..) else { error!(..); return; };
//   A per-mesh failure `return`s instead of `continue`s, so one child with a
//   still-loading Mesh3d produces NO ExplodeFragments at all and discards the
//   fragments already built. integrity/explode.rs:129 skips anything
//   With<Mesh3d>, so the fragment handler is the wreck's only despawn path:
//   A ZERO-HEALTH WRECK LINGERS WITH ITS COLLIDER LIVE.
- return;
+ continue;
//   two lines, and the `entity` in the second error! should be mesh_entity.

// crates/nova_gameplay/src/audio/loops.rs:188,313          (F21)
//   Loop sinks are volume-driven only while the scenario is live
//   (SpaceshipSectionSystems gated on scenario_is_live) but the sink entities
//   are session-persistent and never silenced on unload. Menu ambience ->
//   New Game: the engine hum roars through the whole scenario load.
+ fn silence_loops_on_scenario_unload(...)   // NEW, on OnExit

// crates/nova_gameplay/src/sections/torpedo_section/projectile.rs:37  (F23)
//   update_target_position homes on the target root's raw
//   Transform::translation - the ship's BUILD-SPOT ORIGIN - rather than
//   live_structure_anchor. sections/mod.rs:38-43 states the rule and every
//   other consumer follows it (intent.rs:125, ai/acquisition.rs:170,
//   radar.rs:79, camera/framing.rs:52). Shoot away a large enemy's forward
//   half and the torpedo needs to reach within 15 u of empty space.
```

**Cluster with L11:** F23 shares `torpedo_section/projectile.rs` with F65 and
F66. Three findings, one 90-line file - read it once.

**Cluster with L5:** F41 (`nova_ui/src/status_bar.rs:196`, unconditional `Text`
and `TextColor` writes every frame) is behavior-only and belongs here, but
F46 and F51 are deletions in the **same 365-line untested file** and must wait
for the baseline. **This is the one place where the baseline line cuts through
a file rather than between files.** If one person runs both lanes, read the
file once and hold the F46/F51 commit until L5's window.

## Verified by

`keybind_dock.rs` is the shape to test against. For the per-frame-write
findings the assertion is a change-detection one: **run two frames with no
input and assert the component is not marked changed on the second.**
