# SW-EXAMPLES: example/manual updates for the World Name Field

Owner-approved option (a). Two edits per example (sandbox env var + typed
name before Create), plus the `nova_bench` pilot manual. No cargo run from
this worker; the main worker owns builds/checks.

## 1. Sandbox env var (`NOVA_CONFIG_ROOT`)

Inserted the exact block at the top of `main` (before `editor_app`/any other
env setup) in every file except `bug_failed_assets.rs`:

- `examples/screenshots/lesson_menu_mouse.rs:411-426` (before the existing
  `MENU_BACKDROP_ENV` set_var)
- `examples/screenshots/lesson_generated_world.rs:119-134`
- `examples/screenshots/lesson_menu_advanced.rs:73-88`
- `examples/screenshots/lesson_menu_quality.rs:59-74`
- `examples/screenshots/loop_world_start.rs:131-146`
- `examples/screenshots/screenshot_menu.rs:96-111` (before the existing
  backdrop-pin set_var)
- `examples/systems/system_session_loop.rs:56-71`
- `examples/systems/system_command_shell.rs:138-153` (the `#[cfg(feature =
  "debug")] fn main`, before `editor_app`)
- `examples/systems/system_open_world_identity.rs:54-69`
- `examples/systems/system_open_world.rs:53-68`
- `examples/systems/system_menu_boot.rs:57-72`

`examples/systems/bug_failed_assets.rs`: left untouched as instructed. It
already points `CONFIG_ROOT_ENV` at its own fresh per-run root inside
`stage_broken_mods` (`examples/systems/bug_failed_assets.rs:189-196`).

## 2. Type "Probe World" into World Name Field before Create

Mechanism per file, matching whatever that file (or its nearest sibling)
already used for the seed field:

- **Direct `TextFieldValue` query/insert** (the mechanism
  `lesson_generated_world.rs`, `system_open_world_identity.rs` and
  `system_open_world.rs` use for the seed field): added a
  `.step("... name the world")` / `.on_enter` closure that queries
  `(&Name, &mut TextFieldValue)`, finds `World Name Field`, and sets
  `value.0 = "Probe World"`, placed immediately before the existing
  seed-entry step (or before the Create click where there is no seed step):
  - `examples/screenshots/lesson_generated_world.rs:532-542` (new, before
    the existing "enter the seed" step at 543)
  - `examples/systems/bug_failed_assets.rs:403-413` (no seed field in this
    file; borrowed the mechanism, nearest file in the list that establishes
    it is `system_open_world_identity.rs`)
  - `examples/systems/system_session_loop.rs:372-382` (no seed field;
    same borrowed mechanism)
  - `examples/systems/system_open_world_identity.rs:663-673` (before the
    existing "enter the seed" step)
  - `examples/systems/system_open_world.rs:353-363` (file only *reads* the
    seed field via `field_text`; used the write form of the same
    `TextFieldValue` mechanism to set the name)
  - `examples/systems/system_menu_boot.rs:203-213` (no seed field; same
    borrowed mechanism)

- **Autopilot click + type gesture** (the mechanism `loop_world_start.rs`
  and `screenshot_menu.rs` already use for the seed field: move to the
  field's right edge, press, release, then `type_text`/caret-clear):
  - `examples/screenshots/loop_world_start.rs:609-651`: added
    `move_to_name_field_edge` (mirrors `move_to_seed_field_edge`) and four
    new steps (move, click, release, type) before the existing seed steps.
    No backspace-clear needed; the field opens empty.
  - `examples/screenshots/screenshot_menu.rs:253-261`: added a `.click`
    on `World Name Field` + a `type_text("Probe World")` step, waited on a
    new `name_field_reads` predicate (`screenshot_menu.rs:294-308`, mirrors
    `seed_field_reads`), before the existing `Create World Button` click.

- **No existing field mechanism in the file or forward in the list**
  (`lesson_menu_mouse.rs`, `lesson_menu_advanced.rs`,
  `lesson_menu_quality.rs` never touch the seed field): borrowed the direct
  `TextFieldValue` query/insert mechanism from `lesson_generated_world.rs`
  (nearest sibling in the `examples/screenshots` list that has it), added a
  `.step("name the world")` before the existing `.click("create the
  world", "Create World Button")`:
  - `examples/screenshots/lesson_menu_mouse.rs:555-566`
  - `examples/screenshots/lesson_menu_advanced.rs:215-226`
  - `examples/screenshots/lesson_menu_quality.rs:157-168`

`examples/systems/system_command_shell.rs`: no edit under item 2. This file
never clicks `Create World Button` - it only waits for/asserts the node's
presence (`ui_node_present(CREATE_WORLD_BUTTON)`, lines 537 and 928) to prove
the modal opened and that the menu takes clicks again after the computer
closes. Nothing here presses Create, so there is nothing to type a name
before.

Consts added where the file already had a `SEED_FIELD`/`CREATE_WORLD_BUTTON`
block (`WORLD_NAME_FIELD`, `WORLD_NAME = "Probe World"`, next to them):
`lesson_generated_world.rs`, `loop_world_start.rs`, `screenshot_menu.rs`,
`bug_failed_assets.rs`, `system_session_loop.rs`,
`system_open_world_identity.rs`, `system_open_world.rs`,
`system_menu_boot.rs`. The three files with no such const block
(`lesson_menu_mouse.rs`, `lesson_menu_advanced.rs`, `lesson_menu_quality.rs`)
use the literal strings `"World Name Field"` / `"Probe World"` inline,
matching their existing style of literal `"Create World Button"` strings.

Imports: added `#[cfg(feature = "debug")] use nova_ui::widget::TextFieldValue;`
(or merged into the existing `nova_ui::widget::{...}` import) wherever a file
didn't already import it: `lesson_menu_mouse.rs`, `lesson_menu_advanced.rs`,
`lesson_menu_quality.rs`, `bug_failed_assets.rs`, `system_session_loop.rs`,
`system_menu_boot.rs`.

No doc/comment in any of the 12 files described the exact Create click
sequence in prose, so none needed updating beyond the code itself.

## 3. `crates/nova_bench/src/manual.rs` and `manual.md`

- `manual.rs:81-92` (`opening`, the `PlayTarget::NewGame` arm): the pilot's
  first-message instructions now tell it to fill `World Name Field` with
  `{"text":"Probe World"}` (same rect/target method as the seed field, no
  clearing needed since the field is empty) before the existing seed
  instructions and the `Create World Button` click.
- `manual.rs:127` (`the_manual_documents_every_tool_and_the_opening_names_the_goal`):
  added `assert!(new_game.contains("World Name Field"));` next to the
  existing `World Seed Field` assertion. No new test added.
- `manual.md:134-139`: the `{"text": "<characters>"}` bullet now also
  describes `World Name Field` (click it like the seed field, send
  `{"text": "Probe World"}`, no clearing) ahead of the existing seed-field
  instruction.

## Could not do / left alone

- `bug_failed_assets.rs`: sandbox block intentionally skipped per the
  explicit exception.
- `system_command_shell.rs`: name-typing step intentionally skipped; the
  file never presses `Create World Button`.
