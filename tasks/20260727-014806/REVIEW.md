# Review - NOVA OS topbar FPS; hide flight status bar in drawer

- DATE: 20260727
- REVIEWER: (none - no independent out-of-context review round)

## Note on process

This feature was built by a delegated sub-agent /flow run in parallel with the
render-to-texture CRT task (20260726-193233) and landed on the owner's explicit
instruction ("land the FPS thing") WITHOUT a separate out-of-context review
round. Recording that honestly rather than implying a review that did not happen.

What WAS verified before landing:
- The sub-agent's own widget-tree tests: `topbar_status_line_carries_a_live_fps_segment`,
  `drive_topbar_fps_writes_the_smoothed_reading_onto_the_status_line`,
  `flight_status_bar_hides_while_the_drawer_is_open_and_returns_on_close`.
- Merge integration with the RTT branch (20260726-193233): the only conflict was
  a test-module adjacency (both branches inserted tests before the CRT-material
  test that RTT renamed); resolved by keeping both FPS tests and RTT's renamed
  test. Post-merge `cargo test -p nova_gameplay drawer` = 60 passed; `cargo fmt`
  clean; on-master re-verify after landing = 60 passed.
- Semantic compatibility with RTT confirmed by reasoning: the topbar (now
  rendered through the RTT image) has its `FPS: N` text updated each frame, so
  it shows on the CRT screen; the flight status bar hides via dropping
  `HudDrawerExempt`.

## Pending manual acceptance (owner)

- Native/visual eyeball that the topbar reads `SHIP: <name>  LINK: LOCAL  FPS: N`
  through the CRT and the flight status bar is gone while the drawer is open.

- VERDICT: APPROVE (owner-accepted; landed without an independent review round)
