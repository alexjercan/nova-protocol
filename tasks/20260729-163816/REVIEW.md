# Review: Objective read-notification stack

- TASK: 20260729-163816
- BRANCH: feat/objective-notification-stack

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/objective_stack.rs:483 -
  `pop_chip_on_reveal_tuck` is effectively dead: it writes `HudEmphasis::pop()`
  onto chip entities that `sync_objective_chips` despawns and respawns the next
  frame, seeded from `popped_at_age(..., age_secs)`. By tuck time `age_secs` is
  ~3.2 s, so the rebuilt component has `one_shot_secs = 0` and the pop is gone;
  the reviewer proved it with a probe (one frame at 1.16, then 1.0, and at
  60 Hz that single frame only eases to ~1.013 - invisible). The chip does pop,
  but at POSTING, while the reveal card covers it. So the behaviour asserted by
  the module doc, the TASK step, DECISION.md, the wiki and the CHANGELOG does
  not happen. Move the pop into the notification STATE and seed the emphasis
  from it.
  - Response: fixed, and the fix went further than the finding. The
    notification now carries a second clock, `pop_secs`, `None` until the
    handover; `pop_chip_on_reveal_tuck` writes that STATE (not a doomed
    component), `sync_objective_chips` renders only handed-over chips and seeds
    `popped_at_age` from `pop_secs`, and the DWELL runs from the handover too
    (it used to spend its first ~3.2 s invisible behind the card). Two knock-on
    fixes the finding surfaced: the system order had to change (handover BEFORE
    render, or every chip appears a frame late - the plugin and the rig now
    both read post -> handover -> age -> read -> render), and a fallback
    handover was needed because a RE-WORDED objective is not an "addition", so
    `objective_feedback` spawns it no card and no tuck would ever arrive -
    without it such a chip would wait forever. Pinned by
    `the_chip_pops_when_the_card_lands_and_settles_back` and
    `a_posting_with_no_card_hands_over_on_the_fallback`.
- [x] R1.2 (MAJOR) crates/nova_gameplay/src/hud/objective_stack.rs:582 - the
  test module had no coverage of the pop or the breath at all: the rig never
  registered `drive_hud_emphasis` and no test touched `UiTransform`/`TextColor`.
  DoD 1's "pops then settles to the breath" clause was unproven, and the two
  deleted hint tests that pinned exactly this still-live behaviour
  (`the_hint_pops_when_the_reveal_tucks_in_and_settles_back`,
  `the_hint_breathes_only_while_objectives_are_outstanding`) were removed with
  the module without replacement. That gap is what let R1.1 through.
  - Response: fixed. The rig registers `drive_hud_emphasis` in PostUpdate,
    where the real plugin puts it - NOT in the Update chain, because the chips
    come from `sync_objective_chips`'s deferred commands and an in-chain driver
    never sees this frame's chips (every scale reads the default 1.0; this cost
    a debugging round and is now a comment in the rig). Both deleted tests are
    re-homed onto the stack as
    `the_chip_pops_when_the_card_lands_and_settles_back` and
    `a_settled_chip_breathes_until_it_is_read` (which also pins that a READ
    chip fades monotonically instead of breathing).
