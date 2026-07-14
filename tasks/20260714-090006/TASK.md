# UI/feedback SFX pass: menu clicks, pause/settings toggles, dry-fire cue, radar retarget tick

- STATUS: OPEN
- PRIORITY: 12
- TAGS: v0.6.0,polish,audio,ui

Goal: add the missing feedback cues the audio audit found. All currently silent:

- Main-menu button press (New Game / Sandbox / Settings / Exit) -
  `crates/nova_menu/src/lib.rs:~540-557` (visual hover/press only).
- Pause overlay toggle - `nova_menu/src/lib.rs:~272-290`.
- Settings panel expand/collapse - `nova_menu/src/lib.rs:~507-511`.
- Turret dry-fire when the magazine is empty -
  `crates/nova_gameplay/src/sections/turret_section.rs:~961-965` (silently blocked;
  add a click/buzz on the empty transition).
- Radar re-designation tick - `crates/nova_gameplay/src/input/targeting.rs:~2592`
  (retargets are silent; only the initial lock-on plays; add a subtle lower-volume
  tick).

A grouped SFX polish pass; each cue is a one-liner through the existing audio
system. Sprint tail, low priority.
