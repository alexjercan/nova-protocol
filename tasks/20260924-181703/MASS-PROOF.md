# Docked helm mass-ratio proof (temporary fixtures, removed)

Four disposable fixtures. All are removed from the tree. Apply one patch to
restore its fixture. For the first three, run:
`nix develop --command cargo test -p nova_ship --lib temporary_docked_mass_ratio_table -- --nocapture`

- `mass-proof-fixture.patch`: first run, unmodified controller only (table 1).
- `mass-proof-compensation.patch`: second run, unmodified vs compensated vs
  rigid reference (table 2). It also adds a temporary `TemporaryAssemblyInertia`
  hook in `sections/controller_section.rs` and `physics/pd_controller.rs`.
- `mass-proof-split.patch`: third run, adds the player-torque split modes
  (table 3). It contains the whole compensation fixture, so apply it alone.
- `real-content-proof.patch`: tables 4 and 5, `splitW` on real catalog ships
  and a test-only station, in `nova_scenario`. It contains its own copy of the
  `TemporaryAssemblyInertia` hook and widens `ship_turn_rate` /
  `slew_rotation` to `pub`, so apply it alone. Its run command is in table 4.

## Rig

Shipped helm path (`helm_app`: `ship_turn_rate` -> `slew_rotation` -> real
`PDController` torque, stack tuning pass) on a warship-like player root
(3x2x15 unit cells, density 1, three computers at `steering_lag 0.5`,
`max_torque 9760`, one vector drive `magnitude 9.0` at z = +8). Partner is a
separate dynamic body with no controller, starboard of the player, joined by
the game's `FixedJoint::new(a, b).with_anchor(..).with_basis(..)` at the
collar midpoint. No `DockedShip`, so only the player's PD acts (the Held case).
Torque ceiling is unchanged in every mode: `sum(max_torque) = 29280`.

## Table 1: unmodified controller

- `yaw90`: 90 degree yaw step, 60 s. `err deg` = overshoot past the command.
- `burn`: hold attitude, drive at full for 3 s, 30 s. `err deg` = max attitude error.
- `I1/Iasm`: player yaw inertia over assembly yaw inertia about the combined
  centre of mass. The PD multiplies its acceleration by the player's inertia only.
- `joint m` / `joint deg`: max drift of the partner pose in the player frame.

```
   row        partner   m2/m1  I1/Iasm   err deg settle s  rings   joint m joint deg
 yaw90       undocked    0.00    1.000      0.00     4.18      0     0.000     0.000
 yaw90  workship-like    0.27    0.739      0.00     4.07      0     0.003     0.007
 yaw90       balanced    1.00    0.379     26.45     7.25      0     0.004     0.009
 yaw90       heavy x4    4.00    0.166     43.45    14.32      1     0.010     0.018
 yaw90    station x20   20.00    0.045     63.54    52.13      4     0.014     0.020
  burn       undocked    0.00    1.000      0.00     0.02      0     0.000     0.000
  burn  workship-like    0.27    0.739      0.17     0.02      0     0.003     0.006
  burn       balanced    1.00    0.379      0.44     0.02      0     0.002     0.004
  burn       heavy x4    4.00    0.166      6.84     4.88      0     0.007     0.015
  burn    station x20   20.00    0.045      6.63     7.63      0     0.007     0.013
```

## Table 2: compensation prototype

Modes, all under the same settings and the same torque ceiling:

- `base`: shipped controller, player inertia only (reproduces table 1).
- `comp`: the PD torque and the stack budget use the assembly tensor about the
  combined centre of mass, in the player root frame, computed once after the
  joint settles. The structural arm stays the player's own `HullRadius`.
- `rigid`: partner cells built into the player root, no joint. The ideal single
  body that `comp` tries to fly. Avian measures its inertia and arm.

Columns:

- `I1 yaw` / `Iasm yaw`: player and assembly yaw moment (engine units).
- `budget`: summed `max_angular_acceleration`, rad/s2. 1.021 is the structural
  ceiling. Station `comp`/`rigid` is torque-bound at 29280 / 38998 = 0.751.
- `err deg`: `yaw90` overshoot past 90 deg; burn rows max attitude error.
- `settle s`: last tick with error > 1 deg or rate > 0.05 rad/s. Equal to the
  run length (60 s or 30 s) means it never settled.
- `peakT`: peak `|sum PD output| / 29280`. Can pass 1 because the clamp is per
  principal axis. `sat s`: seconds at or over 0.99.
- `joint m` / `joint deg`: max partner pose drift in the player frame.
- `tail w` / `tail er` / `tail jw`: last 10 s max player rate (rad/s), max
  error (deg), max partner-vs-player relative angular speed (rad/s).
- `burn-bal`: drive thrust line moved through the assembly centre of mass.
  `burn-off`: drive on the player centreline, off the assembly centre of mass.

