# Notes

Items 1-3 landed. Items 4 (torpedoes intercepting torpedoes) and 5 (reactive
dodging) deliberately not built - see the evaluation at the end.

## 1. Terminal weave

A corkscrew laid over the guidance command: `TorpedoWeave` carries a tilt
direction that spins about the command at `weave_rate` and tilts it by
`weave_angle`. Perturbation, not replacement - at zero angle the result IS the
guidance solution, so every property the PN law has survives.

The tilt direction is CARRIED as state and advanced incrementally rather than
rebuilt from a basis each step. `any_orthonormal_pair` flips branch as the
command crosses an axis, which would snap the corkscrew a half turn mid-flight.

Faded to nothing between three blast radii and the proximity fuze, and gated on
`TorpedoArming` so the sideways launch turn is left alone.

### What the measurements actually said

The two headline knobs turned out to do quite different jobs, which was not the
prior expectation:

- **`weave_angle` is the balance knob.** Rounds per intercept track it and are
  all but blind to the rate. Breaking a straight-line lead extrapolation does
  not need a FAST turn, it needs a sustained one.
- **`weave_rate` is the look knob.** At a fixed 0.44 rad the intercept cost is
  ~1245 rounds at every rate from 0.7 to 2.2 rad/s (400 u envelope), while the
  visible swing goes 23.6 u -> 6.1 u across that range.

So the rate was chosen for the picture (1.4 rad/s, ~11 u swing) and the angle
for the exchange (0.44 rad).

### The trap the pure-math rigs walked into

The first tuning pass scored the weave with a guidance-only sim whose attitude
model was a rate limiter. That model tracks the commanded cone almost perfectly;
the real body does not, because the PD attitude controller, the thrust law and
the linear drag each attenuate it. The first shipped values (2.2 rad/s) produced
a weave the sim scored at 4x cost and which was INVISIBLE on screen.

Fixed by adding `the_weave_puts_a_visible_bend_in_the_real_flight_path`, which
runs the real stack (avian, `PDControllerPlugin`, `SpaceshipSectionPlugin`)
against a stationary target at 300 u and measures the flown path: 10.6 u of
lateral swing weaving against 0.0 u straight. That test is the one to trust
when retuning; the `point_defense_cost_tests` module is a RATIO, not an
absolute.

Its rig overwrites the fresh torpedo's pose and velocity onto the line at
cruise. A bay fires along its +Y while the torpedo's nose is its -Z, so every
real launch spends ~10 u of lateral excursion turning onto course - on both
arms, monotone, and an order of magnitude wider than the weave under test.

## 2. Torpedo magazines: a rack of six, refilling at 1 per 10 s

First landed as `reload: None` on both shipped bay builders - a HARD magazine.
Reversed by owner direction: ammunition in this game is a RATE LIMIT, not a
budget, and every other weapon pairs a magazine with an unlimited reload. A bay
with no reload was the one exception, and a spent bay is a ship that has stopped
participating in its own scenario.

The siege bay stays unlimited: it is dressing for a looping menu backdrop, not a
combat participant.

### Why NOT the old +1 per 4 s

That rate was set when an intercept cost 116 rounds. Item 1 tripled it to 369
and nobody re-derived the rate. Sustained arithmetic at the shipped numbers:

| quantity | value |
|---|---|
| PDC magazine / fire rate / reload | 500 rounds, 100/s, 3.0 s discrete |
| duty cycle | 5.0 s firing + 3.0 s reloading = 8.0 s |
| sustained supply, per mount | 62.5 rounds/s |
| intercept cost, weaving torpedo (measured) | 369 rounds |
| **intercepts one mount sustains** | **0.169/s** (one per 5.9 s) |
| bay regen at the OLD 4 s | 0.25/s per bay |

So one bay at the old rate out-supplied one mount by 48%, and the attacker won
by WAITING - the precise failure this bay's design exists to prevent. Break-even
is a 5.9 s regen period.

### The rate, and what it protects

`+1 per 10 s` (0.1/s per bay). Two bays put up 0.20/s against the 0.34/s two
mounts answer - 59% - and 118% of what one mount answers. So saturation still
beats point defense and patience does not, which is the whole point of the
weave: an attacker must out-CARRY the defender, not outlast it.

10 s is also `AI_TORPEDO_COOLDOWN_SECS`. An AI bay already spaces its launches
exactly that far apart, so the regen never gates an AI attacker: for every
shipped AI ship, 10 s regen and the old 4 s regen are INDISTINGUISHABLE, and the
campaign's torpedo pressure is back to what it was before the magazine was made
hard. The six-round rack is a PLAYER burst allowance; the AI never gets to use
one.

The relation is now a test, not a comment:
`no_torpedo_bay_out_sustains_a_point_defense_mount` (nova_authoring
`base_content::sections`) derives both sides from the authored catalog - the
mount's duty cycle from its magazine, joint-tree fire rate and reload, the bay's
from its regen - and fails when a bay out-supplies the best mount in the
catalog. `every_authored_magazine_refills` pins the rate-limit rule itself.

