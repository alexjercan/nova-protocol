# Gantry and its crew

Open [index.html](index.html) directly. This private board compares:

- Gantry's intact and stranded conditions in matched forward/aft views.
- Nadia, Owen, and Ivo in comic color.
- The exact same crew geometry with lore's color-only presentation.

The broad plated cargo body and raised handling frame distinguish Gantry from
Kaveri's open cradle and Ebro's exposed vessels. A single aft bell, forward crew
module, and side transfer collar form a reusable external proposal, not a
construction specification. No physical scale or capacity is selected.

The stranded condition derives from the same model. Only the aft service cover
and roof change; the crew module, cargo body, frame, and transfer collar remain.
Main-drive thrust is not allowed in this condition. There is no explosion,
attack-direction claim, exact electrical diagram, injury, or hull-fate decision.
The comparison uses no thrust in either condition so the shapes can be compared.

The three new heads face forward. Each has authored contour, hair, features,
and separate civilian clothing. Only the original expression is registered.
These are appearance proposals, not new ages, biographies, rank, or final
likenesses. The crew's starting portraits show no later injuries.

Public lore gets the intact ship sheet and the three starting portraits only.
Public exports come from `scripts/gen-lore-designs.py` and the shared library;
they never import this task. The episode script and accepted opening are not
changed by this art batch.

## Sources and checks

```bash
python3 scripts/gen-lore-designs.py --check
python3 tasks/20260908-161328/gantry-study/generate.py --check
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/nova_illustration -p 'test_*.py'
node tasks/20260908-161328/proof/inspect-gantry.mjs web/dist
```

`generate.py` owns the comparison layout, review labels, and offline board, not
the ship or character drawings. It writes three SVGs and `index.html` here.
Browser evidence goes to `../proof/gantry/`; the harness stops its owned browser
and server. Do not rerun earlier proof writers or overwrite previous evidence.

The user accepted these forms and likenesses as working designs. The next
[Aquila pages](../aquila-pages/index.html) reuse them. [Batch review](../GANTRY-REVIEW.md)
records this earlier scope and evidence; do not rerun its proof writer to refresh
unchanged-script claims after the Aquila revision.