```
    case        partner  mode  m2/m1   I1 yaw Iasm yaw I1/Iasm  budget  err deg settle s rings  peakT  sat s joint m joint deg  tail w tail er  tail jw
   yaw90       undocked  base   0.00   1755.0   1755.0   1.000   1.021     0.00     4.17     0  0.061   0.00   0.000    0.000  0.0000   0.000   0.0000
   yaw90       undocked  comp   0.00   1755.0   1755.0   1.000   1.021     0.00     4.17     0  0.061   0.00   0.000    0.000  0.0000   0.000   0.0000
   yaw90       undocked rigid   0.00   1755.0   1755.0   1.000   1.021     0.00     4.17     0  0.061   0.00   0.000    0.000  0.0000   0.000   0.0000
   yaw90  workship-like  base   0.27   1755.0   2374.7   0.739   1.021     0.00     4.07     0  0.061   0.00   0.003    0.007  0.0000   0.000   0.0002
   yaw90  workship-like  comp   0.27   1755.0   2374.7   0.739   1.021     0.00     4.17     0  0.085   0.00   0.004    0.011  0.0000   0.000   0.0001
   yaw90  workship-like rigid   0.00   2374.7   2374.7   1.000   0.985     0.00     4.18     0  0.080   0.00   0.000    0.000  0.0000   0.000   0.0000
   yaw90       balanced  base   1.00   1755.0   4635.0   0.379   1.021    26.45     7.25     0  0.061   0.00   0.004    0.009  0.0000   0.000   0.0000
   yaw90       balanced  comp   1.00   1755.0   4635.0   0.379   1.021     0.05    60.00     0  0.206   0.00   0.013    0.029  0.1259   0.056   0.2026
   yaw90       balanced rigid   0.00   4635.0   4635.0   1.000   0.919     0.00     4.25     0  0.145   0.00   0.000    0.000  0.0000   0.000   0.0000
   yaw90       heavy x4  base   4.00   1755.0  10575.0   0.166   1.021    43.45    14.32     1  0.072   0.00   0.010    0.019  0.0000   0.000   0.0000
   yaw90       heavy x4  comp   4.00   1755.0  10575.0   0.166   1.021     0.58    59.98     0  0.482   0.00   0.102    0.288  1.0314   0.406   1.7079
   yaw90       heavy x4 rigid   0.00  10575.0  10575.0   1.000   0.840     0.00     4.32     0  0.303   0.00   0.000    0.000  0.0000   0.000   0.0000
   yaw90    station x20  base  20.00   1755.0  38997.9   0.045   1.021    63.54    52.13     4  0.085   0.00   0.014    0.020  0.0746   3.489   0.0758
   yaw90    station x20  comp  20.00   1755.0  38997.9   0.045   0.751     1.48    60.00     2  1.366  58.05   0.208    0.273  1.1348   1.483   1.4218
   yaw90    station x20 rigid   0.00  38997.9  38997.9   1.000   0.751     0.00     4.38     0  1.000   0.82   0.000    0.000  0.0000   0.000   0.0000
burn-bal       undocked  base   0.00   1755.0   1755.0   1.000   1.021     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-bal       undocked  comp   0.00   1755.0   1755.0   1.000   1.021     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-bal       undocked rigid   0.00   1755.0   1755.0   1.000   1.021     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-bal  workship-like  base   0.27   1755.0   2374.7   0.739   1.021     0.07     0.02     0  0.011   0.00   0.003    0.006  0.0001   0.000   0.0003
burn-bal  workship-like  comp   0.27   1755.0   2374.7   0.739   1.021     0.08     0.02     0  0.033   0.00   0.005    0.010  0.0000   0.040   0.0000
burn-bal  workship-like rigid   0.00   2374.7   2374.7   1.000   0.985     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-bal       balanced  base   1.00   1755.0   4635.0   0.379   1.021     0.38     0.02     0  0.013   0.00   0.002    0.004  0.0001   0.040   0.0000
burn-bal       balanced  comp   1.00   1755.0   4635.0   0.379   1.021     0.58    29.92     0  0.206   0.00   0.015    0.030  0.1230   0.581   0.2111
burn-bal       balanced rigid   0.00   4635.0   4635.0   1.000   0.919     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-bal       heavy x4  base   4.00   1755.0  10575.0   0.166   1.021     0.15     2.60     0  0.061   0.00   0.014    0.034  0.0002   0.040   0.0002
burn-bal       heavy x4  comp   4.00   1755.0  10575.0   0.166   1.021     1.71    30.00     0  0.482   0.00   0.112    0.672  1.0113   1.208   1.4826
burn-bal       heavy x4 rigid   0.00  10575.0  10575.0   1.000   0.840     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-bal    station x20  base  20.00   1755.0  38997.9   0.045   1.021     0.13     2.73     0  0.061   0.00   0.013    0.024  0.0003   0.040   0.0004
burn-bal    station x20  comp  20.00   1755.0  38997.9   0.045   0.751     3.07    30.00     0  1.366  28.32   0.164    0.306  0.9889   2.305   1.0507
burn-bal    station x20 rigid   0.00  38997.9  38997.9   1.000   0.751     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-off       undocked  base   0.00   1755.0   1755.0   1.000   1.021     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-off       undocked  comp   0.00   1755.0   1755.0   1.000   1.021     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-off       undocked rigid   0.00   1755.0   1755.0   1.000   1.021     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000
burn-off  workship-like  base   0.27   1755.0   2374.7   0.739   1.021     0.17     0.02     0  0.030   0.00   0.003    0.006  0.0000   0.040   0.0001
burn-off  workship-like  comp   0.27   1755.0   2374.7   0.739   1.021     0.13     0.02     0  0.037   0.00   0.005    0.011  0.0001   0.040   0.0003
burn-off  workship-like rigid   0.00   2374.7   2374.7   1.000   0.985     0.17     0.02     0  0.020   0.00   0.000    0.000  0.0000   0.040   0.0000
burn-off       balanced  base   1.00   1755.0   4635.0   0.379   1.021     0.66     0.02     0  0.045   0.00   0.002    0.004  0.0001   0.040   0.0001
burn-off       balanced  comp   1.00   1755.0   4635.0   0.379   1.021     0.58    30.00     0  0.206   0.00   0.012    0.029  0.1253   0.400   0.2160
burn-off       balanced rigid   0.00   4635.0   4635.0   1.000   0.919     0.20     0.02     0  0.048   0.00   0.000    0.000  0.0000   0.040   0.0000
burn-off       heavy x4  base   4.00   1755.0  10575.0   0.166   1.021     6.77     4.88     0  0.061   0.00   0.006    0.013  0.0007   0.040   0.0008
burn-off       heavy x4  comp   4.00   1755.0  10575.0   0.166   1.021     1.65    30.00     0  0.482   0.00   0.114    0.314  1.0488   1.526   1.6454
burn-off       heavy x4 rigid   0.00  10575.0  10575.0   1.000   0.840     0.14     0.02     0  0.077   0.00   0.000    0.000  0.0000   0.040   0.0000
burn-off    station x20  base  20.00   1755.0  38997.9   0.045   1.021     6.73     7.73     0  0.064   0.00   0.010    0.015  0.0004   0.000   0.0004
burn-off    station x20  comp  20.00   1755.0  38997.9   0.045   0.751     2.62    30.00     0  1.366  28.77   0.189    0.290  1.9536   2.538   2.0243
burn-off    station x20 rigid   0.00  38997.9  38997.9   1.000   0.751     0.04     0.02     0  0.092   0.00   0.000    0.000  0.0000   0.040   0.0000
repeat: identical (45 rows)
```

The run built the table twice in one process. All 45 rows matched bit for
bit. The `base` and `comp` rows also match the first cargo run of the same
fixture before the `rigid` mode was added.

## Findings

