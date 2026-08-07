# Code review - HUD, NOVA OS and input

Source: dedicated reviewer over `nova_gameplay/src/hud/**` (33.7k LOC) and
`src/input/**`, 2026-08-07. Findings below were spot-verified against the
tree and against `bevy_ui-0.19.0` source where the claim depended on engine
behavior.

**These are defects, not refactor items.** They exist today on `master` and
are independent of the restructuring epic. Several are cheap.

## Confirmed against bevy_ui source

The two highest-value findings both rest on how Bevy 0.19 handles scroll, so
both were checked in
`~/.cargo/registry/src/*/bevy_ui-0.19.0/src/layout/mod.rs`.

### A. The `f32::MAX` auto-scroll sentinel is never cleared

`crates/nova_gameplay/src/hud/nova_os/shell.rs:379` sets
`scroll.0.y = f32::MAX` to mean "pin to bottom".

Bevy computes `clamped_scroll_position` and writes it to
`node.bypass_change_detection().scroll_position` where `node: &mut
ComputedNode` (`layout/mod.rs:365-369`). **It only reads the `ScrollPosition`
component; it never writes the clamped value back.** So the sentinel persists.

Failure: after any command that appends output, `ScrollPosition.y` is
`f32::MAX`. The player presses PageUp; `input.rs:263` computes
`(f32::MAX + -page).clamp(0.0, max)`. In f32, `f32::MAX - page` is still
`f32::MAX`, which clamps to `max` - the bottom. Nothing moves. The second
press works.

Player-visible symptom: **"PageUp after running a command needs two presses."**

CONFIRMED. Severity: bug. Cheap fix.

### B. Scroll clamp mixes physical and logical pixels

`crates/nova_gameplay/src/hud/nova_os/input.rs:430` `max_nova_os_scroll_y`
builds its bound from `ComputedNode::content_size` / `size` / `scrollbar_size`,
which are **physical** pixels. `ScrollPosition` is **logical** - Bevy converts
with `scroll_pos.y * inverse_target_scale_factor.recip()`
(`layout/mod.rs:346-360`).

Failure on any display with scale factor != 1.0: on a 2x display the returned
maximum is twice the real one, so the stored offset runs to 2x the end and
scrolling back up needs twice as many wheel notches before anything moves.
Separately `page = computed_node.size.y * 0.8` (`input.rs:257`) is physical, so
one PageUp jumps 1.6 viewports.

The codebase already knows this rule - `position_nova_os_block_caret`
(`shell.rs:440`) multiplies by `inverse_scale_factor()`, and
`screen_indicator.rs:418` carries the comment "`ComputedNode::size` is
PHYSICAL". This site just missed it.

`nova_menu/src/widgets.rs:66` `max_menu_scroll_y` is identical and has the
same defect.

CONFIRMED (exact on a 1.0-scale display, wrong on any scaled one).
Severity: bug.

**Correction to `06-ui-layer.md`:** that note said three duplicated scroll
clamps. There are **two** `max_*_scroll_y` (`nova_os/input.rs:430`,
`nova_menu/widgets.rs:66`) and they agree with each other. The third site,
`nova_editor/src/ui/mod.rs:61 scroll_editor_panel`, is unclamped and so is a
different thing. The dedup argument stands; the count was wrong. And both
copies carry defect B, so deduplicating fixes it once instead of twice.

## Confirmed by direct read

### C. Ctrl+`<letter>` types a literal character into the prompt

`crates/nova_gameplay/src/hud/nova_os/input.rs:267` - the
`Key::Character(_) | Key::Space` branch has no Control guard.

Failure: at the `nova>` prompt press Ctrl+C. `close_nova_os_from_menu_keys`
calls `exit_app`, which returns `false` in `Prompt` mode, so nothing happens -
and then `handle_terminal_keyboard` receives `Key::Character("c")` and inserts
it. The prompt reads `c`. Same for Ctrl+U, Ctrl+W, Ctrl+A, Ctrl+K: **every
shell line-editing chord a player will reflexively try silently corrupts the
line.**

`handle_nova_os_app_keyboard` (`:355,:374`) deliberately skips events while
Control is held. The prompt handler chained immediately before it does not.

CONFIRMED by read. Severity: bug. This is the most likely of the set to be
hit by a real player.

### D. The scrollback is rebuilt on every keystroke

`crates/nova_gameplay/src/hud/nova_os/shell.rs:344` `rebuild_terminal_ui`
despawns and respawns **every** scrollback row whenever `NovaOsTerminal`
changes. The run condition is `resource_changed::<NovaOsTerminal>`
(`mod.rs:184`), and every edit goes through `ResMut`, so `DerefMut` marks it
changed on each keystroke - including plain caret movement, which changes
nothing on screen.

Nothing trims `NovaOsTerminal::scrollback`; `submit` only pushes.

Failure: with 400 rows of scrollback, typing a 12-character command despawns
and respawns 4,800 `Text` entities over 12 frames, each with a fresh `String`
from `spawn_terminal_row`'s `row.text.clone()`.

Severity: bug (performance). Note this compounds with finding E.

### E. `reconcile_nova_os_target` relayouts every frame, including while closed

`crates/nova_gameplay/src/hud/nova_os/crt.rs:219` writes `node.width` /
`node.height` unconditionally. `node.width = Val::Px(..)` is a `DerefMut` on
`Mut<Node>`, marking it changed regardless of value equality, so
`ui_layout_system` re-upserts the node into taffy and recomputes the subtree.