- [x] R1.3 (MINOR) CHANGELOG.md:41 - the `[Unreleased]` section still described
  the retired surface as current in three places (a "minimalist top-right
  objective hint (glyph + count + a Tab affordance)", a reveal that "tucks
  up-and-right into the objective hint", and "the objective hint's TAB
  affordance"), contradicting the new bullet. These are unreleased, so they
  describe the upcoming release, not history.
  - Response: fixed - all three now name the stack. The only remaining mention
    of the hint is the new bullet saying what it replaced.
- [x] R1.4 (MINOR) crates/nova_gameplay/src/hud/mod.rs:135 - the `HudTier::Status`
  doc edit was botched: "(the fps/version status bar and the bar)".
  - Response: fixed - "(the fps/version status bar)".
- [x] R1.5 (MINOR) crates/nova_core/src/lib.rs:305 - `setup_status_ui` still
  said "the fps/version bar (and the objective count in it)".
  - Response: fixed - the parenthetical is gone.
- [x] R1.6 (MINOR) crates/nova_gameplay/src/hud/mod.rs:202 and
  crates/nova_assets/src/lib.rs:1244 - `nova_crt_mark`'s rustdoc still named the
  "objective-hint TAB affordance" as a consumer; that consumer is gone.
  - Response: fixed in both; the mark is documented as the NOVA OS drawer plate
    logo only. (The first pass at nova_assets produced "the drawer plate and
    the drawer plate" - caught on re-read and cleaned up.)
- [x] R1.7 (NIT) crates/nova_gameplay/src/hud/mod.rs:1245 - stale prose on
  `childless_node_is_left_to_inherit_the_status_bar` ("The objective count is a
  CHILD of the status bar root...").
  - Response: fixed - "A childless status-bar item is a CHILD of ...".
- [x] R1.8 (NIT) crates/nova_gameplay/src/hud/objective_stack.rs:498 - the new
  `#[expect(clippy::type_complexity)]` is unfulfilled (a new clippy warning on
  this branch), and `breathe_objective_chips`'s query does not need `mut`.
  - Response: fixed - attribute dropped, query is read-only.
- [x] R1.9 (MINOR) crates/nova_gameplay/src/hud/objective_stack.rs:374 - the
  chip spawned the instant `GameObjectives` changed, so the objective's
  sentence was on screen TWICE for the reveal card's full 3.2 s. New behaviour
  (the retired hint showed only a count). Suggested gating the chip's
  visibility on the tuck if R1.1 is fixed that way.
  - Response: fixed by the R1.1 handover - a chip is not rendered at all until
    the card hands over, so the text is never on screen twice. Pinned by the
    first assertion of `a_posting_shows_the_objective_text_not_a_count`.

Verification the reviewer ran (not findings): `cargo check --workspace
--all-targets --features dev` clean; `cargo test -p nova_gameplay --lib --
hud::` 281 passed; `-p nova_menu --lib` 73 passed; an independent
`cargo run -p nova_probe -- run playable` OK (5/6, fps SKIPPED - no baseline).
Falsifiability mutations: gutting `read_on_nova_os` failed the NOVA-OS test
alone; deleting the completed-objective retain failed the completion test
alone; a temporary probe demonstrated R1.1. All four claimed deviations (no
range suffix, completion drops the chip, 96 px placement, the diamond as a
node) were checked against the code and hold, as does the `chip_paint`
duplicate-component reasoning. DoD 3's grep is trivially satisfiable (the
identifier `StatusBarRootMarker` does not exist anywhere in `crates/` - the bcs
API is `status_bar()`), so the reviewer re-verified the real claim
independently: nothing in the HUD parents an objective block into the bar.

Pending user check: DoD 5, the owner playtest.

## Round 2

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

All nine round-1 findings confirmed resolved (the reviewer re-proved R1.1 and
R1.2 by mutation rather than reading the diff). Four new findings, one of them
a real defect the round-1 fix introduced.

- [x] R2.1 (MAJOR) crates/nova_gameplay/src/hud/objective_stack.rs:533 -
  `pop_chip_on_reveal_tuck` matched a tuck to a notification POSITIONALLY
  ("the oldest still waiting"), but `ObjectiveRevealTucked` carries no
  identity and producer and consumer were both unordered members of
  `NovaHudSystems`. The reviewer measured both legal schedule orders with a
  probe driving the real card animation: in the chain-first order, with two
  objectives posted 1 s apart, the FALLBACK handed `first` over and `first`'s
  now-spare tuck then handed `second` over a full second before its own card
  landed - putting that sentence on the chip and the in-flight card at once,
  i.e. R1.9 back again, decided by a build-to-build tie-break. Two further
  paths mis-hand-over regardless of order, because a card outlives its
  notification: a completion before handover, and a player-ship respawn. Give
  the message an identity and match on it.
  - Response: fixed. `ObjectiveRevealTucked(pub String)` now carries the
    objective id (stored on `ObjectiveRevealMarker`, which already formatted it
    into its `Name`), and the handover matches on that id, so a stray tuck
    matches nothing. The stack's chain is also explicitly
    `.after(objective_reveal::animate_objective_reveals)`, so the tuck is
    consumed the frame it is written rather than by tie-break. Two regression
    tests: `a_late_tuck_does_not_hand_over_the_next_objective` reproduces the
    reviewer's measured race (fallback hands over first, first's card lands
    late, second must stay behind its own card) and
    `a_stray_tuck_from_an_orphaned_card_hands_over_nothing` covers the
    outlived-card paths. BOTH fail under the old positional match - checked,
    because the first attempt at the former did NOT (its tucks arrived in
    posting order, so positional matching coincidentally agreed; it was
    rewritten as the fallback race).
- [x] R2.2 (MINOR) crates/nova_gameplay/src/hud/objective_stack.rs:985 - the
  "until it is read" half of `a_settled_chip_breathes_until_it_is_read` did not
  pin its claim: removing the `read_secs.is_some()` guard from
  `breathe_objective_chips` left all 10 tests passing, because the post-read
  loop sampled 2 frames and the 2.4 s wave happened to be descending there.
  - Response: fixed - the read half now asserts the chip renders the FADE's own
    alpha (`ObjectiveNotification::alpha() * rest`) rather than sampling for
    non-monotonicity over a window too short to be conclusive. Removing the
    guard now fails the test - checked.
- [x] R2.3 (MINOR) crates/nova_gameplay/src/hud/objective_stack.rs:361 -
  `read_on_nova_os` marks every entry read INCLUDING ones still behind their
  card, so an objective posted within ~3.2 s of the player pressing Tab is
  discarded unshown. Defensible, but undocumented.
  - Response: documented rather than changed - it follows the owner's model
    ("after some time or after open the TAB thing also goes away"): the player
    has just read that objective in the computer's own list. The behaviour and
    its rationale are now in `read_on_nova_os`'s doc comment.
- [x] R2.4 (NIT) crates/nova_assets/src/lib.rs:1244 and
  crates/nova_core/src/lib.rs:305 - correct prose left raggedly wrapped after
  the round-1 edits.
  - Response: fixed, both reflowed.

The reviewer also disputed the "cannot be seen end to end" claim usefully: the
lifeline walk DOES hold `screen_convoy` past 3.2 s later in its script, and
more to the point the screenshot examples already have a timed `at(<secs>, ...)`
script, so a few lines posting an `Objective` at 1.0 s and capturing at 4.5 s
would show the handover deterministically. Filed as **20260729-182853**
(backlog) rather than widening this branch.

## Round 3

- VERDICT: APPROVE
- REVIEWER: out-of-context

All four round-2 findings verified resolved. The reviewer re-proved R2.1 two
ways: reverting to positional matching fails both new regression tests, and
re-running its round-2 integration probe (real cards, the unfavourable
chain-first order, two postings 1.0 s apart) now reports `second` appearing at
frame 64 - exactly when its own card lands - where round 2 measured frame 45, a
second early. It also confirmed the R2.2 assertion is not tautological (it
reads the rendered `TextColor` and compares against a value computed
independently from `ObjectiveNotifications`) and that reverting
`ObjectiveRevealTextMarker` to private is a hard compile error, i.e. R3.1's
exposure was forced by the approach rather than careless.

Three NITs, all taken:

- [x] R3.1 (NIT) crates/nova_gameplay/src/hud/objective_reveal.rs:167 -
  ordering by naming the system function forced `animate_objective_reveals` and
  (transitively) `ObjectiveRevealTextMarker` to `pub(super)`. A `SystemSet`
  exposes one zero-sized marker instead, and is the pattern the module already
  uses (`NovaHudSystems`, `HudSituationSensing`).
  - Response: fixed - `pub(super) struct ObjectiveRevealAnimation`; the system
    and the marker are private again.
- [x] R3.2 (NIT) crates/nova_gameplay/src/hud/objective_stack.rs:436 - the fade
  wrote an ABSOLUTE alpha while the breath wrote one RELATIVE to the tone's own
  alpha. They agree only because the amber tone is opaque today; give it a
  sub-1.0 alpha and a read chip would render brighter than an unread one.
  - Response: fixed - one `chip_alpha(factor)` helper, used by both paths, that
    is always a fraction of the tone's rest alpha. Re-checked that the R2.2
    mutation (dropping the read guard) still fails the test under the new
    convention, since a convention change could have made it pass again.
- [x] R3.3 (NIT) crates/nova_gameplay/src/hud/objective_stack.rs:369 - the
  doc comment cited DECISION.md for the never-shown case, but point 4 speaks to
  a chip that is UP, not to a posting that never showed.
  - Response: fixed - the comment now says plainly that this EXTENDS the
    owner's model rather than reading it off, and leans on the argument that
    actually carries the case (the player just read it in the computer's list).

The reviewer notes one honest gap it could not close: `fps_within_baseline` is
SKIPPED in every probe run for want of a baseline, so whether the new `.after()`
ordering constraint costs any parallelism is unmeasured either way.

Pending user check: DoD 5, the owner playtest.