- Unmodified: overshoot grows with mass ratio (26 deg at 1:1, 63 deg and 4
  reversals at 20:1). Peak torque stays at 6-9 % of the ceiling. The loop is
  not torque-limited. It under-drives because it scales by `I1`.
- `rigid` reference: 0 deg overshoot and 4.2-4.4 s settle at every ratio,
  with the same torque ceiling. The station row saturates for 0.8 s and still
  lands. So the torque ceiling can turn the heavy assembly without a raise.
- `comp` removes most overshoot (26 -> 0.05, 43 -> 0.58, 63 -> 1.48 deg), but
  no docked `comp` row at ratio 1:1 or higher settles. In `yaw90` at 1:1 the
  tail error is 0.06 deg, so `settle` fails on rate jitter, not on position.
  In the last 10 s the pair keeps a relative spin of 0.2 rad/s (1:1) and
  1.0-2.0 rad/s (4:1, 20:1) across the joint, and the player rate follows it.
  Joint pose drift rises 3-20x over `base` (to 0.1-0.2 m, 0.3-0.7 deg). A
  large relative rate with a small angle is chatter near the fixed-step rate,
  not a slow structural mode. Station `comp` sits at the torque clamp for 58 of
  60 s on two principal axes at once (`peakT` 1.366), so it bang-bangs in pitch
  or roll as well as yaw. The 0.27 workship-like row stays clean.
- The inertia tensor is not the problem: `comp` assembly yaw inertia equals
  avian's measured inertia of the `rigid` body in every row (yaw element only;
  pitch and roll elements not compared). The torque ceiling is not the problem:
  `rigid` flies the same mass with the same ceiling.
- Likely cause (hypothesis, ablations not run): a sampled-control limit. The
  shipped stack gives a total damping gain of 72 (3 x 4.5 x 2 x 4 x 0.667), so
  `kd * dt` is about 1.1 at a 64 Hz fixed step (the Bevy default, not checked
  in this rig). `comp` scales the torque by `Iasm`, but the torque goes into
  the player root only (`sync_controller_section_forces`), and inside one step
  the root responds nearer `I1` than `Iasm`. The explicit damper is stable only
  while `kd * dt * Iasm / I1 < 2`, a ratio limit near `Iasm / I1 = 1.8`. The
  data fits: 1.35 (workship-like) is clean, 2.64 (1:1) and higher chatter.
- Joint physics is therefore the design conflict. Assembly compensation alone
  on the root torque is not safe from about 1:1 up.
- Ablations that would separate the causes: scale the gains by `I1 / Iasm`,
  split the torque across both bodies by inertia, raise joint solver
  iterations or substeps, merge the pair into one compound body while docked.

## Not tested

- Actual hull geometry: all hulls are unit-cell boxes. Collar cells are massless.
  No real ship content, off-centre collars, or real section mass layout.
- Station behavior: the "station x20" partner is a dense free dynamic body,
  not a station. No kinematic or static anchor and no station content.
- The `DockedShip` path, both PDs acting, and undock or re-dock transitions.
- Pitch and roll steps, and the assembly arm in the structural ceiling of
  `comp` (it uses the player arm, so its budget is 10-20 % above `rigid` at
  1:1 and 4:1; at 20:1 both are torque-bound at 0.751 and `comp` still rings).
- A merged compound body.

## Table 3: player-torque split prototype

Scope approved by the owner: split only the PLAYER-COMMANDED helm torque over
both docked roots. The partner has no controller, no thrusters and no AI. No
torque on the partner comes from anywhere except the player's PD output.

New modes, same rig, same gains, same torque ceiling (29280):

- `splitT`: `comp` torque (assembly tensor), then each root gets a pure torque
  `I_i,asm * a`, with `a = Iasm^-1 * tau`. `I_i,asm` is that root's tensor
  about the assembly centre of mass (parallel axis written out). The torques
  sum to `tau`.
- `splitW`: `comp` torque, then each root gets the rigid-body wrench: torque
  `I_i,own * a` about its own centre of mass, plus a force `m_i * (a x r_i)`
  at its centre of mass. `r_i` is the offset from the assembly centre of mass.
  The forces sum to zero. Torques plus `r x F` sum to `tau`.

How it is applied: `sync_controller_section_forces` still puts all of `tau` on
the player root. A test-only system (`split_helm_torque`) runs after it in the
same fixed step. It subtracts `tau` from the player and applies the split to
both roots through avian `Forces`. Masses, tensors and offsets are taken once
after the joint settles, in the player root frame, and rotated by the player
rotation every step. No production interface was added beyond the table 2
`TemporaryAssemblyInertia` hook.

Changes from table 2:

- Every mode now does the same number of settle updates. Before this change,
  `rigid` did 4 and the jointed modes did 8. That fixed-step phase offset
  moved only `rigid` rows (station `yaw90` settle 4.38 -> 4.37 s). `base` and
  `comp` rows match table 2 exactly.
- `tq P` / `tq S`: yaw share of a unit yaw command applied as TORQUE to the
  player / partner root. It is computed from the split, not measured. In
  `splitW`, the rest goes through the `r x F` force pair, so the two do not
  sum to 1. Other modes show 1 / 0.
- `dL w`: max over the run of `|L - L_rigid(t)|` divided by `Iasm yaw`. It is
  the same-tick angular momentum error against the `rigid` run, expressed as
  an assembly rate in rad/s. `L` is about the combined centre of mass, and
  both runs have the same update count.

Undocked rows are identical in all five modes. They are omitted.

