# Asymmetric burn push traces

Headless traces from a disposable unit test (not kept) that stepped
`BurnPush::step` at the shipped 64 Hz fixed tick with `FlightSettings`
spool rates, Normal mode rig (20.6 u). Columns: tick, time, commanded,
`engaged`, `extension` and the camera target push in u. Every scenario has 32
quiet ticks first, so the command starts on tick 33.

- `before-*`: PR #84 at 7fed262a7, 0.5 s engage and release debounce.
- `after-*`: engage on the first commanded tick, hold through quiet, release
  after 0.5 s of quiet.

| scenario | before: first move, max push | after: first move, max push, release |
|---|---|---|
| `sustained`: 96 lit ticks | tick 64, 3.09 u | tick 33, 3.09 u, tick 160 |
| `tap1`: 1 lit tick | never, 0 u | tick 33, 0.28 u, tick 65 |
| `tap8`: 8 lit ticks | never, 0 u | tick 33, 1.63 u, tick 72 |
| `brake29on1off`: 29 lit, 1 cold, x8 | never, 0 u | tick 33, 3.09 u, tick 303 |
| `trim3on61off`: 3 lit, 61 cold, x4 | never, 0 u | tick 33, 0.76 u, per pulse |

After, the extension never falls while `engaged`. The trims are spaced past
the debounce, so each one leans 0.76 u and eases home.

## Rendered comparison

A disposable example (not kept) loaded the player skiff fixture alone with its
bodies frozen, at a fixed 1/60 s virtual step under lavapipe at 960x540. It
pressed the real `main_drive` key per scenario and captured every third frame.
Two builds: 7fed262a7 `step` and the new `step`. Frame 0 is the first
`Playing` frame. `render-skiff-*.csv` gives the rendered hull width in px (198
at rest, 174 at full push). The `commanded` column is not filled in.

- `start_stop`: held frames 30 to 150.
- `pulse`: 12 frames on, 8 off, from frame 30 to 150. An input approximation
  of autopilot brake pulses, not an autopilot flight.
- `tap`: frames 30 and 31.

| | before | after |
|---|---|---|
| `start_stop` first move | frame 63 | frame 33 |
| `start_stop` release starts | frame 183 | frame 183 |
| `pulse` | never moves | 174 px by frame 72, flat through every gap, home from frame 174 |
| `tap` | never moves | 194 px by frame 39, held, home by frame 78 |

`cmp-skiff-*.jpg` stack the before row over the after row. At 160x90 tiles
the 4 to 24 px change does not read by eye; the CSVs carry the measurement.
The carrier did not reach `Playing` within 3 min under lavapipe and was not
captured.
