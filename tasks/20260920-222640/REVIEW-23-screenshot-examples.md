# Review 23: Screenshot, lesson, and loop examples

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: major and minor findings

## Findings

### MAJOR - `examples/screenshots/screenshot_damage_levels.rs:453-457` - Wide shot does not wait for asynchronous capture completion

The `shoot the row` step requests `damage-levels.png` and has no `until` guard. Autopilot steps default to immediate completion, while capture writes on a later frame. The next step immediately reframes the camera, so the establishing shot can use the next camera pose or be lost under load.

The same file documents this exact asynchronous race and waits after its column shots. Every other screenshot call in this batch waits for `shot_written` or an explicit delay. The defect was established by source inspection, not reproduced by running the capture.

Why not BLOCKER: one documentation capture is at risk; shipped game behavior is unaffected.

### MINOR - `examples/screenshots/lesson_novaos_contacts.rs:118-135` - Contact capture does not prove the intended selection

The script assumes two next actions select the hostile contact, then captures after a frame settle. It never asserts the current map readout or selected contact. A spawn-order or cycle regression can capture the wrong contact while the producer still completes.

### MINOR - `examples/screenshots/loop_goto_standoff.rs:52-60,413-444` - A claimed park-distance proof only logs values

The file calls its park log proof that both ships stop near `ARRIVAL_MARGIN`, but `report_the_park_points` has no assertion or marker bounding the measured gap. A standoff regression remains pilot prose unless a person notices the log or loop.

### MINOR - Screenshot fixtures bypass exported section ID constants

`examples/screenshots/screenshot_damage_levels.rs:548,551,568,572` and `screenshot_section_trials.rs:125,226` hardcode section IDs for which `nova_ship` exports constants. Current values match. Future prototype renames would not fail at compile time and can silently break these producers.

## Adjudication

- The idle-orbit duplication was already counted in review 19.
- Dropped the round-type damage-value difference. The example claims production travel rules, not production balance values, and exaggerated damage can be a valid visual fixture.
- Dropped the self-acknowledged Hollow blast backstop. Its comment gives a concrete inspection/removal rule.

## Coverage

The pair fully read all 79 top-level `lesson_*`, `loop_*`, and `screenshot_*` files and every included shared/nested module, including common fixtures. Capture calls, completion predicates, input transitions, and runtime IDs were audited per file.

Together with review 19, every playable and screenshot example now has a full static pass.

Not checked: no Cargo command, game, capture, probe, GPU run, workspace test, or Clippy run. Visual composition and asynchronous failures were not observed in rendered output.