```
    case        partner   mode  m2/m1   I1 yaw Iasm yaw I1/Iasm  budget   tq P   tq S  err deg settle s rings  peakT  sat s joint m joint deg  tail w tail er  tail jw   dL w
   yaw90  workship-like   base   0.27   1755.0   2374.7   0.739   1.021  1.000  0.000     0.00     4.07     0  0.061   0.00   0.003    0.007  0.0000   0.000   0.0002 0.0712
   yaw90  workship-like   comp   0.27   1755.0   2374.7   0.739   1.021  1.000  0.000     0.00     4.17     0  0.085   0.00   0.004    0.009  0.0000   0.000   0.0001 0.0342
   yaw90  workship-like splitT   0.27   1755.0   2374.7   0.739   1.021  0.781  0.219     0.00     4.17     0  0.085   0.00   0.002    0.005  0.0000   0.000   0.0001 0.0343
   yaw90  workship-like splitW   0.27   1755.0   2374.7   0.739   1.021  0.739  0.061     0.00     4.17     0  0.083   0.00   0.001    0.002  0.0000   0.000   0.0000 0.0342
   yaw90  workship-like  rigid   0.00   2374.7   2374.7   1.000   0.985  1.000  0.000     0.00     4.18     0  0.080   0.00   0.000    0.000  0.0000   0.000   0.0000 0.0000
   yaw90       balanced   base   1.00   1755.0   4635.0   0.379   1.021  1.000  0.000    26.45     7.25     0  0.061   0.00   0.004    0.009  0.0000   0.000   0.0000 0.4450
   yaw90       balanced   comp   1.00   1755.0   4635.0   0.379   1.021  1.000  0.000     0.05    60.00     0  0.206   0.00   0.013    0.029  0.1259   0.056   0.2026 0.0859
   yaw90       balanced splitT   1.00   1755.0   4635.0   0.379   1.021  0.500  0.500     0.00     4.17     0  0.167   0.00   0.002    0.000  0.0001   0.000   0.0001 0.0888
   yaw90       balanced splitW   1.00   1755.0   4635.0   0.379   1.021  0.379  0.379     0.00     4.17     0  0.162   0.00   0.001    0.000  0.0000   0.000   0.0000 0.0888
   yaw90       balanced  rigid   0.00   4635.0   4635.0   1.000   0.919  1.000  0.000     0.00     4.25     0  0.145   0.00   0.000    0.000  0.0000   0.000   0.0000 0.0000
   yaw90       heavy x4   base   4.00   1755.0  10575.0   0.166   1.021  1.000  0.000    43.45    14.32     1  0.072   0.00   0.010    0.019  0.0000   0.000   0.0000 0.5524
   yaw90       heavy x4   comp   4.00   1755.0  10575.0   0.166   1.021  1.000  0.000     0.58    59.98     0  0.482   0.00   0.102    0.288  1.0314   0.406   1.7079 0.0984
   yaw90       heavy x4 splitT   4.00   1755.0  10575.0   0.166   1.021  0.302  0.698     0.00     4.17     0  0.377   0.00   0.004    0.007  0.0000   0.000   0.0000 0.1462
   yaw90       heavy x4 splitW   4.00   1755.0  10575.0   0.166   1.021  0.166  0.664     0.00     4.17     0  0.369   0.00   0.001    0.002  0.0000   0.000   0.0000 0.1439
   yaw90       heavy x4  rigid   0.00  10575.0  10575.0   1.000   0.840  1.000  0.000     0.00     4.32     0  0.303   0.00   0.000    0.000  0.0000   0.000   0.0000 0.0000
   yaw90    station x20   base  20.00   1755.0  38997.9   0.045   1.021  1.000  0.000    63.54    52.13     4  0.085   0.00   0.014    0.020  0.0746   3.489   0.0758 0.5523
   yaw90    station x20   comp  20.00   1755.0  38997.9   0.045   0.751  1.000  0.000     1.48    60.00     2  1.366  58.05   0.208    0.273  1.1348   1.483   1.4218 0.0917
   yaw90    station x20 splitT  20.00   1755.0  38997.9   0.045   0.751  0.097  0.903     0.01     4.35     0  1.302   1.10   0.005    0.008  0.0110   0.007   0.0071 0.0230
   yaw90    station x20 splitW  20.00   1755.0  38997.9   0.045   0.751  0.045  0.900     0.00     4.35     0  1.003   0.98   0.001    0.001  0.0004   0.001   0.0003 0.0229
   yaw90    station x20  rigid   0.00  38997.9  38997.9   1.000   0.751  1.000  0.000     0.00     4.37     0  1.000   0.82   0.000    0.000  0.0000   0.000   0.0000 0.0000
burn-bal  workship-like   base   0.27   1755.0   2374.7   0.739   1.021  1.000  0.000     0.07     0.02     0  0.011   0.00   0.003    0.006  0.0001   0.000   0.0003 0.0026
burn-bal  workship-like   comp   0.27   1755.0   2374.7   0.739   1.021  1.000  0.000     0.07     0.02     0  0.034   0.00   0.005    0.008  0.0005   0.040   0.0015 0.0051
burn-bal  workship-like splitT   0.27   1755.0   2374.7   0.739   1.021  0.781  0.219     0.09     0.02     0  0.035   0.00   0.004    0.010  0.0001   0.040   0.0003 0.0055
burn-bal  workship-like splitW   0.27   1755.0   2374.7   0.739   1.021  0.739  0.061     0.06     0.02     0  0.038   0.00   0.003    0.006  0.0000   0.040   0.0003 0.0051
burn-bal  workship-like  rigid   0.00   2374.7   2374.7   1.000   0.985  1.000  0.000     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000 0.0000
burn-bal       balanced   base   1.00   1755.0   4635.0   0.379   1.021  1.000  0.000     0.38     0.02     0  0.013   0.00   0.002    0.004  0.0001   0.040   0.0000 0.0025
burn-bal       balanced   comp   1.00   1755.0   4635.0   0.379   1.021  1.000  0.000     0.58    29.92     0  0.206   0.00   0.015    0.030  0.1230   0.581   0.2111 0.0185
burn-bal       balanced splitT   1.00   1755.0   4635.0   0.379   1.021  0.500  0.500     0.34     0.02     0  0.031   0.00   0.002    0.004  0.0001   0.040   0.0001 0.0050
burn-bal       balanced splitW   1.00   1755.0   4635.0   0.379   1.021  0.379  0.379     0.38     0.02     0  0.032   0.00   0.002    0.004  0.0001   0.040   0.0001 0.0045
burn-bal       balanced  rigid   0.00   4635.0   4635.0   1.000   0.919  1.000  0.000     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000 0.0000
burn-bal       heavy x4   base   4.00   1755.0  10575.0   0.166   1.021  1.000  0.000     0.15     2.60     0  0.061   0.00   0.014    0.034  0.0002   0.040   0.0002 0.0068
burn-bal       heavy x4   comp   4.00   1755.0  10575.0   0.166   1.021  1.000  0.000     1.71    30.00     0  0.482   0.00   0.112    0.672  1.0113   1.208   1.4826 0.0587
burn-bal       heavy x4 splitT   4.00   1755.0  10575.0   0.166   1.021  0.302  0.698     0.21     2.75     0  0.377   0.00   0.011    0.035  0.0005   0.040   0.0006 0.0276
burn-bal       heavy x4 splitW   4.00   1755.0  10575.0   0.166   1.021  0.166  0.664     0.14     2.27     0  0.377   0.00   0.010    0.015  0.0007   0.040   0.0004 0.0211
burn-bal       heavy x4  rigid   0.00  10575.0  10575.0   1.000   0.840  1.000  0.000     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000 0.0000
burn-bal    station x20   base  20.00   1755.0  38997.9   0.045   1.021  1.000  0.000     0.13     2.73     0  0.061   0.00   0.013    0.024  0.0003   0.040   0.0004 0.0023
burn-bal    station x20   comp  20.00   1755.0  38997.9   0.045   0.751  1.000  0.000     3.07    30.00     0  1.366  28.32   0.164    0.306  0.9889   2.305   1.0507 0.0767
burn-bal    station x20 splitT  20.00   1755.0  38997.9   0.045   0.751  0.097  0.903     0.20     3.02     0  1.007   0.80   0.012    0.026  0.0114   0.040   0.0078 0.0186
burn-bal    station x20 splitW  20.00   1755.0  38997.9   0.045   0.751  0.045  0.900     0.20     2.90     0  1.006   0.82   0.012    0.022  0.0004   0.040   0.0004 0.0209
burn-bal    station x20  rigid   0.00  38997.9  38997.9   1.000   0.751  1.000  0.000     0.00     0.02     0  0.000   0.00   0.000    0.000  0.0000   0.000   0.0000 0.0000
burn-off  workship-like   base   0.27   1755.0   2374.7   0.739   1.021  1.000  0.000     0.17     0.02     0  0.030   0.00   0.003    0.006  0.0000   0.040   0.0001 0.0018
burn-off  workship-like   comp   0.27   1755.0   2374.7   0.739   1.021  1.000  0.000     0.14     0.02     0  0.044   0.00   0.003    0.008  0.0000   0.040   0.0000 0.0045
burn-off  workship-like splitT   0.27   1755.0   2374.7   0.739   1.021  0.781  0.219     0.15     0.02     0  0.034   0.00   0.003    0.008  0.0001   0.040   0.0004 0.0038
burn-off  workship-like splitW   0.27   1755.0   2374.7   0.739   1.021  0.739  0.061     0.13     0.02     0  0.026   0.00   0.002    0.003  0.0000   0.040   0.0000 0.0019
burn-off  workship-like  rigid   0.00   2374.7   2374.7   1.000   0.985  1.000  0.000     0.17     0.02     0  0.020   0.00   0.000    0.000  0.0000   0.040   0.0000 0.0000
burn-off       balanced   base   1.00   1755.0   4635.0   0.379   1.021  1.000  0.000     0.66     0.02     0  0.045   0.00   0.002    0.004  0.0001   0.040   0.0001 0.0059
burn-off       balanced   comp   1.00   1755.0   4635.0   0.379   1.021  1.000  0.000     0.58    30.00     0  0.206   0.00   0.012    0.029  0.1253   0.400   0.2160 0.0222
burn-off       balanced splitT   1.00   1755.0   4635.0   0.379   1.021  0.500  0.500     0.20     0.02     0  0.058   0.00   0.001    0.000  0.0002   0.040   0.0001 0.0019
burn-off       balanced splitW   1.00   1755.0   4635.0   0.379   1.021  0.379  0.379     0.20     0.02     0  0.048   0.00   0.000    0.000  0.0001   0.040   0.0001 0.0001
burn-off       balanced  rigid   0.00   4635.0   4635.0   1.000   0.919  1.000  0.000     0.20     0.02     0  0.048   0.00   0.000    0.000  0.0000   0.040   0.0000 0.0000
burn-off       heavy x4   base   4.00   1755.0  10575.0   0.166   1.021  1.000  0.000     6.77     4.88     0  0.061   0.00   0.006    0.013  0.0007   0.040   0.0008 0.1137
burn-off       heavy x4   comp   4.00   1755.0  10575.0   0.166   1.021  1.000  0.000     1.65    30.00     0  0.482   0.00   0.114    0.314  1.0488   1.526   1.6454 0.0556
burn-off       heavy x4 splitT   4.00   1755.0  10575.0   0.166   1.021  0.302  0.698     0.23     0.02     0  0.377   0.00   0.005    0.010  0.0003   0.040   0.0003 0.0145
burn-off       heavy x4 splitW   4.00   1755.0  10575.0   0.166   1.021  0.166  0.664     0.22     2.48     0  0.377   0.00   0.004    0.010  0.0006   0.040   0.0007 0.0130
burn-off       heavy x4  rigid   0.00  10575.0  10575.0   1.000   0.840  1.000  0.000     0.14     0.02     0  0.077   0.00   0.000    0.000  0.0000   0.040   0.0000 0.0000
burn-off    station x20   base  20.00   1755.0  38997.9   0.045   1.021  1.000  0.000     6.73     7.73     0  0.064   0.00   0.010    0.015  0.0004   0.000   0.0004 0.0760
burn-off    station x20   comp  20.00   1755.0  38997.9   0.045   0.751  1.000  0.000     2.62    30.00     0  1.366  28.77   0.189    0.290  1.9536   2.538   2.0243 0.0842
burn-off    station x20 splitT  20.00   1755.0  38997.9   0.045   0.751  0.097  0.903     0.28     2.28     0  1.006   0.68   0.007    0.016  0.0116   0.040   0.0079 0.0162
burn-off    station x20 splitW  20.00   1755.0  38997.9   0.045   0.751  0.045  0.900     0.23     2.58     0  1.016   0.63   0.005    0.010  0.0011   0.040   0.0006 0.0183
burn-off    station x20  rigid   0.00  38997.9  38997.9   1.000   0.751  1.000  0.000     0.04     0.02     0  0.092   0.00   0.000    0.000  0.0000   0.040   0.0000 0.0000
repeat: identical (75 rows)
```

