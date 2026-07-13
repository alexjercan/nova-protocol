# Retro: Flight feel retune (torque-budget turn rates, camera weight)

- TASK: 20260709-095043
- BRANCH: feature/flight-feel-retune (squash-merged as 83c5b73)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES with 3 MAJOR + 5 MINOR + 4 NIT;
  round 2 APPROVE)

What shipped is in the task's Resolution and
`tasks/20260709-095043/NOTES.md`. First REQUEST_CHANGES round of these
cycles - and every MAJOR was earned.

## What went well

- **The codebase had already written the design brief.** The
  `est_turn_rate_deg` doc comment said "a knob rather than a derivation from
  PD gains + inertia (recorded for the retune)" - reading the knob's own
  documentation produced the core design (`hull_turn_rate`) in minutes.
  Past-session breadcrumbs in doc comments are cheap and they compound.
- **Outcome-invariant physics tests survived a physics-model change
  untouched.** All 23 pre-existing flight tests passed under a completely
  different turn-rate regime because they assert "arrives at rest at the
  standoff", not "takes N seconds". Third cycle in a row this style has paid;
  it is now simply how physics tests are written here.
- **The reviewer's quantitative skepticism caught what qualitative review
  would not.** All three MAJORs came from the reviewer recomputing my claims
  (respawn path for smoothing, per-tick impulse ratios for off-center
  engines, the actual flagship's inertia). The off-axis test then confirmed
  the impulse math exactly on the first run.

## What went wrong

- **I tuned against the test rig and called it the game (R1.3).** max_torque
  10 was justified by "the stock 3-section ship" - which exists only in
  flight.rs tests. The shipped flagship has 4.7x that inertia; its turn rate
  silently halved. Root cause: I derived the numbers from the ship I had
  been staring at in tests instead of grepping the scenario configs for what
  players actually fly. Tuning rationale must name shipped content.
- **The headline feature only worked for the first life (R1.1).** Smoothing
  was applied on mode change; death/respawn re-inserts a default ChaseCamera
  and no mode change follows. Root cause: I never traced the component's
  lifecycle across the death path - the same death path this repo has
  wrestled with before (camera controller removal is right there in
  loader.rs). New rule of thumb: for any "set once" write to a component,
  ask what re-creates that component and when.
- **I quoted optima as delivered behavior (R1.4).** The flip times ignored
  the 0.9 scale and the PD's ramp lag - numbers with decimals that were
  really vibes. Same lesson as the COM cycle's perception claim, second
  occurrence: numbers in docs need their derivation and their caveats or
  they will not survive review.
- **A multi-file edit script died mid-run again** (changelog anchor matched
  twice; the commit ran anyway because the commands were separate lines, not
  chained) - leaving a commit that claimed bookkeeping it did not contain,
  patched up one commit later. Third occurrence of this failure family
  (multi-thruster retro, COM cycle). The fix is mechanical: per-edit
  apply-and-report loops AND chaining the commit to the script's exit status.

## What to improve next time

- Before choosing any tuning constant, list the shipped entities it binds
  (grep the scenario/asset configs, not the test rigs) and put a number to
  each in the rationale.
- For every only-on-change write, trace the component's full lifecycle -
  especially across death/respawn/scene transitions.
- Chain bookkeeping commits to the edit script's exit (`&&` on one line, or
  commit in the script) so a dead script cannot be followed by a live commit.
- Keep inviting the reviewer to recompute claims against sources; it is the
  highest-yield review instruction in these cycles.

## Action items

- [x] tatr 20260709-155920: thrust balancing for off-center engines.
- [x] tatr 20260709-155921: AI rotation path adopts slew + hull_turn_rate.
- [x] tatr 20260709-155922: disabled-in-place controller still torques.
- [ ] Playtest with the user (handoff below is in the cycle report): flip a
  full vs stripped ship, camera lean under burn, deadband twitch; retune the
  documented constants from their feedback.
