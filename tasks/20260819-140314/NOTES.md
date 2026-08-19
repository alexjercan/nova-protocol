# NOTES - what the model measured once it was code

All figures read out of the built tree on 2026-08-19, on branch `attitude-model`.
`LOAD_LIMIT` = 8 G = 78.48 m/s2, `METERS_PER_UNIT` = 10, every controller at
`max_torque: 1501`. "flip" is the bang-bang 180, `2 * sqrt(pi / alpha)`, the
same figure the explainer quotes.

## The shipped fleet - the page was right to 0.2 %

Read through `hull_mass_properties` + `structural_arm` over the authored
`ship_catalog()` boxes, which is avian's own arithmetic on the same colliders
the flown ship spawns.

| ship | colliders | mass | I (largest) | arm | structural | torque | binds | flip |
|---|---|---|---|---|---|---|---|---|
| racer | 7 | 8.28 | 10.87 | 2.178 u | 3.604 | 138.05 | structure (38x) | 1.87 s |
| cargob | 9 | 18.95 | 48.85 | 2.933 u | 2.676 | 30.73 | structure (11x) | 2.17 s |
| cargoa | 9 | 15.86 | 35.79 | 2.761 u | 2.842 | 41.94 | structure (15x) | 2.10 s |

The explainer predicted `I` = 10.86 / 48.79 / 35.73 and arms of 2.18 / 2.93 /
2.76 u, for flips of 1.87 / 2.17 / 2.10 s. The code agrees on every digit it
published. The 1-1-1 reference hull lands on 5.232 rad/s2 and 1.55 s, also as
published.

## WFC-generated hulls - NEW, and they sit at the knee

Eight hulls from `wfc_ships --ships 8` (seeds 20260815..20260822), measured live
off their spawned rigid bodies.

| seed | sections | mass | I | arm | structural | torque | ceiling | flip | binds |
|---|---|---|---|---|---|---|---|---|---|
| 20260815 | 168 | 174.85 | 1977 | 6.876 u | 1.141 | 3.037 | 1.141 | 3.32 s | structure |
| 20260816 | 248 | 250.90 | 2566 | 6.861 u | 1.144 | 2.340 | 1.144 | 3.31 s | structure |
| 20260817 | 224 | 240.51 | 2342 | 6.237 u | 1.258 | 3.846 | 1.258 | 3.16 s | structure |
| 20260818 | 176 | 191.57 | 2200 | 7.228 u | 1.086 | 2.730 | 1.086 | 3.40 s | structure |
| 20260819 | 236 | 249.57 | 2652 | 6.630 u | 1.184 | 2.264 | 1.184 | 3.26 s | structure |
| 20260820 | 188 | 200.71 | 2174 | 6.305 u | 1.245 | 1.381 | 1.245 | 3.18 s | structure |
| 20260821 | 240 | 252.84 | 2795 | 6.835 u | 1.148 | 3.223 | 1.148 | 3.31 s | structure |
| 20260822 | 184 | 194.57 | 1987 | 6.893 u | 1.139 | 3.022 | 1.139 | 3.32 s | structure |

Findings for the later judgement the owner asked for:

- Every WFC hull is structure-bound, so the 4v4 flies on `LOAD_LIMIT` alone and
  `max_torque` is invisible there too.
- They flip in 3.2 s to 3.4 s against a shipped corvette's 2.10 s. Uniform:
  the arm only moves 6.24 u to 7.23 u across eight seeds, because the collapse
  fills a fixed grid.
- **The headroom is 1.1x to 3.5x, not the fleet's 11x to 115x.** WFC hulls are
  the first content anywhere near the crossover. Seed 20260820 is 11 % from
  becoming torque-bound.
- The spread in headroom is the CONTROLLER COUNT, which the collapse does not
  control: 3.037 / 1977 x 1501 says seed 20260815 mounts two computers,
  20260817 mounts six. A seed that rolls one computer on a 240-section hull
  would go torque-bound and read as a barge, and nothing in `wfc_hull` stops it.

## Where the game disagreed with the page

- **The torpedo is not small.** The record expected a torpedo's tiny arm to give
  it about 31 rad/s2, roughly 3x today's authored 10.0. Its two sections are
  1 u cubes at z = 0 and z = 1, so its centre of mass sits at 0.5 and its arm is
  1.0 u = 10 m: the structural ceiling is 78.48 / 10 = **7.85 rad/s2**, a 21 %
  REDUCTION on today's 10.0. It is implemented with no exemption, at
  `max_torque: 50` (nine times the 6.5 its structure can spend), so it is
  structure-bound like every hull. If a torpedo should be sharper the answer is
  a smaller torpedo, not a bigger number.
- **A flown flip is not a bang-bang flip.** The guidance layer slews the command
  at `hull_turn_rate` = `0.9 * sqrt(pi * alpha) / 2`, which is the bang-bang
  AVERAGE trimmed 10 %, so the hull never runs the bang-bang profile. Measured
  on the 3-section rig, a 170 degree turn takes 2.77 s to get inside 5 degrees
  and 3.55 s to park, against the ideal 1.55 s. The ratio is a property of the
  guidance trim, not of this model, and it applies equally to the 5.00 s figure
  the change is quoted against.
- **The precision curve is now an unpaid cost.** `STACK_PRECISION_LIMIT` stays,
  as decided, but the authority curve used to pay for it. On a structure-bound
  hull a stack now buys nothing and still divides the P gain, so a 3-section
  fighter goes 94.9 -> 90.7 deg/s peak and 2.77 -> 3.48 s traverse from one
  computer to ten, with overshoot already at 0.00 deg in both. Every hull that
  ships is structure-bound, so this is a small, real, discoverable penalty for
  stacking on exactly the craft a player flies. Worth a look before release; it
  was not in scope to remove here.
- **`min` over principal axes needed a choice.** The envelope uses the LARGEST
  principal moment, which is what `hull_turn_rate` already budgets against and
  the conservative axis. Unobservable in shipped content, which is
  structure-bound on every axis.

## Flown table (`cargo test --lib -p nova_ship stacking -- --nocapture`)

170 degree flip, one shipped computer unless noted. `traverse` is seconds to
within 5 degrees.

| hull | ctrl | onset | peak deg/s | traverse | overshoot | settle |
|---|---|---|---|---|---|---|
| fighter (3 x d1) | 1 | 0.10 | 94.9 | 2.77 | 0.00 | 3.55 |
| fighter | 10 | 0.12 | 90.7 | 3.48 | 0.00 | 4.63 |
| cruiser (15 x d1) | 1 | 0.18 | 42.9 | 4.50 | 0.00 | 5.28 |
| cruiser | 10 | 0.18 | 42.9 | 5.13 | 0.00 | 6.27 |
| barge (15 x d20) | 1 | 0.37 | 32.3 | 7.52 | 6.70 | 10.02 |
| barge | 2 | 0.25 | 33.3 | 5.77 | 0.00 | 6.63 |
| barge | 4 | 0.18 | 42.9 | 5.03 | 0.00 | 6.12 |
| barge | 10 | 0.18 | 42.9 | 5.13 | 0.00 | 6.27 |

The barge at four computers lands on the cruiser's numbers exactly: same
length, same arm, same structural ceiling. Buying enough computers buys a big
ship its physics back and nothing beyond it.