Logs (git-ignored by `*.log`, local only):
`mass-proof-split-run1.log` and `-run2.log` (older `dL` normalization, same
code, byte-identical to each other), `-run3.log` (current `dL w`, unaligned
rigid settle), `-run4.log` (this table). Across runs 1-3, every column except
`dL` matched. Each run also built the table twice in one process, and both
builds matched bit for bit.

### Before / after (docked `yaw90`, observed)

```
                    comp (root torque only)          splitW
ratio   settle s  tail jw  joint m  sat s     settle s  tail jw  joint m  sat s
0.27      4.17     0.0001   0.004   0.00        4.17     0.0000   0.001   0.00
1:1      60.00*    0.2026   0.013   0.00        4.17     0.0000   0.001   0.00
4:1      59.98*    1.7079   0.102   0.00        4.17     0.0000   0.001   0.00
20:1     60.00*    1.4218   0.208  58.05        4.35     0.0003   0.001   0.98
rigid reference settle: 4.18 / 4.25 / 4.32 / 4.37 s; 20:1 rigid sat 0.82 s
* never settled
```

## Findings (split)

- Observed: with the same gains, assembly tensor and torque ceiling, both
  split modes turn every docked ratio 90 degrees with at most 0.01 deg
  overshoot, 0 reversals, and a settle of 4.17-4.35 s. The rigid body settles
  in 4.18-4.37 s. `comp` never settled from 1:1 up.