The only gate is `run_if(resource_exists::<NovaOsRtt>)` (`mod.rs:238`), and
`NovaOsRtt` lives from ship spawn to ship despawn - so this runs **while the
player is flying and the monitor is hidden**, over a subtree holding hundreds
of `Text` children.

`keybind_dock.rs:537` carries the guard and the comment for exactly this
("dirtying a `Node` costs a UI relayout even when the numbers are unchanged").
An `if node.width != desired` guard fixes it.

Severity: bug (performance). Same class at `shell.rs:442`
(`position_nova_os_block_caret` writes `node.left` unconditionally every frame
while open) and `nova_os_ship/scene.rs:750,772` (unconditional `TextColor` /
`BorderColor` / `BackgroundColor` writes, in a function that already guards its
`Text` write two lines above - inconsistent rather than deliberate).

### F. `last_len: Local<usize>` survives shell teardown

`crates/nova_gameplay/src/hud/nova_os/shell.rs:363`. Auto-scroll-to-bottom
stays dead after a respawn until the new session's scrollback grows past the
old session's length.

Failure: play until scrollback holds 200 rows, lose the ship.
`remove_nova_os` (`spawn.rs:710`) calls `terminal.reset_session()`, back to 6
welcome rows - but `last_len` is still 200. Respawn, open NOVA OS, type
`help`: new rows land below the fold and the view stays pinned at the top,
because `len > *last_len` is false. Broken for the next ~190 rows.

`reconcile_nova_os_header` (`:288`) and `rebuild_nova_os_footer_hints`
(`:320`) both carry an explicit `Added<Marker>` override for exactly this
hazard. This one does not.

Severity: bug. **This is the failure mode recorded in the
`mode-keyed-reconciler-just-spawned-override` memory, recurring at a site that
was not covered.**

## Lower severity

| Site | Issue |
| --- | --- |
| `hud/nova_os_ship/mod.rs:127`, `nova_os_map/mod.rs:132` | `NovaOsShipSystems` / `NovaOsMapSystems` are declared as `SystemSet`s but never passed to `configure_sets`, so they have no ordering edge to `NovaHudSystems` - which owns both the producer and the consumer of what they write. Whether a `ship repair` result row appears this frame or next is decided by Bevy's arbitrary topological order. The `peek_pending_invocation` dance at `nova_os_ship/app.rs:195` exists because of this; the hazard is in comments but never expressed to the scheduler |
| `hud/nova_os_ship/scene.rs:397` | `ship_input` reads raw `ButtonInput<KeyCode>`, bypassing the app router's Control guard, so `Ctrl+[` both exits the app and cycles selection backwards |
| `nova_os/src/terminal/view.rs:222` | `prompt_completion_ghost` strips the prefix off the untrimmed prompt while `refresh_parse` (`edit.rs:338`) uses the trimmed one, so a leading space turns the prompt green with no ghost rendered |
| `hud/nova_os/spawn.rs:20` | `setup_nova_os` has no "shell already exists" guard and overwrites `NovaOsRtt`. Speculative - could not confirm a double-add occurs. `input/player/flight_rig.rs:78` shows the `q_existing.is_empty()` pattern that is missing |

## Came back clean

Worth recording, because these were the suspected areas:

- **`nova_os/src/terminal/edit.rs` UTF-8 cursor arithmetic is correct
  throughout.** `insert_text`/`backspace`/`delete`/`move_cursor_*` all go
  through `char_indices()` and keep the cursor on a char boundary; the slicing
  `debug_assert!`s in `view.rs` hold. History indexing has no off-by-one.
  Tab-completion cycling is index-safe even if the candidate set shrinks
  mid-cycle. **The byte-vs-char panic hypothesis is dead.**
- `nova_os/src/shell.rs` - longest-word-prefix match, arity check and
  `words[name_words..]` slicing all in-bounds by construction; `levenshtein`
  correct.
- **No `unwrap`/`expect`/runtime-sized indexing anywhere in non-test code under
  `hud/` or `input/`.** Both gameplay-invocation handlers use `args.first()`
  with an error row, not `args[0]`.
- `hud/keybind_dock.rs` - the most careful reconciler in the tree. `set_if_neq`
  throughout, guarded `Node` writes, `Added<DockChip>` overrides, real
  `.after()` edges for the two systems whose comments claim an order. **Use it
  as the reference implementation when fixing D/E/F.**
- `input/player/flight_rig.rs`, `input/targeting/gesture.rs` - no unusable
  binding state, and pause gating is correct in both directions (press-side
  observers check `is_frozen()`, release-side deliberately ungated so a key
  held across a Tab transition still clears).
- `hud/nova_os/crt.rs` pointer forwarding - `nova_os_crt_screen_to_image_uv`
  correctly mirrors the shader's forward chain rather than inverting it, and
  returns `None` outside the picture.

## Bearing on the epic

The `hud/` seam is the largest single move planned. Findings D, E and F are all
**reconciler discipline** defects, and `keybind_dock.rs` shows the codebase
already has the correct pattern. Fixing them before or during the move is
cheap; fixing them after means re-reading 33.7k lines that have just shifted.

Finding F specifically recurs a failure mode already recorded in memory, at a
site the earlier fix did not cover - evidence that this is a systematic gap in
the HUD layer, not a one-off.
