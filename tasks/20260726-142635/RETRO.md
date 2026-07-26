# Retrospective

## What Went Well

- Comparing the two screenshots first made the issue concrete: the problem was not hierarchy, it was readability, contrast and overlaid CRT intensity.
- Reusing a single welcome-row helper for default terminal state and initial UI spawn removed a fragile duplicated boot-state path.

## What Went Wrong

- The previous flow closed with manual visual comparison still pending, and that allowed a structurally-correct but visually weak screen to land.
- The shader and fallback effects were both present, but their combined visual strength was not evaluated in the game screenshot.

## Improve Next Time

- For UI fidelity tasks, capture or inspect the in-game render before landing, even when the HTML reference and widget-tree tests match structurally.
- Keep render effects conservative by default, then increase intensity from a screenshot, rather than stacking high-alpha fallback and shader layers.