- Observed in docked `yaw90` from 1:1 up: the chatter is gone. The tail
  relative spin across the joint falls from 0.20-1.71 rad/s (`comp`) to at
  most 0.0071 rad/s (`splitT`) and 0.0003 rad/s (`splitW`). Joint drift falls
  from 0.013-0.208 m to at most 0.005 m (`splitT`) and 0.001 m (`splitW`).
- Observed: `splitW` loads the joint less than `splitT`. Station `yaw90`:
  tail jw 0.0003 vs 0.0071 rad/s, joint 0.001 vs 0.005 m. `splitT` spins each
  root with more torque than its own inertia needs. The joint must turn the
  surplus into orbital motion about the assembly centre of mass.
- Observed: saturation at 20:1. `splitW` holds `peakT` 1.003 for 0.98 s,
  close to `rigid` (1.000, 0.82 s). `splitT` reaches 1.302 for 1.10 s. `comp`
  stayed on the clamp for 58 s. The torque ceiling is unchanged.
- Observed: burns. In `burn-off`, the split rows hold attitude within
  0.13-0.28 deg. `base` drifts 6.7 deg at 4:1 and 20:1, `comp` never settles
  from 1:1 up, and `rigid` holds 0.04-0.20 deg. In `burn-bal`, the split rows hold within
  0.06-0.38 deg against 0.00 for `rigid`, and settle in 2.3-3.0 s at 4:1 and
  20:1. The drive still pushes only the player root, so the joint carries the
  thrust in every jointed mode. The split does not touch thrust.
- Momentum: the split only redistributes `tau`, so the total `L` of `splitT`,
  `splitW` and `comp` is nearly the same (`dL w` 0.034-0.15 rad/s in `yaw90`).
  `dL w` therefore does not rank the split modes. Use the joint and tail
  columns for that. Inference, not isolated: most of the `yaw90` gap to `rigid`
  comes from the budget. The split modes keep the player's `HullRadius` arm
  (1.021) where `rigid` gets 0.985 / 0.919 / 0.840, so they turn faster. At
  20:1 both are torque-bound at 0.751, and `dL w` falls to 0.023.
- The table 2 hypothesis holds up: the same controller and tensor are stable
  once the torque reaches each root in proportion to its inertia. This is
  consistent with a root-only torque limit. It is not a proof of the cause.
- Angular impulse: the split needed no new production interface in this rig.
  It used avian `Forces` on both roots from a test system. A production
  version would need an owner that reads both roots' mass properties and
  applies torque, and for `splitW` also force, to a root the player does not
  own. That ownership is not decided here.

## Not tested (split)

- Real hull geometry, real section mass layout, off-centre collars, massive
  collar cells. All hulls are unit-cell boxes.
- A real station: kinematic or static anchors, station content, station
  controllers. "station x20" is a dense free dynamic body.
- Pitch and roll steps, combined-axis commands, sustained player slews.
- Partner AI, partner thrusters or partner PD. All are absent by design.
- The `DockedShip` path, the take/relinquish toggle, undock and re-dock
  transitions, and a split that tracks a changing assembly (damage, mass
  change). The split parts are fixed after settle.
- The assembly arm in the structural budget. The split uses the player arm.
- Frame rate: the rig advances 1/60 s per update against the default 64 Hz
  fixed step. Other step rates were not run.
- Rendered or player-flow behavior. No `nova-probe` or `nova-bench` run. This
  table does not show that docked gameplay is safe.

## Table 4: splitW on real content (temporary fixture, removed)

Scope approved by the owner: a disposable physics proof of the
player-commanded `splitW` torque and force on real catalog ships. This is not
the production helm. `real-content-proof.patch` restores the fixture. Run it
with:
`nix develop --command cargo test -p nova_scenario --lib temporary_docked_helm_real_content_table -- --ignored --nocapture`
(`PROOF_STATION=1` for table 5, `PROOF_REPEAT=1` for the in-process repeat,
`PROOF_ONLY="<case> <partner> <mode>"` for one row).

### Rig

- Headless `unfinished_integrity_physics_app` + `PDControllerPlugin` +
  `SpaceshipSectionPlugin { render: false }` + `SpaceshipPlugin`. The
  catalogs are deserialized from the generated `assets/base/sections` and
  `assets/base/ships` RON (read only).
- Ships are spawned by `spaceship_scenario_object` and the real
  `insert_spaceship_sections`. The player is `block_line_warship`: 155
  sections, mass 185.4, three computers (sum `max_torque` 29280),
  `HullRadius` 11.36. The drives are the warship's own: side drives at
  (+-2, 0, 4) with magnitude 1, and the vector drive at (0, 0, 8.5) with
  magnitude 9.
- Partners, all real catalog content, all `RigidBody::Dynamic`:
  `block_workship` (m2/m1 0.295), `block_frame_tender` (0.464),
  `block_line_warship` (1.000). Each docks with its `port_collar` against the
  player's `starboard_collar` (the tracked collar change in this sprout).
- The dock is the real `DockingConnectionRequest`: real port pair, real
  `FixedJoint` anchor and basis, `DockedShip` on both roots. The face gap at
  capture is 0.5 cell (5 m). At 0.1 cell a workship collider touched the
  warship before the dock (pre-dock speed 0.39), so 0.1 was rejected.
