# Retro: Spike - NOVA OS CRT monitor look and feel

- TASK: 20260726-193040
- BRANCH: (none - research spike)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The two failed text-glow attempts in the parent task (`20260726-180807`) paid
  off here: they turned "why can't we just glow the text?" into two concrete,
  verified constraints (UI materials can't sample the content; the blit camera
  has no bloom), which made the RTT recommendation land as the ONLY real path
  rather than one option among many.
- Splitting the work by cost/independence (cheap shader + casing wins now, the
  heavy RTT pipeline as a separate headline) means the monitor keeps improving
  even if the big task is never scheduled.

## What went wrong

- Nothing material. Minor: `tatr new` assigns time-based IDs, so the SPIKE.md
  "Next steps" had to be written with placeholder IDs and reconciled after the
  tasks existed - a small ordering wrinkle inherent to the tool.

## What to improve next time

- When seeding tasks from a spike, create the tasks FIRST, then write the
  SPIKE.md "Next steps" with the real IDs, to avoid the placeholder/reconcile
  step.

## Action items

- Seeded tasks (see SPIKE.md): 20260726-193155, 20260726-193219, 20260726-193233.
  No promotion-worthy recurring lesson from this spike.
