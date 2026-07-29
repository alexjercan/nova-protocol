# Retro: Bug: sandbox nova_probe mod cache from installed local mods

- TASK: 20260729-015406
- BRANCH: fix/probe-profile-sandbox
- REVIEW ROUNDS: 2 (R1: 1 MAJOR, 2 MINOR, 1 NIT -> R2 APPROVE)

## What went well

- The reproduction was cross-process from the start (the test binary
  re-executing itself as a child), so the rig tested what the BUG is about -
  what a spawned process resolves - instead of what the parent's env looks
  like. Both integration tests carry their unsandboxed leg permanently, so
  neither can rot into a test that passes either way.
- Running a real `probe run playable` produced the decisive artifact nothing
  else would have: the run WROTE `enabled_mods.ron` into
  `probe-runs/<sha>/playable/profile/config/nova-protocol/` instead of the
  operator's `~/.config`. Positive evidence the redirection is live, not just
  an absence of reads.
- The isolated-lever leg (NOVA_MOD_CACHE_ROOT alone moves the index while the
  config stores still read the poison) meant the two levers were proven
  separately rather than by one all-or-nothing env blob.
- The out-of-context reviewer earned its keep twice: it found the real gap
  (R1.1), and on re-review it ran a MUTATION check (deleting the sandbox from
  one builder and confirming the test goes red) rather than accepting the fix
  on trust.

## What went wrong

- R1.1 (the samply pass shipped unsandboxed): I swept by the artifact I was
  EDITING - the two named env-builder functions, `clean_pass_env` and
  `trace_pass_env` - instead of by the mechanism, the `run_supervised` call
  sites. There were four; the samply pass built its env inline at the call
  site, so it was invisible to a "which builders exist?" sweep. Root cause: I
  read enough of probe.rs to find the builders and stopped, then wrote docs
  ("every native child run", "no pass ships unsandboxed") whose universal
  quantifier I had never actually enumerated. The overclaim was in the diff
  before the gap was.
- R1.2 (the wiring test could have gone vacuous): I derived the test's
  expectations from the host environment (subtract the inherited vars), which
  is right for the policy but means an all-three-exported host leaves the
  assertion loop with nothing to iterate - a silent green. I chose the
  host-derived form specifically to avoid a spurious failure and did not ask
  the next question: what does this test assert when the subtraction empties it?

## What to improve next time

- When a change claims "every X", enumerate X mechanically before writing the
  claim: grep the SPAWN/CALL sites (`run_supervised`, `Command::new`), not the
  helper functions, and make the test iterate that enumeration so a new site
  cannot join silently. The docs sentence is the tell - if prose says "every",
  the sweep that justified it must have been a grep, not a reading.
- Any test whose expectation set is COMPUTED (from env, config, a filtered
  list) needs a non-empty guard, or it degrades to a pass that asserts nothing
  under a condition nobody re-checks.

## Action items

- [x] Ledger: bumped `pin-each-caller-not-just-shared-core` to x4 (already in
      Pending promotions -> work skill) with the spawn-site-not-builder
      sharpening; added `computed-expectations-need-a-nonempty-guard` (x1).
- No follow-up code tasks: the reviewer's own sweep confirmed no other repo
  path reads operator profile state (`nova_debug::screenshot`'s
  `dirs::download_dir()` is an output path under the `debug` feature, not read
  state), and the web pass is documented out of scope.
