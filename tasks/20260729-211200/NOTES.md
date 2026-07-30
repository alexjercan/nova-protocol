# NOTES - the objective chip IS the posting

Record for the change that retired the diegetic objective reveal card. The
build-shape fork (delete the module vs keep it dormant) is DECISION.md; this
file is what the diff actually did and what it cost.

## What changed

- **Deleted** `crates/nova_gameplay/src/hud/objective_reveal.rs` (418 lines:
  `ObjectiveRevealPlugin`, `ObjectiveRevealMarker`, `ObjectiveRevealTucked`,
  `ObjectiveRevealAnimation`, `spawn_objective_reveal`, `REVEAL_*` constants and
  the module's two tests), its `pub mod` + plugin registration in `hud/mod.rs`,
  and its spawn call in `objective_feedback.rs`.
- **Ungated the chip** in `hud/objective_stack.rs`. The notification carried TWO
  clocks - `age_secs` from the posting (driving the `REVEAL_TOTAL_SECS`
  handover fallback) and `Option<f32> pop_secs` from the card's tuck (driving
  the pop and the read dwell, and gating whether the chip rendered at all).
  Now one: `age_secs` seeds the pop AND runs the dwell, and every notification
  in `shown` renders. Gone with it: `hand_over` / `handed_over`,
  `pop_chip_on_reveal_tuck`, and the `.after(ObjectiveRevealAnimation)` set
  ordering on the plugin's chain.
- **Removed `NovaOsTabAnchor`** (`hud/nova_os.rs`) and the stack's
  `update_stack_anchor` that published it, plus `STACK_ANCHOR_SIZE`. Grepped
  first: the card was its only consumer, so both producer and consumer left in
  the same diff and nothing dangles.
- **Simplified `objective_feedback`**: `added_objectives` existed only to feed
  the reveal spawn loop and an `added` bool; it collapses to the bool. The
  module now owns exactly the posting BLIP and the green completion ghost - a
  posting's visual is the stack's chip.
- Docs: wiki `hud.md`, CHANGELOG `[Unreleased]` (two lines rewritten, the
  "Diegetic objective reveal" line DELETED - the card never shipped in a
  release, so there is nothing for a reader to be told about), and a dated
  invalidation note on backlog task 20260729-182853, whose whole premise was
  capturing the card-to-chip handover.

## Tests

Fail-first: `a_posting_shows_the_objective_text_not_a_count` was rewritten to
assert the chip on the POSTING frame and run against the unmodified tree -
red with `left: [] right: ["SALVAGE THE WRECK"]`, i.e. the gate, not a typo.
Then the implementation turned it green.

The card's disappearance took four tests with it. Per
`deleting-a-test-salvage-live-assertions`, each was read for what it still
pinned before deleting:

- `a_late_tuck_does_not_hand_over_the_next_objective` and
  `a_stray_tuck_from_an_orphaned_card_hands_over_nothing` pinned per-ID tuck
  matching (dead) - but between them they also pinned that two live postings
  do not share one clock. Salvaged as `each_posting_runs_its_own_dwell`, which
  posts two objectives 75% of a dwell apart and asserts the older reads out
  while the younger is still unread.
- `a_posting_with_no_card_hands_over_on_the_fallback` pinned that a RE-WORDED
  objective (which got no card) still reached the screen. Salvaged as
  `a_re_worded_objective_shows_its_chip_on_the_same_frame`, which asserts the
  new WORDS on the chip, not just its presence.
- `the_stack_publishes_the_reveal_tuck_anchor` pinned the anchor only; it went
  with the resource, and the rig's hand-placed `GlobalTransform` (there only so
  the anchor had a translation to read) went with it.
- `objective_feedback`'s `ObjectiveRevealMarker` assertion in
  `a_message_swap_of_the_same_id_leaves_no_ghost` pinned "a same-id swap adds
  no second card". The live half - a swap makes no ghost - is asserted two
  lines above it; the chip half belongs to `objective_stack` and is covered by
  `a_read_chip_returns_only_when_its_objective_changes`. Dropped, not re-homed.

`the_chip_pops_when_the_card_lands_and_settles_back` was renamed
`the_chip_pops_on_the_posting_and_settles_back` and re-aimed at the posting
frame; it keeps the rendered-SCALE assertion that is the whole reason it exists
(review R1.1 of 20260729-163816, where a pop written onto the rebuilt entity
never played).

## Verification

- `cargo check --workspace --all-targets`: clean, no warnings (the point of
  deleting rather than dormanting).
- `cargo test -p nova_gameplay --lib -- objective_stack:: objective_feedback::`:
  14 passed, 0 failed - 10 stack + 4 feedback, both modules non-empty.
- `grep -rn "objective_reveal\|NovaOsTabAnchor\|ObjectiveRevealTucked" crates/
  examples/ tests/`: no hits (DoD 3).
- `cargo run -p nova_probe -- run playable`: see REVIEW/TASK for the verdict.
- `cargo fmt --all --check`: clean.

The full local test suite and clippy were NOT run (repo convention - CI runs
both); the tests above are the ones this task wrote or touched.

## Reflection

Two things went right and are worth repeating. First, the DECISION.md written
at the plan gate had already done the hard reasoning - that route 2 (dormant
module) could not stand on its own, because with no card there is no tuck and
every posting would sit out the fallback - so the build had no fork left to
discover. Second, deleting a module is where `deleting-a-test-salvage-live-
assertions` earns its keep: four tests died with the card, and two of them were
carrying an assertion about the STACK that nothing else pinned. Reading each
assertion (not each test) is what turned a net -4 tests into -4/+2 with no
coverage hole.

What to watch next time: `OBJECTIVE_DWELL_SECS` (12 s) was chosen when the
dwell started at the card's tuck, i.e. the player had ~3.2 s of card plus 12 s
of chip. It now starts at the posting, so the objective is on screen for 3.2 s
LESS than it used to be. That is a deliberate no-change here (12 s of a chip
you can read from frame one is not obviously short, and the owner's playtest is
the tuning gate), but it is a real behaviour delta the diff does not shout
about, and the manual DoD item is where it gets judged.
