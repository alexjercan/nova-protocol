# Retro: Mods main-menu section (list + enable/disable, base locked)

- TASK: 20260714-174126
- BRANCH: menu/mods-panel
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The two Explore agents' up-front maps (menu structure + persistence) meant the UI
  was pure pattern-matching: the ModsPanel is the SettingsPanel modal + the editor's
  data-driven list + `button()`/`observe()`. No new UI machinery invented.
- The catalog metadata seam (`ModCatalog` in nova_assets, re-exporting `ModEntry`)
  kept nova_menu decoupled from the asset machinery - the menu reads two plain
  resources (`ModCatalog`, `EnabledMods`) and mutates one. Clean crate boundary.
- The goal-critical chain (toggle -> `EnabledMods` -> live re-merge) was already
  proven by 174120's `toggling_enabled_mods_remerges_live`, so this task only had to
  prove the menu half (the panel lists demo + the toggle flips the set). Splitting the
  loading (174120) from the UI (174126) made each independently testable.
- Independent review confirmed the whole chain end to end, including the bevy_ui_widgets
  detail that `Activate.entity` is the button entity `on_mod_toggle` reads.

## What went wrong

- Adding `scroll_mods_panel` with a `MessageReader<MouseWheel>` broke FOUR existing
  menu tests: the minimal headless test `app()` has no `InputPlugin`, so
  `Messages<MouseWheel>` is absent and the `MessageReader` system param panics on run.
  Root cause: a new system pulled an input-message dependency that the menu's test
  harness never needed before. Fixed with a `resource_exists::<Messages<MouseWheel>>`
  run condition (present in the real app via `DefaultPlugins`, absent in tests).

## What to improve next time

- A system with a `MessageReader<T>`/event dependency added to a plugin that has
  minimal-app tests needs a `resource_exists::<Messages<T>>` (or equivalent) run
  condition, or the tests that merely enter the state will panic. Check the plugin's
  own test harness for what plugins it omits before adding an event-reading system.

## Action items

- [x] Panel lists demo with a toggle (goal), base locked; toggle flips `EnabledMods`.
- [ ] R1.1 (deferred, pre-existing): Settings + Mods modals can overlap - a small
      mutually-exclusive-modal follow-up if the menu grows more panels.
- [ ] 174131 (persistence) makes the enabled set survive restarts - the last task.
- [x] Lesson: `messagereader-needs-resource-guard-in-tests`.
