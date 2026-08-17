# Retro

- The torque field encoded two concepts: hardware authority and hull-size compensation. Naming the replacement in acceleration units removed that ambiguity.
- Mechanical value conversion was wrong for outcome-only test rigs and torpedo steering. Preserve test intent, not numeric ratios, when a unit changes.
- The first range revision compared unequal initial errors after reload. A slower moving command and a response window derived from measured convergence made the 10x-inertia comparison fair.
- Run the focused affected range before a full category probe. It finds task-local failures faster and avoids spending minutes on unrelated ranges.
- The old gain ratio already encoded an exact small-signal lag: `kd / kp = 0.5 s`. Exposing that behavior removed two misleading control-theory fields without changing shipped handling.
- Keep response quality and authority independent in mixed stacks. The smallest lag supplies the response while acceleration keeps its own diminishing rank.
