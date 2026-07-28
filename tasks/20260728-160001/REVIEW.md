# Review: NOVA OS map view label table + map goto <label>

- TASK: 20260728-160001
- BRANCH: feat/map-view-labels

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Out-of-context reviewer ran `cargo check -p nova_os -p nova_gameplay` (clean),
`cargo fmt --check` (clean), the new `map_` tests (10 passed incl. the three
named), the DoD filters `map_view` (2) and `map_goto` (2), the full `ship_` set
(57 passed - proving the peek-then-take refactor did not change ship behavior),
and `nova_os --lib` (18). It confirmed the diff mirrors the ship-view label
pattern, that the two apply systems both hold `ResMut<NovaOsTerminal>` (so Bevy
serializes them) and peek-then-take only their own verb (neither swallows the
other's), that `merge_arg_completions` keeps each app's Tab set intact, and that
the three new tests would fail if the fix were removed. Verdict APPROVE with two
NITs.

In-session supplement: independently confirmed `set_arg_completions` had become
production-dead (only its own unit test used it; both sync systems use
`merge_arg_completions`) - so R1.2 was adopted, not just noted. Re-ran
`nova_os --lib` (18 passed) after converting that test to the production
`merge_arg_completions` path and removing the dead API; fmt still clean.

- [x] R1.1 (NIT) crates/nova_gameplay/src/hud/nova_os_map.rs:222 - `info_cell`
  right-pads range to width 3 (`{:>3.0}`), so a 2-digit range reads `" 60 u"`
  where the TASK example showed `60 u`.
  - Response: Intentional - the right-alignment lines the range numbers up
    across rows in the free-form INFO column (the header is not aligned to it
    anyway), which reads better than ragged-left. Left as-is; it is a NIT.
- [x] R1.2 (MINOR) crates/nova_os/src/terminal.rs - `set_arg_completions` is now
  reached only from its own unit test; production moved to
  `merge_arg_completions`. Remove the dead API or document the production path.
  - Response: Fixed. Removed `set_arg_completions`, converted
    `nova_os_arg_completion_expands_injected_codes` to drive
    `merge_arg_completions` (the production seam), and repointed the two doc
    references. `nova_os --lib` 18 passed, fmt clean.

Pending user checks (manual DoD, cleared at flow Finish):
- Open the `map` app and confirm each contact blip reads its code (`SELF`,
  `HOST-1`, `AST-1`, ...).
