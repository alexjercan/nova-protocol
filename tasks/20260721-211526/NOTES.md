# Comms Stack Implementation Notes

## What changed

- Replaced the single-line comms display with a bottom-left stack driven by
  `StoryFeed`: up to three visible cards, oldest at the top and newest at the
  bottom.
- Kept the full transcript in `StoryFeed`; visible dismiss and queue overflow
  only affect the HUD stack.
- Added `V` to dismiss the oldest visible card and `B` to promote queued
  backlog into the visible stack without waiting for dwell expiry.
- Added `icon: Option<AssetRef<Image>>` to `StoryMessageActionConfig` and
  `StoryLine`. Authored `Some("self://...")` and `Some("dep://...")` paths use
  the existing mod asset-ref validation and rewrite path. `None` or an omitted
  field renders the HUD fallback tile.
- Left generated base scenario messages on the fallback path by setting builder
  values to `icon: None`; serialized RON omits the field, so existing content
  stays backwards compatible.

## Why this approach

The optional `AssetRef<Image>` keeps story messages aligned with the rest of the
mod asset model instead of inventing a comms-only icon registry. It also avoids
forcing existing content to choose artwork before it can use stacked comms.

The stack owns only transient display state. The scenario sync remains the
source of truth for the transcript, which keeps dismiss/skip behavior visual and
preserves the future Tab drawer log contract.

## Bugs and diagnosis

- The first scenario test command tried to pass two Cargo test filters at once.
  Cargo accepts one positional test name, so the checks were rerun separately.
- The first mod-ref test draft parsed a `Scenario(...)` RON wrapper as a bare
  `ScenarioConfig`. Parsing it as `Content` matched the actual mod-content
  shape and let the test cover rewrite/validation through the production path.
- The stale-doc grep initially matched valid authoring prose such as "one line
  per beat." The DoD proof command was narrowed to stale UI claims about a
  single comms display instead.

## Verification

- `nix develop --command cargo test -p nova_gameplay hud::comms_panel`
- `nix develop --command cargo test -p nova_scenario story_message_icon_ron_round_trips`
- `nix develop --command cargo test -p nova_scenario story_sync_carries_the_authored_icon`
- `nix develop --command cargo test -p nova_assets story_message_icon_refs_rewrite_and_validate_like_other_assets`
- `nix develop --command cargo run -p nova_assets --bin content -- lint`
- `nix develop --command cargo check --all-targets`
- `nix develop --command cargo fmt --check`
- `rg -n "shows ONE|single line|queue and display" web/src/wiki/hud.md web/src/wiki/dev/scenario-system.md web/src/wiki/dev/guide-author-scenario.md crates/nova_gameplay/src/hud/comms_panel.rs crates/nova_scenario/src/actions.rs crates/nova_scenario/src/lint.rs`

## Self-reflection

The fail-first tests were useful because they forced the icon field to exist at
all the real boundaries: RON, sync, HUD rendering, and mod asset refs. Next time
I would write the production-shaped RON fixture directly as `Content` from the
start instead of temporarily treating it as the inner scenario type.