### The player is not the AI here

The AI intercepts perfectly and has no autonomous-PD gap; the PLAYER has no
autonomous point defense at all and hand-aims every intercept. The rate above is
therefore chosen on the AI-versus-AI arithmetic, which is the STRICTER side:
where the player defends, the AI attacker is paced by its 10 s cadence and not
by this regen, so nothing here made hand-defending harder than it was before the
hard magazine. Where the player ATTACKS, the alpha strike is unchanged and the
sustained rate is 2.5x slower than it was pre-weave.

The player-attacks case is one shipped fight: **ledger ch5, the raid**. It is
the only time the campaign hands the player a hull with bays (a cargo-B: two
pods, two PDCs) and it puts that hull against six two-PDC corvettes - twelve
mounts, 2.0 intercepts/s between them. The player's sustained 0.2/s is nothing
against that, and the twelve-torpedo rack is everything, which is exactly the
shape this rate is chosen to produce. Ch1-ch4 fly a cargo-A, whose pods are
HULL, not bays - the player carries no ordnance in them at all.

### Backdrop, and the live check

The `menu_gauntlet` batteries relaunch every 15-24 s on scripted timers, slower
than the regen, so they now never run dry - the corvette's own hard magazines
(`SetAmmo`, scenario-level) still end the doomed stand on schedule.

That is also what makes the backdrop the cheapest live proof. Run headless on
Xvfb with the bay's ammo logged on each launch (`NOVA_MENU_BACKDROP=menu_gauntlet`,
`RUST_LOG=info,nova_ship=debug`), battery w1 launched three times at exactly
15.0 s apart and read `rounds left 5` on all three: the spent torpedo was back
before the next trigger pull. A hard magazine reads 5, 4, 3 there. The act
otherwise played as authored - each mount picked its own inbound, one torpedo
was shot down, and the corvette fell at ~43 s with no errors.

**Noticed, not fixed, and not caused by this change:** the gauntlet corvette
carries 400 rounds per turret with the reload stripped, and
`CORVETTE_ROUNDS_PER_TURRET` says that is "roughly a TEN-torpedo defense". At
116 rounds an intercept it was 6.9; at the weave's 369 it is 2.2, and the live
run bears that out - the stand fell after two intercepts. The stand still falls
inside a menu visit, which is what the constant was cut to 400 for, so this is a
stale claim rather than a broken scene. The number is a backdrop-pacing decision
and wants a playtest, not a derivation.

## 3. Per-turret point defence

`TurretSectionArc`, solved once from the joint tree. The general question is IK;
every shipped turret is an unlimited traverse hinge carrying a limited elevation
hinge, and that shape has a closed form because traversing does not change
elevation. Trees that do not match get no component and bear anywhere.

Own-hull raycast occlusion was NOT added. `ai_line_of_fire_blocked` deliberately
treats the shooter's own colliders as transparent (the muzzle sits on its hull),
and the depression floor already IS the geometric statement of "my hull is in
the way" for a hull-mounted mount. A ray that had to ignore only the muzzle's
immediate neighbours would be both expensive and fragile.

Dwell: hold until the target dies, leaves the arc, or a rival arrives in less
than half the time to impact. Acquisition shaves ~3 degrees off the arc and
holding does not, so a torpedo drifting across the depression floor is not
picked up and dropped on alternate frames.

## Ordnance durability - still open, but the derivations were wrong

Both prior derivations (~400-500 hp from Starsector, ~2054 hp from Nova's own
spike) assume every point-defence round that is fired LANDS. The weave has
falsified that: measured effective hit rate against a weaving torpedo is ~0.8%
of rounds fired. Scaling either number by that is meaningless.

With the landed 150 u envelope, a weaving torpedo at the shipped 10 hp already
survives 3.7 s of one PDC's continuous fire and dies 39 u from its target.
Nothing in the hundreds is warranted. If playtest wants "survives one mount,
stopped by two", the number is ~15-20, not 400+.

## Not built, and why

- **Item 4 (torpedoes intercepting torpedoes)** - a new targeting mode, and the
  task says to land 1-3 first and see.
- **Item 5 (reactive dodging)** - item 1 does not fall short. At the landed
  envelope the open-loop weave already triples the ammunition an intercept costs
  and carries a torpedo from a kill at 114 u to a kill at 39 u. Closed-loop
  awareness of individual rounds would add real complexity to a mechanism that
  is already delivering. Recommend not building it.
- **The player-side auto-engage toggle** in the definition of done. Player
  turrets have no autonomous point defence at all today - they follow the
  crosshair and the combat lock - so the toggle is the visible half of a feature
  whose other half (player turrets defending themselves) does not exist yet.
  That is its own piece of work, not a switch.
