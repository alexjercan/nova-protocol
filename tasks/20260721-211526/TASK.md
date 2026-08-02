# Comms messages: notification-style stacking, skip control, speaker icons, dismiss

- PRIORITY: 55
- TAGS: v0.9.0, feature, hud, ui
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Goal

Owner direction (playtest, 2026-07-21): make comms messages richer -
STACKING (multiple visible as notification-style popups, new messages
under/on top per the questionnaire), a SKIP control, a per-speaker ICON,
and explicit dismiss (keypress) alongside the timeout; placement stays
left, top-vs-bottom per the questionnaire; the FULL conversation log lives
in the Tab drawer (that part rides the Tab family).

Today's panel (task 20260717-163033) shows ONE line, queued with dwell -
this task grows it into a stack. Release slot per the questionnaire
(v0.9.0 default under the no-new-features rule). /plan breaks it into
steps at pickup.

## Notes

- Depends on: 20260721-211512 (the Tab drawer spike) for the log view; the stack itself is
  independent.
- Owner decisions (questionnaire, 2026-07-21): BOTTOM-LEFT, CHAT-STYLE
  stack - newest line at the bottom, older lines push up and fade (a
  conversation transcript, not an alert feed). Dismiss: keypress AND
  timeout. RELEASE: v0.9.0 with the Tab family (stays backlog until v0.9.0
  planning); the full log view lives in the Tab drawer.
- Plan-gate owner correction (2026-07-25): `StoryMessage` should add
  `icon: Option<AssetRef<Image>>`, authored as `icon: Some("self://...")` or
  `icon: Some("dep://...")`; `None` or an omitted field uses the fallback icon.

## Steps

- [x] Replace the one-line `CommsDisplay` state machine in
  `crates/nova_gameplay/src/hud/comms_panel.rs` with a visible stack model over
  `StoryFeed`: newest visible card at the bottom, older cards push upward, and
  pending overflow still drops oldest while the full log remains in
  `StoryFeed`.
- [x] Extend the story-message data path with an optional image asset ref:
  add `icon: Option<AssetRef<Image>>` to
  `nova_scenario::StoryMessageActionConfig`, mirror it into
  `nova_gameplay::StoryLine`, and update the event-world sync in
  `crates/nova_scenario/src/world.rs`.
- [x] Render each visible comms entry as its own notification-style card with a
  resolved `ImageNode` icon when `icon` is `Some`, a stable fallback when it is
  `None`, a speaker label, wrapped text, fade timing, and bottom-left layout
  that does not fight the flight-status row or objective hint.
- [x] Thread the new icon field through authored/generated base scenario content
  and mod asset-ref rewriting/validation, using existing `self://` and
  `dep://` semantics and preserving back-compat for older `StoryMessage` RON.
- [x] Add comms controls in `comms_panel.rs`: a keypress that dismisses the
  oldest visible card immediately and a skip key that fast-forwards queued
  backlog into the visible stack without waiting for the dwell timeout.
- [x] Keep scenario teardown strict: an emptied `StoryFeed` clears visible
  cards, pending cards, tweens, and control state in the same update.
- [x] Add or update HUD tests that prove arrival-order stacking, timeout
  expiry, explicit dismiss, skip behavior, queue cap, teardown reset, and
  optional icon rendering/fallback.
- [x] Add or update scenario serde and mod-ref tests that prove omitted icons
  default to `None`, `icon: Some("self://...")` parses/round-trips, and
  resource-ref rewriting/validation sees StoryMessage icon paths.
- [x] Update player-facing and author-facing docs for the new comms behavior:
  `web/src/wiki/hud.md`, `web/src/wiki/dev/scenario-system.md`, and
  `web/src/wiki/dev/guide-author-scenario.md`; add a terse v0.9.0
  `CHANGELOG.md` entry if the current Unreleased section expects feature notes.
- [x] Record implementation notes in `tasks/20260721-211526/NOTES.md`,
  including what changed, why the icon approach was chosen, bugs diagnosed, and
  self-reflection for future sessions.

## Definition of Done

- A burst of at least three `StoryLine`s shows multiple simultaneous
  bottom-left cards in arrival order, with the newest at the bottom and older
  cards pushed upward/fading rather than overwritten (test:
  `a_burst_stacks_visible_lines_newest_at_bottom`).
- A visible comms card leaves by timeout, and the dismiss key removes a visible
  card immediately without deleting the transcript from `StoryFeed` (test:
  `dismiss_hides_a_visible_line_without_touching_the_log`).
- The skip control promotes queued backlog into the visible stack without
  waiting for the normal dwell floor, while still respecting the visible-stack
  cap (test: `skip_promotes_pending_lines_into_the_stack`).
- `StoryMessage` RON remains backwards-compatible when `icon` is omitted and
  round-trips `icon: Some("self://...")` / `icon: Some("dep://...")` as an
  `AssetRef<Image>` path (test: `story_message_icon_ron_round_trips`).
- `self://` and `dep://` StoryMessage icon refs participate in the existing mod
  resource-ref rewrite/validation path (test:
  `story_message_icon_refs_rewrite_and_validate_like_other_assets`).
- Authored icon refs render as image nodes, while `None` or omitted icons render
  a stable fallback instead of blank UI (test:
  `speaker_icons_use_authored_refs_and_fallback`).
- Scenario teardown clears visible and pending comms state immediately (test:
  `emptied_feed_resets_the_comms_stack_immediately`).
- The changed Rust code builds and focused tests pass (cmd:
  `nix develop --command cargo test -p nova_gameplay hud::comms_panel`).
- Scenario schema tests pass with the new optional icon field (cmd:
  `nix develop --command cargo test -p nova_scenario story_message_icon_ron_round_trips`).
- Content validation accepts icon asset refs and still warns on invalid
  resources through the existing mod-ref gate (cmd:
  `nix develop --command cargo run -p nova_assets --bin content -- lint`).
- Rust formatting is clean (cmd: `nix develop --command cargo fmt --check`).
- Player and author docs no longer describe comms as a one-line queue (cmd:
  `rg -n "shows ONE|single line|queue and display" web/src/wiki/hud.md web/src/wiki/dev/scenario-system.md web/src/wiki/dev/guide-author-scenario.md crates/nova_gameplay/src/hud/comms_panel.rs crates/nova_scenario/src/actions.rs crates/nova_scenario/src/lint.rs`).
