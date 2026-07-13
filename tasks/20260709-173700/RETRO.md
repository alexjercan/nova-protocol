# Retro: Three-tier turret auto-fire feed

- TASK: 20260709-173700
- BRANCH: feature/turret-lock-feed (squash-merged onto master)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- **The discriminator had to be world-space, and the plan caught that at
  assert-writing time.** Dead-ahead geometry projects the camera-ray point
  and the target to the same pixel, so a screen-space pip assert cannot see
  which tier feeds the turret; asserting |aim_point - target_anchor| < 5 m
  proves it regardless of framing. Choosing probes that discriminate the
  hypotheses, not just confirm the happy path, is becoming the house test
  style.
- **A widened query surfaced its own migration.** Adding
  TurretSectionTargetVelocity to the turret query made the 150711-era test
  fail loudly on missing resources instead of silently not matching - the
  fail-loud direction, though by luck (resource validation fires before
  query matching).
- **Concurrent master movement absorbed again** (health overkill fix landed
  mid-task); merge, re-verify, land - routine by now.

## What went wrong

- **A wedged Xvfb burned 240 s and briefly looked like a code deadlock.**
  The display server was restarted inside a dying compound command; the app
  then froze mid-run with no panic (present blocks, frame loop stops, the
  self-exit timer never fires). This is the third display-server lesson in
  the family: the retro rule said "own command" and the restart still got
  chained behind a pgrep in a subshell. New, sharper rule: never (re)start
  the display server in the same Bash invocation as anything else, and
  treat a no-panic freeze in a scripted range as a display problem FIRST.

## What to improve next time

- Display server lifecycle: dedicated invocation, verify with pgrep in a
  separate command, only then run the app.
- When a scripted range hangs without a panic, check the display server
  before reading the diff.

## Action items

- None new; AI-side velocity feed already noted on 20260709-155921.