- Take control = remove `DockedShip` from the player root only. The partner
  stays neutral, so its PD torque and its thrust stay off
  (`sync_controller_section_forces`, `thruster_impulse_system`).
  Relinquish = insert `DockedShip` again. There is no production toggle.
- Helm: the stacking rig's `ship_turn_rate` -> `slew_rotation` command path
  (temporarily widened to `pub`). `NovaFlightPlugin` is not added. Drive
  inputs are written directly.
- Modes: `base` (shipped, player inertia, root torque only), `comp` (assembly
  tensor, root torque only), `splitW` (assembly tensor, rigid-body wrench over
  both roots, gated off while the player is neutral), `rigid` (partner
  sections merged into the player root through an Inline design, partner
  computers stripped, so the reference has the same three computers and the
  same drives).
- Cases: `yaw90`/`pitch90`/`roll90` steps, 40 s. `burn-all`: all three
  forward drives for 3 s, 30 s. `burn-bal`: the forward-drive subset with the
  smallest moment about the assembly centre of mass. That is always the
  starboard side drive alone, 1/11 of full thrust, so it is a weak burn.
  `auth`: yaw 45, neutral from 2 s to 12 s (mid-slew), then resume, 40 s.
  `redock`: yaw 90, `DockingReleaseRequest` at 20 s, `DockingConnectionRequest`
  at 22 s, split recomputed, yaw back to 0, 50 s.
- Columns as in table 2/3, plus: `off deg` is the max off-axis attitude
  error, `jw max` is the max partner-vs-player relative angular speed over the
  whole run, `arm m` is the burn thrust line's moment arm about the assembly
  centre of mass, and `alive` means the connection and `FixedJoint` exist at
  the end. Transition columns: `neut deg` is the assembly turn during neutral,
  `neut dL` is `|L(12 s) - L(2 s)| / Iasm yaw` (rad/s), `re m` / `re deg` are
  the relative pose change at re-dock, `after s` is the settle time after
  resume or re-dock, and `after er` is the final error in degrees. 0.040 is
  the f32 `angle_between` floor.

### Real content: yaw/pitch/roll 90 (selected columns; full rows in the logs)

```
case    partner  ratio   mode   err deg  settle  rings  joint m  jw max  tail jw
yaw90   workship 0.295   base     0.00    3.65     0    0.001   0.0197  0.0002
                         comp     0.00    4.47     0    0.002   0.0230  0.0002
                         splitW   0.00    4.47     0    0.000   0.0097  0.0002
                         rigid    0.00    4.48     0    0.000   -       -
yaw90   tender   0.464   base     5.42    5.32     0    0.002   0.0159  0.0002
                         comp     0.00    4.47     0    0.003   0.0205  0.0002
                         splitW   0.00    4.47     0    0.001   0.0069  0.0002
                         rigid    0.00    4.53     0    0.000   -       -
yaw90   warship  1.000   base    36.18   10.25     1    0.002   0.0224  0.0001
                         comp     0.00    4.47     0    0.005   0.0779  0.0302
                         splitW   0.00    4.47     0    0.001   0.0018  0.0001
                         rigid    0.00    4.50     0    0.000   -       -
pitch90 warship  1.000   base    31.75    7.42     0    0.001   0.0234  0.0001
                         comp     0.01    4.47     0    0.005   0.0701  0.0521
                         splitW   0.00    4.47     0    0.000   0.0002  0.0002
                         rigid    0.00    4.50     0    0.000   -       -
roll90  workship 0.295   base    46.24   16.02     1    0.000   0.0090  0.0002
roll90  tender   0.464   base    51.32   22.48     2    0.001   0.0061  0.0002
roll90  warship  1.000   base    57.70   33.42     3    0.000   0.0060  0.0002
                         comp     0.04    4.38     0    0.003   0.0493  0.0404
                         splitW   0.00    4.45     0    0.000   0.0019  0.0001
                         rigid    0.00    4.50     0    0.000   -       -
```

`base` warship `roll90` tail error is 7.0 deg at 40 s. Roll inertia is where
the player is smallest (I1 roll 370 vs Iasm roll 1728-3543), so `base` rings
hardest in roll even at 0.295.

### Real content: burns, authority, re-dock

- `burn-all` (thrust line 12.6-27.5 m off the assembly centre of mass): the
  max attitude error for `splitW` is 0.10 / 0.13 / 0.13 deg (workship, tender,
  warship). For `rigid` it is 0.11 / 0.14 / 0.13 deg. Warship `base` drifts
  0.41 deg, and `comp` holds 0.22 deg with a tail relative spin of 0.053
  rad/s.
- `burn-bal` (7.6 m arm, one side drive): every row holds within 0.00-0.02
  deg except warship `comp`, which keeps 0.028 rad/s tail chatter.
- `auth`: in every jointed mode, `neut dL` is 0.0000-0.0017 rad/s. So the
  player applies no torque while neutral. The pair coasts on the momentum
  it had at 2 s (94-165 deg in 10 s). After resume, `splitW` settles in
  4.48 s against `rigid` 4.53-4.67 s. The warship `comp` / `base` rows take
  7.9 s. In `auth` rows, `err deg`, `rings` and `settle` span the whole run,
  including the neutral coast. They measure the coast, not the controller.
  Only `after s` and `after er` grade the resumed helm. In `redock` rows,
  `err deg` is the max over both legs and `rings` is their sum. `after s`
  grades the leg after the re-dock.
- `redock`: in every real-content row the re-dock is accepted. The pose change
  at re-capture is at most 0.007 m and 0.001 deg. After re-dock, `splitW`
  settles in 4.47 s against `rigid` 4.48-4.53 s. Joint drift after re-dock is
  <= 0.001 m.
- `alive` is true in all 91 rows.

## Table 5: test-only heavy dynamic station (NOT shipped content)

The owner chose option C. `temporary_test_station` exists only inside the
test app, never in the catalog. It is a dynamic ship: a solid 16x10x24 box of
3840 real `reinforced_hull_section` cells with one real
`docking_port_section` on its -X face (hatch outboard, the same turn as the
catalog port collars). m2/m1 = 20.718. `rigid` is 3996 sections, mass
4194.8. The row is torque-bound: `budget` 0.099 (split) vs 0.094 (rigid).

