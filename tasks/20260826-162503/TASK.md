# Modal input: one owner for the keyboard, not a guard list per system

- STATUS: CLOSED
- PRIORITY: 74
- TAGS: v0.12.0,input,editor,ui

Follows the review-fix task. Raised by round 1 of `/nova-review`, where a
one-line guard fix was judged the wrong shape for the problem.

## The problem

No arbiter decides which consumer owns a keystroke. Each keyboard system carries
its own hand-written guard list, so anything new suppresses nothing by default.
`typing_into_a_field` guards one specific modal consumer; the keybind rebind is
a second one and was added without a predicate, so pressing Delete to bind
Delete also runs `delete_key` and destroys the marked section
(`keybind.rs:381`, `placement.rs:276`, `lib.rs:175`). The review-fix task adds a
stop-gap predicate for exactly that pair. This task removes the need for it.

## The shape

Modal input, along the lines of vim's normal / insert / visual. A mode owns the
keyboard while it is active and every consumer asks the mode rather than
carrying a denial list. Bind mode swallows everything but its own exit, so the
camera stops taking WASD without knowing bind mode exists; placement and
shortcuts go quiet the same way.

Consumers that already are modes in all but name: text entry
(`typing_into_a_field`), the gallery (`gallery_open`), the keybind rebind, and
plausibly Nova OS.

## Done when

- The stop-gap guard from the review-fix task is deleted, and the collision it
  covered is unreachable by construction rather than by predicate.
- A new `examples/systems` range with its `catalog_drift` roster.
- Live run: `cargo check` cannot see an input mode fail.

## Landed

`nova_ui::input_mode` is the arbiter. Two rules cover every keyboard system:

- A VERB runs in `InputMode::Normal` alone (`in_input_mode(InputMode::Normal)`).
- A MODE'S OWN system runs with `owns_or_enters(mode)` - its mode and Normal,
  because the key that OPENS a mode is pressed before the mode exists.

`InputMode`'s declaration order IS the exclusivity order (Normal < Browse <
Insert < Bind), so who wins a contested frame is one enum away rather than a
rule per pair. Modes are CLAIMED, not set: whoever holds the state that defines
a mode writes a `ClaimKeyboard` each frame it holds, `InputModeSystems`
(PreUpdate) keeps the greatest claim, and a mode ends by going quiet. The
claimant lives with the state it reads - the gallery and the rebind are not
names `nova_ui` could know.

Deleted, not replaced:

- `keybind::rebind_armed`, the stop-gap. Delete under Bind is now unreachable
  by construction: `delete_key` answers in Normal, and Bind is not Normal.
- `ui::inspector::typing_into_a_field` and its nine call sites.
- `escape_backs_out`'s own `rebind.target.is_some()` check and its two
  `.before()` edges. A mode holding the keyboard is a mode Escape never reaches.

### Decisions

- The gallery keeps `gallery_open` for the systems that DRAW. "Is the build view
  visible" is a different question from "who owns the keyboard", and gating the
  draw chain on a mode would stop it repainting while a field is focused.
- NOVA OS was left alone. Its keyboard is gated on `PauseStates::NovaOs`, an app
  state, not a hand-written denial list - it is already one owner at a time, and
  converting it would be churn with no defect behind it.
- Ctrl+S and Del now go quiet while the parts gallery is up. They did not
  before. Browse swallowing everything but its own exit is the point of the
  shape, and neither key is one a builder reaches for over a full-screen browser.
- `in_input_mode` and `owns_or_enters` answer TRUE with no `InputMode` resource
  in the world: an app with no arbiter has no modes. A headless test of one verb
  does not have to stand a mode machine up.

### Proof

New range `examples/systems/system_input_modes` presses one key per mode - the
key that, without the arbiter, two owners would both answer - and reads the
NEGATIVE after a bounded settle, because there is no event for a key that went
nowhere. Four invariants on the `catalog_drift` roster (`SYSTEMS_INVARIANTS`
174 -> 178):

- `insert mode keeps delete off the tree`
- `the keyboard comes back to normal`
- `browse mode keeps escape off the back-out`
- `bind mode keeps delete off the tree`

Live runs, all exit 0, on a sandboxed profile (`XDG_CONFIG_HOME` /
`XDG_DATA_HOME` / `NOVA_MODDING_CACHE_ROOT`):

- `system_input_modes` 2.6 s - all four verdicts printed.
- `system_ship_editor` 10.8 s, `bug_sandbox_soak` 46.8 s, `system_nova_os`
  2.6 s, `system_menu_boot` 2.4 s - the walks that share these keys.

Mutation-checked LIVE: dropping the Normal gate from `delete_key` fails
`system_input_modes` at the Insert beat (exit 101). Unit tests
`del_does_not_delete_under_bind_mode` and
`the_editor_claims_the_keyboard_for_its_two_modes` were mutation-checked the
same way.

Unit tests: 6 new in `nova_ui::input_mode` (resolve, decay to Normal, the field
claim, the verb rule, the owner rule, the no-arbiter fallback), 1 new in
`nova_editor` (the claimant table), 1 rewritten in `placement`. nova_editor 310,
nova_ui 44. `cargo check --workspace --all-targets --features debug`,
`cargo fmt --check` and `catalog_drift` all exit 0. Workspace Clippy skipped
locally per the standing instruction; CI runs it.
