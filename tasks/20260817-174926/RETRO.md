# Retro

## What worked

- Keeping arena rules in the example avoided a speculative match framework.
- Existing list rows and buttons were enough for the configurator. Only text
  entry needed a reusable widget.
- Xvfb plus direct pointer and keyboard input proved pause and return paths that
  a static UI test would miss.
- Stable section ids made arena-local rebinding retention small and explicit.

## Bugs and fixes

- Lobby action wrappers aligned to the row instead of the input frame. Match
  their top offset to the seed field.
- Wipe-only results left neutralized wreck roots standing forever. Count live
  flight computers and add boundary and inactivity breakers.
- The first structure snapshot could land before all section health existed.
  Require the complete roster snapshot to stay stable for a second frame.
- Result rows initially called neutralized wrecks survivors. Track operational
  state separately from remaining structure.
- Minimal apps using `NovaUiPlugin` did not initialize `KeyboardInput` messages.
  Make the text-field layer initialize the message it reads.
- The pinned nightly added stricter lints. Applied mechanical lint updates found
  by the CI-equivalent Clippy and default-feature checks.

## Next time

- Define operational defeat separately from entity lifetime when persistent
  wrecks exist.
- Capture starting metrics only after a stable setup edge, not the first root
  count match.
- Add interactive rendered paths as soon as an overlay exists; they exposed
  lifecycle and cursor behavior faster than component-only tests.