```
case     mode   err deg  settle  rings  peakT  sat s  joint m joint d  jw max  tail jw
yaw90    base    76.94   39.98*    0    0.113   0.00   0.001   0.002  0.0150  0.0150
         comp     8.47   39.98*    0    1.334  39.22   0.017   0.040  0.2071  0.1698
         splitW   8.90   10.77     0    1.334   9.82   0.001   0.001  0.0066  0.0005
         rigid    9.09   11.08     0    1.002  10.32   0.000   0.000  -       -
pitch90  comp     8.38   39.97*    1    1.334  39.47   0.021   0.050  0.2490  0.2490
         splitW   8.90   10.77     0    0.905   0.00   0.000   0.000  0.0012  0.0005
         rigid    9.06   11.08     0    0.758   0.00   0.000   0.000  -       -
roll90   comp     9.88   39.97*    1    1.334  39.15   0.019   0.049  0.2311  0.2139
         splitW   8.91   10.77     0    1.117   0.15   0.000   0.001  0.0074  0.0004
         rigid    9.07   11.08     0    0.486   0.00   0.000   0.000  -       -
burn-all base    13.05   29.98*    0    0.110   0.00   0.001   0.002  0.0142  0.0104
         comp     0.58   29.98*    0    1.334  29.30   0.018   0.042  0.2150  0.1905
         splitW   0.12    0.02     0    1.334   1.67   0.001   0.003  0.0151  0.0004
         rigid    0.00    0.02     0    0.318   0.00   0.000   0.000  -       -
* never settled. base pitch/roll never reach the command (tail error 75 / 41 deg).
```

- Station `burn-all` arm 114.6 m, `burn-bal` arm 94.6 m (one side drive).
  In `burn-bal`, `splitW` holds 0.00 deg and `comp` chatters (tail jw 0.27
  rad/s).
- Station `auth` (the same caveat as above: only `after s` / `after er` grade
  the helm): `neut dL` 0.0000-0.0002 in every mode. After resume,
  `splitW` settles in 25.87 s against `rigid` 25.48 s. Both end at the error
  floor. `comp` ends with 0.21 deg error and 0.20 rad/s tail chatter. `base`
  ends at 1.16 deg.
- Station `redock`: `splitW` re-docks with a 0.014 m / 0.002 deg pose change
  and then flies like `rigid` (settle after re-dock 10.77 vs 11.08 s).
  `comp` re-docks 1.887 m / 0.54 deg away from its first pose, because its
  chatter keeps the pair moving through the 2 s free window. It then reaches
  0.084 m / 0.31 deg joint drift and 1.11 rad/s relative spin, and stays on
  the clamp for 47 s. `base` is refused at 22 s: it had not finished the
  first slew (76 deg overshoot) when it was released, so after release the
  player and station separate beyond the capture envelope. The gate refuses
  correctly. This is not a joint failure.
- `peakT` 1.334 for `splitW` and `comp` is the per-principal-axis clamp on
  two axes at once. `rigid` stays at 1.002. `splitW` holds the clamp for
  9.8 s in `yaw90` against 10.3 s for `rigid`. `comp` holds it for 39 s.

## Repeats

- `real-content-run1.log`: 91 rows, built twice in one process
  (`repeat: identical (91 rows)`). `real-content-run2.log`: a separate
  process. Every row and setup line is byte-identical to run 1.
- `test-station-run1.log`: 28 rows, in-process repeat identical.
  `test-station-run2.log`: a separate process, byte-identical rows.

## Findings (real content)

- Observed: on real catalog geometry and mass, `splitW` flies the docked pair
  like the merged rigid body on every axis and ratio tested, 0.295 to 20.7.
  The settle time is within 0.35 s of `rigid` in every step row (1:1:
  4.47 vs 4.50 s; station: 10.77 vs 11.08 s). Overshoot is equal or lower,
  and there are no reversals.
- Observed: `splitW` keeps joint drift at <= 0.001 m and <= 0.003 deg in all
  jointed rows, and the max relative spin at <= 0.0151 rad/s. The joint is
  alive at the end of every row, and a re-dock after release holds the same
  pose to within 0.014 m.
- Observed: `base` is not acceptable from 0.295 up in roll, or from 0.464 up in
  yaw (overshoot 5-77 deg, up to 3 reversals, and pitch/roll at 20.7 never
  reach the command). `comp` removes the overshoot but chatters across the
  joint from 1:1 up (tail relative spin 0.03-0.053 rad/s at 1:1, 0.17-0.27
  at 20.7), never settles at 20.7, and destabilizes the re-dock.
- Observed: neutral is clean. With `DockedShip` back on the player root, the
  assembly angular momentum is conserved to 0.0017 rad/s or better. Resume
  from a coasting pair settles like `rigid`.
- Joint safety: supported for `splitW` on this real content in this rig. It
  is not supported for `comp` from 1:1 up. There is no joint-force readout.
  Avian's `FixedJoint` in this game has no break threshold, so `alive` only
  proves that no release path fired. Joint load is judged by drift and
  relative spin only.

## Limitations (table 4/5)

- The station is test-only authoring (a solid box), not content. No catalog
  station exists. Anchors and beacons are `Static` and have no port.
- One dock geometry: the warship starboard collar, parallel hulls, 5 m face
  gap. No off-clocked or rolled capture.
- `burn-bal` is one side drive (1/11 thrust). A full-thrust aligned burn is
  not possible with the warship's drives on a flank dock.
- `rigid` is close to matched, not exact: warship pair mass 370.3 vs 370.8
  jointed (cause not isolated), and the rigid `HullRadius` is larger, so its
  budget is 0.634-0.674 vs 0.691 (station 0.094 vs 0.099).
- No flight layer: no vector-drive gimbal, manual burn, RCS, autopilot or AI.
  The partner is always neutral. The split parts are fixed at capture and
  recomputed only at re-dock. No damage or section loss while docked.
- The authority toggle is emulated by inserting and removing `DockedShip`.
  The split owner is a test system on the player root. Production ownership
  is still undecided.
- Review (read-only agent): the wrench math, the frames and the parallel-axis
  sum are correct. The split sums every PD output, but
  `sync_controller_section_forces` skips `SectionInactiveMarker` computers.
  They differ only if a computer is disabled mid-run, which no row does.
- 60 Hz updates against the 64 Hz default fixed step. Headless ECS only. No
  `nova-probe`, `nova-bench` or rendered frames. This does not show that
  docked gameplay is safe for a player.
