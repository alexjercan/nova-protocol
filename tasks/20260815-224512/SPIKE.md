# SPIKE: engagement range, ammunition economy, ordnance survivability

Codebase side of the combat-balance spike. Every number below is READ from the
tree at `926f6aa5` or DERIVED from those values by the formulas quoted. No
balance value was changed. Nothing was implemented.

Scale: **1 unit = 10 m**, so **100 u = 1 km**. Every range is given in both.

Owner decision already taken, and this spike is written to it:

> Muzzle speed STAYS at 100 u/s. Range comes down via projectile LIFETIME.

Section 6 shows why that is the correct call in arithmetic, not adjectives.

---

## 0. The one-paragraph answer

`range = muzzle_speed * projectile_lifetime` is the only physical reach a
turret has - there is **no `range` field on a turret at all**. Lifetime is a
pure reach knob with zero damage side effects. But reach is not where fights
happen: **`AI_STANDOFF_RANGE = 250 u` decides that**, and it is not in the
task's list of known knobs. Set lifetime to 2.0 s without moving the standoff
and AI ships will park at 190-310 u firing rounds that die at 200 u.
Separately, the intercept cost of a torpedo is not the 2-3 rounds it takes to
kill it: it is the **133-296 rounds fired during the round's time of flight**,
because the gun keeps firing at a target that is still alive. That product is
the whole ammunition economy, and it falls LINEARLY with engagement range.

---

## 1. Complete inventory of range knobs

### 1a. Physical reach (what a projectile can actually cross)

| Knob | Location | Value | Units | km | Notes |
|---|---|---|---|---|---|
| `muzzle_speed` (PDC kinetic/pierce, better turret) | `base_content/sections/standard.rs:232,362` | 100.0 | u/s | - | LOCKED by owner decision |
| `muzzle_speed` (light turret, enemy craft turrets) | `standard.rs:441`, `ships/shared.rs:312` | 60.0 | u/s | - | scavenger grade |
| `projectile_lifetime` (every shipped turret) | `standard.rs:233,363,442`, `shared.rs:313` | 5.0 | s | - | **THE LEVER** |
| DERIVED turret reach (100 u/s guns) | `muzzle_speed * projectile_lifetime` | 500 | u | 5.00 | no field stores this |
| DERIVED turret reach (60 u/s guns) | same | 300 | u | 3.00 | |
| `TurretSectionConfig::default()` lifetime | `turret_section/config.rs:254` | 5.0 | s | - | bare examples only |
| Camera far plane | bevy default, nothing overrides (`nova_editor/src/scenario.rs:707`) | 1000 | u | 10.0 | rounds past this are not drawn at all |

There is **no turret `range` field**. Searching `crates/nova_ship/src/sections/turret_section/`
for `range` returns only doc prose and unrelated `rand::random_range`. The
"turret range" in the task brief is this derived product.

### 1b. Decision ranges (when a ship chooses to shoot or close)

| Knob | Location | Value | Units | km | What it gates |
|---|---|---|---|---|---|
| `AI_FIRE_RANGE_FACTOR` | `input/ai/guns.rs:19` | 0.9 | - | - | margin below reach |
| DERIVED AI fire gate | `guns.rs` `on_projectile_input` | 450 | u | 4.50 | `muzzle_speed * lifetime * 0.9` |
| `AI_STANDOFF_RANGE` | `input/ai/maneuver.rs:28` | 250 | u | 2.50 | **where fights actually happen** |
| `AI_STANDOFF_BAND` | `maneuver.rs:31` | 60 | u | 0.60 | orbit band half-width (190-310 u) |
| `AI_ENGAGE_RANGE` | `input/ai/behavior.rs:167` | 800 | u | 8.00 | passive -> Engage transition |
| `AI_POINT_DEFENSE_RANGE` | `input/ai/acquisition.rs:233` | 400 | u | 4.00 | inbound torpedo grabs the guns |
| `AI_TARGET_MAX_RANGE` | `acquisition.rs:89` | 2000 | u | 20.0 | acquisition scan |
| `AI_TARGET_HYSTERESIS_DISCOUNT` | `acquisition.rs:93` | 0.8 | - | - | incumbent stickiness |
| `AI_THREAT_AIM_RANGE` | `input/ai/threat.rs:25` | 500 | u | 5.00 | "someone is aiming at me" -> Evade |
| `AI_TORPEDO_MAX_RANGE` | `input/ai/torpedo.rs:22` | 1000 | u | 10.0 | torpedo launch envelope, outer |
| `AI_TORPEDO_MIN_RANGE_BLAST_FACTOR` | `torpedo.rs:28` | 3.0 | x blast_r | - | inner edge = 90 u (std), 135 u (heavy) |
| `AI_AVOID_MARGIN` / `_HYSTERESIS` | `input/ai/passive.rs:46,54` | 20 / 10 | u | - | obstacle avoidance |
| `AI_WAYPOINT_SLACK` | `passive.rs:22` | 25 | u | 0.25 | patrol arrival |
| `engage_range` override | `nova_scenario/objects/spaceship.rs:138` | None -> 800 | u | 8.00 | per-ship authored |
| `pd_range` override | `spaceship.rs:148` | None -> 400 | u | 4.00 | per-ship authored |
| `CORVETTE_ENGAGE_RANGE` (main-menu gauntlet backdrop) | `scenarios/main_menu/gauntlet.rs:45` | 300 | u | 3.00 | only shipped engage override |
| gauntlet backdrop `pd_range` | `gauntlet.rs:213` | 130 | u | 1.30 | only shipped pd override |
| (1600 / 150 at `spaceship.rs:608,609` is a UNIT TEST fixture, not shipped content) | | | | | |

### 1c. Player-side ranges

| Knob | Location | Value | Units | km |
|---|---|---|---|---|
| `TARGETING_MAX_RANGE` | `input/targeting/contacts.rs:17` | 20000 | u | 200 |
| ship / gravity-well lock range | `contacts.rs:138` | 20000 (`TARGETING_MAX_RANGE`) | u | 200 |
| `torpedo_lock_range` | `targeting/state.rs:71` | 2500 | u | 25.0 |
| `signature_range_per_unit` | `state.rs:69` | 30 | u per sig unit | - |
| `unsigned_lock_range` | `state.rs:70` | 5 | u | 0.05 |
| `lock_dwell_reference_range` | `state.rs` default | 2000 | u | 20.0 |
| `range_hysteresis` | `state.rs` default | 1.15 | - | - |

**The player HUD has no notion of weapon range.** Grepping `nova_hud`,
`nova_os_ui` and `nova_ui` for `muzzle_speed`, `projectile_lifetime` or
`effective_range` returns nothing. Whatever lifetime becomes, the player gets
no readout of it.

### 1d. Ordnance (there is no "fuel" - endurance IS lifetime)

| Knob | Location | Standard torpedo | Siege torpedo | Units |
|---|---|---|---|---|
| `max_speed` | `standard.rs:510,577` | 35.0 | 70.0 | u/s |
| `projectile_lifetime` | `standard.rs:506,571` | 100.0 | 60.0 | s |
| DERIVED reach (max_speed x lifetime) | - | 3500 u / **35 km** | 4200 u / **42 km** | - |
| `linear_damping` | `standard.rs:511,578` | 0.8 | 0.4 | - |
| `spawner_speed` (launch push) | `standard.rs:505,568` | 1.0 | 2.0 | u/s |
| `nav_constant` (PN gain) | `standard.rs:509,574` | 3.0 | 4.0 | - |
| `blast_radius` | `standard.rs:512,584` | 30.0 | 45.0 | u |
| fuze distance (`blast_radius * 0.5`) | `torpedo_section/projectile.rs` | 15.0 | 22.5 | u |
| `blast_damage` | `standard.rs:520,585` | 750.0 | 2000.0 | hp at centre |
| `projectile_health` | `standard.rs:530,593` | **10.0** | 5000.0 | hp per body section |
| `arm_time` / `arm_distance` | `standard.rs:507,508` | 0.5 / 5.0 | 0.5 / 5.0 | s / u |
| `ammo_capacity` | `standard.rs:533,594` | 6 | None (unlimited) | rounds |
| `reload` | `standard.rs:537` | 4.0 s, +1/cycle, continuous | None | - |

Note: `linear_damping` is non-zero, so a torpedo does not hold `max_speed`
ballistically. `thrust_headroom` (`projectile.rs`) tapers thrust over the last
5 u/s below `max_speed`, so cruise sits just under it. The derived reach above
is an upper bound and is far past any arena; **lifetime is not what limits a
torpedo, guidance and the target dying is**.

### 1e. Damage and rate

| Knob | Location | Value |
|---|---|---|
| `BETTER_TURRET_BULLET_DAMAGE` (kinetic PDC) | `standard.rs:42` | 4.0 |
| `PIERCE_PDC_BULLET_DAMAGE` | `standard.rs:52` | 2.0 (`= 4.0 * 0.5`) |
| light turret `bullet_damage` | `standard.rs:446` | `representative_kinetic_damage(0.05, 60)` = **3.825** |
| `TurretSectionConfig::default()` `bullet_damage` | `config.rs:256` | `representative_kinetic_damage(0.1, 100)` = **20.25** |
| PDC / better turret `fire_rate` (per muzzle) | `standard.rs:223,358` | 100.0 rounds/s |
| light turret `fire_rate` | `standard.rs:434` | 25.0 rounds/s |
| `MAX_SHOTS_PER_TICK` | `turret_section/firing.rs:134` | 8 (caps 512 rounds/s) |
| `AI_BURST_FIRE_SECS` / `AI_BURST_HOLD_SECS` | `guns.rs:21,22` | 1.5 / 0.8 (bypassed for point defense) |
| `AI_FIRE_ALIGNMENT` | `guns.rs:14` | 0.95 |
| joint traverse speed | `config.rs:28` | PI rad/s = 180 deg/s |
| `REFERENCE_CLOSING_SPEED` | `nova_gameplay/src/damage.rs:153` | 100.0 u/s |
| kinetic clamp | `damage.rs:159,165` | [0.25, 2.0] |
| pierce power clamp | `damage.rs:169,174` | [0.5, 3.0] |
| `PIERCE_BASE_POWER` / `MAX_PIERCE_LAYERS` | `damage.rs:185,193` | 300.0 / 6 |

---

## 2. Lifetime -> reach, at the locked muzzle speed of 100 u/s

Formula: `reach = muzzle_speed * projectile_lifetime`.
AI fire gate: `gate = reach * AI_FIRE_RANGE_FACTOR` (0.9).

| lifetime (s) | reach (u) | reach (km) | AI fire gate (u) | gate (km) | in owner's 1-2 km band? |
|---|---|---|---|---|---|
| 1.00 | 100 | 1.00 | 90 | 0.90 | at the floor |
| 1.25 | 125 | 1.25 | 113 | 1.13 | yes |
| 1.50 | 150 | 1.50 | 135 | 1.35 | yes |
| 1.75 | 175 | 1.75 | 158 | 1.58 | yes |
| **2.00** | **200** | **2.00** | **180** | **1.80** | **yes, top of band** |
| 2.25 | 225 | 2.25 | 203 | 2.03 | just over |
| 2.50 | 250 | 2.50 | 225 | 2.25 | over |
| 3.00 | 300 | 3.00 | 270 | 2.70 | over |
| **5.00 (CURRENT)** | **500** | **5.00** | **450** | **4.50** | **2.5x over** |

The 60 u/s guns (light turret, enemy craft turrets) share the same
`projectile_lifetime` field but a different muzzle speed, so the SAME lifetime
gives them 60% of the reach:

| lifetime (s) | reach at 60 u/s (u) | km | AI fire gate (u) |
|---|---|---|---|
| 2.00 | 120 | 1.20 | 108 |
| 2.50 | 150 | 1.50 | 135 |
| 3.00 | 180 | 1.80 | 162 |
| 3.33 | 200 | 2.00 | 180 |
| 5.00 (current) | 300 | 3.00 | 270 |

**Consequence:** a single global lifetime cannot put both gun grades in the
same band. At lifetime 2.0 the scavenger gun reaches 1.2 km and the player's
PDC 2.0 km. That is a defensible grade difference (it is already 5 km vs 3 km),
but it should be a decision, not a side effect. Authoring lifetime per section
is already supported - it is a plain `TurretSectionConfig` field.

---

## 3. The AI knobs must move WITH the lifetime

Physical reach is set by lifetime. WHERE a fight happens is set by three other
numbers, and every one of them is currently tuned against the 450 u fire gate.

| Knob | Now (u) | Meaning | Required relation | At lifetime 2.0 (gate 180 u) |
|---|---|---|---|---|
| DERIVED fire gate | 450 | rounds are worth firing | `= 0.9 * 100 * L` | 180 |
| `AI_STANDOFF_RANGE` | 250 | **the orbit radius a fight settles at** | must be WELL inside the gate | ~100-120 |
| `AI_STANDOFF_BAND` | 60 | half-width of that orbit band | `standoff + band < gate` | ~30-40 |
| `AI_POINT_DEFENSE_RANGE` | 400 | inbound torpedo grabs the guns | `<= gate` | 150-180 |
| `AI_ENGAGE_RANGE` | 800 | passive ship commits to a fight | `> gate`, sets approach length | see below |
| `AI_THREAT_AIM_RANGE` | 500 | "being aimed at" -> Evade | should track the gate | ~200 |
| `AI_TORPEDO_MAX_RANGE` | 1000 | torpedo launch envelope | independent of guns | unchanged |

**This is the pairing most likely to be got wrong.** Today `standoff + band =
310 u` sits inside the `450 u` gate with 140 u to spare. Drop lifetime to 2.0
and leave the standoff alone and the band becomes 190-310 u against a 180 u
gate: **an AI ship would orbit permanently outside its own weapon range and
never fire a shot.** The failure is silent - no error, no warning, the guns
simply never trigger. Ratio to preserve:

```
(AI_STANDOFF_RANGE + AI_STANDOFF_BAND) / fire_gate = 310 / 450 = 0.69
```

At a 180 u gate that ratio gives `standoff + band = 124`, e.g. standoff 95,
band 30. Keeping the band proportional matters too: a 60 u band around a 100 u
standoff is a 40-160 u orbit, which is a wildly different fight shape from
190-310 u.

**`AI_ENGAGE_RANGE = 800` is the pacing knob, not a reach knob.** It decides
how long a ship flies before it can shoot. At `AI_MAX_CHASE_SPEED = 20 u/s`,
closing from 800 u to a 180 u gate is `620 / 20 = 31 s` of approach with no
fire exchanged. Today it is `800 - 450 = 350 u / 20 = 17.5 s`. If the fight is
meant to open at the same tempo, engage_range should come down roughly in
proportion (~400-500 u), or the approach doubles.

Shipped content that is tuned against these distances and will feel the change:

- `ledger_ch2` wave spawns at 500 u and 800 u from the player
  (`nova_assets/tests/ledger_ch2_encounter.rs:42,44`) - both far outside a
  180 u gate. Those are absolute pins, so the tests keep passing; the SCENE
  changes (long silent approach).
- `lifeline` raiders at 700 u, `final_tally` hostiles at 700 u.
- The gauntlet already authors `engage_range: 300` and `pd_range: 130`
  (`gauntlet.rs:206,213`) - the only shipped content already tuned close-in,
  and the only content that would need almost no change.

**CI coupling to check before landing:** `nova_authoring/src/balance.rs:173`
derives every ship's threat envelope as
`EFFECTIVE_RANGE_MARGIN * muzzle_speed * projectile_lifetime`, and
`balance_audit_gate.rs` fails on both active findings AND **stale acks**.
Shrinking lifetime shrinks every envelope, so findings can disappear and leave
an ack unmatched. The single shipped ack
(`crates/nova_authoring/balance_acks.ron`, ledger_ch4 "auditor") is driven by
the 1000 u TORPEDO envelope, not by turret reach, so it survives - but the
gate must be re-run, because a lifetime change is a balance-audit change.

---

## 4. Time-of-flight arithmetic

Against a STATIONARY target, `t = d / muzzle_speed`:

| distance (u) | km | time of flight (s) at 100 u/s |
|---|---|---|
| 100 | 1.00 | 1.00 |
| 150 | 1.50 | 1.50 |
| 180 | 1.80 | 1.80 |
| 200 | 2.00 | 2.00 |
| 400 | 4.00 | 4.00 |
| 500 | 5.00 | 5.00 |

Against a CLOSING target the gap shuts at the closing speed, so
`t = d / (muzzle_speed + v_target)`:

| launch range (u) | vs standard torpedo (35 u/s), closing 135 | intercept happens at (u) | vs siege torpedo (70 u/s), closing 170 | intercept at (u) |
|---|---|---|---|---|
| 100 | 0.74 s | 74 | 0.59 s | 59 |
| 150 | 1.11 s | 111 | 0.88 s | 88 |
| 200 | 1.48 s | 148 | 1.18 s | 118 |
| 300 | 2.22 s | 222 | 1.76 s | 176 |
| **400 (current pd_range)** | **2.96 s** | **296** | **2.35 s** | **235** |

### Torpedo exposure to point defence

Exposure window = time from entering `pd_range` to the proximity fuze at
`blast_radius * 0.5`:

```
window = (pd_range - blast_radius * 0.5) / torpedo_speed
```

| pd_range (u) | std torpedo window (s) | std ToF (s) | ratio | siege window (s) | siege ToF (s) | ratio |
|---|---|---|---|---|---|---|
| 100 | 2.43 | 0.74 | 3.28 | 1.11 | 0.59 | 1.88 |
| 150 | 3.86 | 1.11 | 3.47 | 1.82 | 0.88 | 2.06 |
| 180 | 4.71 | 1.33 | 3.54 | 2.25 | 1.06 | 2.12 |
| 200 | 5.29 | 1.48 | 3.57 | 2.54 | 1.18 | 2.16 |
| **400 (now)** | **11.00** | **2.96** | **3.71** | **5.39** | **2.35** | **2.29** |

**The ratio is almost invariant in range.** As `pd_range` grows large,

```
window / ToF -> (muzzle_speed + v_torpedo) / v_torpedo
             = 135/35 = 3.86  (standard)
             = 170/70 = 2.43  (siege)
```

**Therefore: shortening engagement range does NOT make torpedoes survive.** It
shortens the window and the flight time by the same factor. A torpedo entering
at 100 u is exposed for 3.3 flight-times; at 400 u it is exposed for 3.7. Same
fight, smaller stage. Range is an AMMUNITION lever and a READABILITY lever, not
an ordnance-survivability lever. That is the most load-bearing result in this
document after Section 6.

---

## 5. The cost of an intercept, today

### 5a. Rounds needed to kill a torpedo

An ordnance body carries TWO 10 hp sections - `torpedo_controller` and
`torpedo_thruster` (`torpedo_section/bay.rs:309,335`, both
`health: config.projectile_health`). `on_torpedo_body_destroyed`
(`bay.rs:16`) marks the WHOLE torpedo dead when EITHER section reaches zero.
**So the kill threshold is 10 hp on one section, not 20.**

Kinetic PDC: `damage_per_hit = 4.0 * clamp(closing / 100, 0.25, 2.0)`.

| closing speed (u/s) | scenario | damage/hit | rounds to kill 10 hp | trigger time after first arrival |
|---|---|---|---|---|
| 75 | stern chase, target fleeing 25 u/s | 3.00 | 4 | 0.030 s |
| 100 | station-keeping (the REFERENCE) | 4.00 | 3 | 0.020 s |
| 120 | target charging 20 u/s | 4.80 | 3 | 0.020 s |
| 125 | threshold where 2 rounds suffice | 5.00 | 2 | 0.010 s |
| **135** | **standard torpedo head-on** | **5.40** | **2** | **0.010 s** |
| **170** | **siege torpedo head-on** | **6.80** | **2** | **0.010 s** |
| >=200 | clamped at 2.0x | 8.00 | 2 | 0.010 s |

Worked example at closing 135 (the realistic intercept):
round 1 bites 5.40, section drops 10.0 -> 4.60, round is expended
(`4.0 * 1.35 <= 10.0`, so `pierce_remainder` returns `None`). Round 2 bites
5.40 > 4.60, so it kills and carries on with
`4.0 - 4.60/1.35 = 0.59` damage left.

Pierce PDC: `damage_per_hit = 2.0` FLAT - `hit_bite` does not apply the speed
curve to Pierce (`damage.rs:283`). So **5 rounds, at any closing speed**.
Speed buys pierce POWER instead: crossing a 10 hp layer costs
`10 / clamp(closing/100, 0.5, 3.0)` = **7.41 power at closing 135**, out of
`PIERCE_BASE_POWER = 300`. Power is never the binding constraint against
ordnance; `MAX_PIERCE_LAYERS = 6` is. So one pierce round can rake **6 torpedo
sections = 3 whole torpedoes** if they line up, dealing 2.0 to each. Five
aligned pierce rounds therefore kill three torpedoes for the price of one
torpedo's worth of fire. That is the pierce round's real anti-ordnance
identity, and it is currently invisible because nothing in a salvo lines up.

Light turret (60 u/s, 3.825 authored) at closing 60: `3.825 * 0.6 = 2.295`
per hit, so **5 rounds** to kill an ordnance section, at 25 rounds/s.

### 5b. Rounds actually FIRED per intercept - the number that matters

The AI point-defence path fires CONTINUOUSLY: `on_projectile_input`
(`guns.rs`) sets `defending = pd_target.is_some()` and then
`firing_allowed = defending || ...`, which **bypasses the burst cadence
entirely**. The target does not die until the first rounds arrive, which is one
full time-of-flight later. So:

```
rounds_wasted_per_intercept = fire_rate * pd_range / (muzzle_speed + v_torpedo)
```

| pd_range (u) | vs std torpedo | vs siege torpedo | as fraction of the 500-round magazine |
|---|---|---|---|
| 100 | 74 | 59 | 15% |
| 150 | 111 | 88 | 22% |
| 180 | 133 | 106 | 27% |
| 200 | 148 | 118 | 30% |
| 300 | 222 | 176 | 44% |
| **400 (now)** | **296** | **235** | **59%** |

**One torpedo intercept at the shipped 400 u pd_range burns ~296 rounds to
land a 2-round kill: 99.3% waste.** With `infinite_ammo` off that is 59% of a
magazine per torpedo, and the standard bay carries 6. This is precisely the
"point defence has no cost" problem, and its cause is the flight time, not the
damage numbers.

Intercepts per 500-round magazine (standard torpedo):

| pd_range (u) | intercepts per magazine |
|---|---|
| 400 (now) | 1.69 |
| 200 | 3.38 |
| 180 | 3.75 |
| 100 | 6.75 |

**Answer to "what must a magazine hold for point defence to be a decision":**
against the 6-torpedo standard bay, a full salvo costs `6 * 133 = ~800 rounds`
at a 180 u pd_range. A 500-round magazine covers 3.75 of the 6. So the CURRENT
magazine already makes a full salvo unanswerable **once the range comes down
and infinite ammo is off**. No magazine change is required to create the
decision; the range change creates it. If anything the knob to watch is the
reload (3.0 s to refill 500 on empty = 62.5 rounds/s sustained, a 62.5% duty
cycle), which is generous relative to a 4.7 s exposure window.

### 5c. How many torpedoes can one mount engage?

`AIPointDefenseTarget` is a single `Option<Entity>`
(`acquisition.rs`), so a ship engages **one torpedo at a time no matter how
many turrets it has** - every turret slews to the same target. Kills are
therefore sequential, and each kill costs one time-of-flight, during which the
remaining salvo closes. Range multiplies by
`r = muzzle_speed / (muzzle_speed + v_torpedo)` per kill:

```
kills = ln(fuze_range / pd_range) / ln(r),  r = 100/135 = 0.741 (std), 100/170 = 0.588 (siege)
```

| pd_range (u) | standard (r=0.741) | siege (r=0.588) |
|---|---|---|
| 100 | ~6 kills | ~3 kills |
| 200 | ~9 kills | ~4 kills |
| 400 (now) | ~11 kills | ~5 kills |

With unlimited ammo a single mount stops 11 standard torpedoes. Range barely
helps (it is a logarithm). **Ammunition is the only thing that caps this**, and
it caps it hard: 1.69 intercepts per magazine at 400 u.

### 5d. Ordnance toughness is the WRONG lever - the arithmetic

For a torpedo to reach its fuze, it must outlast the fire it takes after the
first round arrives:

```
hp_needed = fire_rate * damage_per_hit * (exposure_window - time_of_flight)
```

| pd_range (u) | torpedo | applied dps | seconds under fire | hp needed to survive |
|---|---|---|---|---|
| 100 | standard | 540 | 1.69 | **911** |
| 200 | standard | 540 | 3.80 | **2054** |
| 400 | standard | 540 | 8.04 | **4340** |
| 200 | siege | 680 | 1.36 | 924 |

Ordnance is at **10 hp**. To survive one PDC mount it would need 900-4300. The
siege torpedo's 5000 hp is exactly this arithmetic solved the other way, and
its authoring comment says so. **No plausible ordnance hp saves a standard
torpedo from a 100 rounds/s gun that cannot miss.** The levers that actually
bite, in order of effect:

1. **fire_rate** - it multiplies both the waste and the applied dps. At 10
   rounds/s the hp needed at 200 u falls from 2054 to 206.
2. **ammunition** - already computed above; the binding constraint today.
3. **making the gun miss** - there is no dispersion model at all. The lead
   solve (`aim.rs lead_intercept_point`) is exact, the turret traverses at
   180 deg/s, and point defence is exempt from the line-of-fire ray. A PDC
   round in Nova cannot miss.
4. **salvo size** - saturating a one-target-at-a-time defence.

Ordnance hp is a rounding error next to any of these.

---

## 6. The `REFERENCE_CLOSING_SPEED` trap

`REFERENCE_CLOSING_SPEED = 100.0` (`nova_gameplay/src/damage.rs:153`) is a
plain constant. **It is not derived from `muzzle_speed`; nothing recomputes
it.** Both speed curves read exactly 1.0 there:

```
kinetic_damage_multiplier(c) = clamp(c / 100, 0.25, 2.0)   -> scales DAMAGE
pierce_power_multiplier(c)   = clamp(c / 100, 0.5,  3.0)   -> scales PENETRATION
```

The invariant it encodes: a round fired from a station-keeping ship at a
station-keeping target closes at exactly its muzzle speed, so the multiplier is
1.0 and every authored `bullet_damage` in the catalog means what it says.

### Which levers move the reference, and which do not

| Lever | Moves the anchor? | Effect on damage |
|---|---|---|
| **`projectile_lifetime`** | **NO** | **none at all - reach only** |
| `AI_STANDOFF_RANGE`, `AI_STANDOFF_BAND` | NO | none (decides where, not how hard) |
| `AI_ENGAGE_RANGE`, `pd_range`, `AI_FIRE_RANGE_FACTOR` | NO | none |
| `AI_TORPEDO_MAX_RANGE` | NO | none |
| `fire_rate`, `ammo_capacity`, `reload` | NO | changes dps, not per-hit |
| `bullet_damage` | NO | linear, no cross-talk |
| **`muzzle_speed`** | **YES** | **breaks the 1.0 invariant globally** |
| ship speeds (`speed_cap` 25, `AI_MAX_CHASE_SPEED` 20, `AI_ORBIT_SPEED` 8) | no, but they move the ARGUMENT | +/-25% per-hit swing |
| torpedo `max_speed` (35 / 70) | no, but it moves the ARGUMENT | PDC hits a siege torpedo 26% harder than a standard one |

If `muzzle_speed` dropped from 100 to 40 to get a 200 u reach at the current
5.0 s lifetime:

- station-keeping kinetic multiplier becomes `40/100 = 0.4`, so every kinetic
  round in the game deals **40% of its authored damage**. The 4.0 PDC becomes
  1.6, the 400 burst dps becomes 160.
- pierce power becomes `clamp(0.4, 0.5, 3.0) = 0.5`, hitting the FLOOR. Every
  pierce round permanently rakes at half rated depth and the whole upper half
  of the pierce curve becomes unreachable in normal play.
- rounds to kill a 10 hp torpedo section go from 2 to 7.
- "fixing" it by also moving the constant to 40 rebalances the OTHER weapons -
  the 60 u/s light turret would jump from 0.6x to 1.5x, a 2.5x buff.

Lifetime does none of this. **`projectile_lifetime` is the only range lever in
the tree with a provably empty blast radius on the damage model.** The owner's
decision is correct and this is the arithmetic behind it.

### The pre-existing violation worth knowing about

The light turret and every enemy craft turret already fire at 60 u/s. Against a
station-keeping target their multiplier is `0.6`, so the authored 3.825 lands
as **2.295**. The anchor is only true for the 100 u/s guns. This is not caused
by anything in this task, but any statement of the form "authored damage IS
applied damage" (which `balance.rs:107` makes) is already only true for half
the catalog.

---

## 7. Relative motion: reach is not a constant

Rounds inherit the full muzzle-point velocity of the firing ship
(`firing.rs`: `linear_velocity = muzzle_direction * muzzle_speed + inertia_vel`,
where `inertia_vel` includes the ship's linear velocity AND the tangential
swing of an off-centre muzzle on a rotating hull). `TempEntity` is a wall-clock
TTL. So:

```
reach_in_shooter_frame = muzzle_speed * lifetime                (fixed)
reach_against_a_target = closing_speed * lifetime               (varies)
closing_speed          = (v_round - v_target) . round_direction
```

The gap between round and target closes at exactly the same `closing_speed`
that `damage.rs` feeds the damage curve. **Reach and per-hit damage move
together, from one quantity.**

At **lifetime 2.0 s** (nominal 200 u / 2.00 km):

| situation | closing (u/s) | effective reach (u) | km | vs nominal |
|---|---|---|---|---|
| target fleeing at 25 u/s, shooter stationary | 75 | 150 | 1.50 | **0.75x** |
| target fleeing at 20 | 80 | 160 | 1.60 | 0.80x |
| station-keeping duel (the REFERENCE) | 100 | 200 | 2.00 | 1.00x |
| pure stern chase, both at 20 u/s | 100 | 200 | 2.00 | 1.00x |
| target charging at 20 | 120 | 240 | 2.40 | 1.20x |
| head-on, both at 20 | 140 | 280 | 2.80 | 1.40x |
| head-on, both at 25 (player cap) | 150 | 300 | 3.00 | 1.50x |
| vs standard torpedo head-on | 135 | 270 | 2.70 | 1.35x |
| vs siege torpedo head-on | 170 | 340 | 3.40 | 1.70x |

**Full swing across realistic speeds: 0.75x to 1.7x nominal**, i.e. 1.5 km to
3.4 km at a nominal 2 km. Note the pure stern chase (shooter and target at the
same speed) reads exactly 1.00x - the shooter's inherited velocity cancels the
target's. **A chase does not need more lifetime; a chase against something
FASTER than the shooter does.**

**The AI fire gate does not model any of this.** `on_projectile_input` uses
`config.muzzle_speed * config.projectile_lifetime * 0.9`, a shooter-frame
constant. Against a target fleeing at 25 u/s the true reach is `0.75 * 200 =
150 u` while the gate says 180 u, so the AI fires 20% past what its rounds can
reach. Against a charger the true reach is 240 u and the gate holds fire at
180 u, wasting 25% of the envelope. `AI_FIRE_RANGE_FACTOR = 0.9` is the margin
that is supposed to absorb this, and 0.9 is not enough for the fleeing case
(it needs ~0.75 to be strictly safe, or the gate needs to use closing speed).

---

## 8. Does a round vanishing mid-flight read badly?

Likely NOT noticeable, on three counts. Reporting only; nothing implemented.

1. **Angular size.** The default round is `Cuboid::new(0.05, 0.05, 0.3)`
   (`turret_section/render.rs:120`), i.e. 0.5 m x 0.5 m x 3 m at 1 u = 10 m.
   Bevy's default vertical FOV is 45 deg and nothing overrides it. At 1080p
   that is ~24 px/deg. A 3 m round at 200 u (2 km) subtends 0.086 deg =
   **~2 px**. At the current 500 u it is already sub-pixel. Rounds expire where
   they are already a flicker.
2. **Camera far plane.** 1000 u (bevy default, explicitly noted as unoverridden
   in `nova_editor/src/scenario.rs:707`). Rounds are clipped at 10 km anyway,
   so the current 5 km expiry is already the first of two invisible walls.
3. **Where the eye is.** Fights settle at the standoff (250 u now, ~100 u
   after a matched retune). Expiry at 200 u happens beyond the target, off the
   axis the player is watching.

The case where it WOULD show: firing into empty space with nothing at the aim
point, with the camera looking down the barrel. The stream would end at a
visible plane 2 km out. Two cheap mitigations, if wanted later, in
increasing cost:

- fade the round's emissive over the last ~15% of its life (needs a per-round
  material, which conflicts with the shared-handle optimisation in
  `DefaultProjectileRender` - see Section 10);
- scale the round down over the last 0.2 s (a `Transform` tween, no material
  churn, no asset allocation).

There is **no bullet trail today.** `render.rs:2` and `mod.rs:222` both mention
"projectile-trail effects" in doc comments, but grepping the turret section for
`trail` finds no implementation - only the muzzle flash exists. So there is no
trail to shorten.

---

## 9. The ammunition mechanism

### How it works now

| Layer | File | Behaviour |
|---|---|---|
| Component | `nova_ship/src/sections/ammo.rs:40` | `SectionAmmo { rounds, capacity }`. **Absence of the component = unlimited.** |
| Component | `ammo.rs:117` | `SectionReload` seeded from `SectionReloadConfig`; rides ON the magazine, so no magazine = no reload |
| Authoring | `turret_section/config.rs:179`, `torpedo_section/mod.rs` | `ammo_capacity: Option<u32>`, `reload: Option<SectionReloadConfig>` |
| Build | `turret_section/setup.rs:146` | `None` -> attach no `SectionAmmo` -> unlimited |
| Spend | `turret_section/firing.rs:225,275` | gate before the muzzle loop plus `try_consume()` per round |
| Refill | `ammo.rs:196` | `tick_section_reload`, add-only, no ordering needed against firing |

Every shipped weapon ALREADY authors a finite magazine:

| Weapon | capacity | fire rate | seconds of fire | reload | style | sustained rate |
|---|---|---|---|---|---|---|
| PDC kinetic / pierce / better turret | 500 | 100/s | 5.0 s | 3.0 s | discrete on empty | 62.5/s (62.5% duty) |
| light turret / enemy turrets | 150 | 25/s | 6.0 s | 2.5 s | discrete on empty | 17.6/s (70.6% duty) |
| torpedo bay | 6 | 1/s | - | 4.0 s | continuous +1 | 0.25/s |
| siege torpedo bay | None | 1/s | - | None | **unlimited** | - |

### Exactly where infinite ammo is granted

**One place.** `crates/nova_scenario/src/objects/spaceship.rs:350-352`:

```rust
let infinite_ammo =
    matches!(controller_config, SpaceshipController::Player(config) if config.infinite_ammo);
```

then at lines 410 and 429 it overwrites the resolved prototype:

```rust
if infinite_ammo { turret_config.ammo_capacity = None; }
if infinite_ammo { torpedo_config.ammo_capacity = None; }
```

The flag is `PlayerControllerConfig::infinite_ammo` (`spaceship.rs:81`), a
plain `bool` with no default attribute, **player-scoped only** - enemies are
never flagged. Note it strips `ammo_capacity` only; `reload` is left set, and
`setup.rs:150` then attaches no `SectionReload` because reload rides on the
magazine. So the override is clean, single-point, and total.

**Who currently sets it true:**

| Site | Kind |
|---|---|
| `examples/screenshots/screenshot_nova_os.rs:177`, `screenshot_flight.rs:501`, `screenshot_combat.rs:621,694` | example |
| `examples/sections/turret_section.rs:311`, `hull_section.rs:365`, `torpedo_section.rs:237` | example |
| `examples/systems/player_path.rs:230`, `examples/stress/many_projectiles.rs:276`, `examples/ui/hud_range.rs:216` | example |
| `crates/nova_scenario/src/loader/mod.rs:554` | serde round-trip TEST fixture only |
| `crates/nova_authoring/tests/ledger_ch5_raid.rs:346` | asserts the ledger ch5 "victory lap" ships it TRUE |

**And - the real blast radius - SEVEN shipped webmod scenarios author it true**
in hand-written RON. `webmods/**` is NOT generated (there is no Rust builder;
`lint_walk.rs:145` only walks it to LINT it), so these are edits to
hand-authored files:

| File | Line |
|---|---|
| `webmods/the-ledger/ledger_ch1.content.ron` | 187 |
| `webmods/the-ledger/ledger_ch2.content.ron` | 167 |
| `webmods/the-ledger/ledger_ch2b.content.ron` | 161 |
| `webmods/the-ledger/ledger_ch3.content.ron` | 232 |
| `webmods/the-ledger/ledger_ch4.content.ron` | 147 |
| `webmods/the-ledger/ledger_ch5_the_raid.content.ron` | 189 (plus a comment at 16) |
| `webmods/gauntlet/gauntlet.content.ron` | 136 |

**That is the WHOLE ledger campaign plus the gauntlet.** Every one of those
scenarios is currently played with unlimited player ammunition, so demoting
infinite ammo to a debug cheat means those seven scenarios get finite ammo for
the first time and each needs a playtest. This is the single biggest hidden
cost in the task.

**Who sets it false (the base campaign, all generated from Rust):**
`shakedown/mod.rs:515` -> `assets/base/scenarios/shakedown_run.content.ron:49`,
`broadside.rs:157`, `lifeline.rs:184`, `final_tally.rs:158`,
`asteroid_field.rs:96`, plus `nova_editor/src/scenario.rs:437` and
`assets/mods/example/example.content.ron:333`. A shakedown walk test even PINS
it off (`shakedown/tests/walk.rs:875`).

So the split is clean and worth stating plainly: **base campaign = finite
already; every webmod = infinite.**

### What would have to change to demote it to a debug-only cheat

Mechanism only - not implemented.

1. **Seven shipped scenarios must be re-authored and re-playtested** (the table
   above), and `crates/nova_authoring/tests/ledger_ch5_raid.rs:346`
   (`assert!(cfg.infinite_ammo, "infinite ammo for the victory lap")`) inverts
   or goes. This is the work; the code change is trivial next to it.
2. **Gate the field.** The cleanest seam is the SAME single site
   (`spaceship.rs:350`): make the grant conditional on a debug signal in
   addition to the flag. Candidates already in the tree: the `debug` cargo
   feature (`cargo run --features debug`, used by the probe), or a resource the
   debug plugin inserts. Examples run from the workspace and can enable the
   feature; shipped scenarios cannot.
3. **Serde policy.** `infinite_ammo` currently has no
   `skip_serializing_if`, so it round-trips in every scenario RON. Demoting it
   should mean authored `true` in a non-debug build is either ignored (with a
   `warn!`) or rejected by `content lint`. Ignoring is safer - existing mod
   bundles would still load.
4. **No engine work is needed.** Finite ammo is already the code's default
   path; every shipped prototype already authors a magazine and a reload; the
   HUD ammo readout already handles both states
   (`nova_hud/src/ammo_readout.rs` explicitly skips magazine-less weapons, and
   `nova_os_ui/src/ship/sections.rs:446` prints "has unlimited ammo"). **The
   code change is one condition at `spaceship.rs:350`.** The cost is entirely
   in content: seven scenario RONs and their playtests.

---

## 10. Round rendering

### Where the mesh comes from today

| Step | File | What happens |
|---|---|---|
| Spawn | `turret_section/firing.rs:346` | round gets `BulletProjectileRenderMesh(config.projectile_render_mesh.clone())` |
| Observer | `turret_section/render.rs:128` `insert_projectile_render`, on `Add, TurretBulletProjectileMarker` | branches on that `Option<AssetRef<WorldAsset>>` |
| `Some(asset_ref)` | `render.rs:147` | child with `WorldAssetRoot(asset_ref.resolve(&asset_server))` - a GLB scene |
| `None` (**every shipped turret**) | `render.rs:154` | child with the shared `DefaultProjectileRender` handles |
| The default | `render.rs:116-126`, `FromWorld` | `Cuboid::new(0.05, 0.05, 0.3)` + `StandardMaterial` `srgb(1.0, 0.9, 0.2)` |

So the "cube" is really a 0.05 x 0.05 x 0.3 u box = **0.5 m x 0.5 m x 3 m**, an
elongated amber slug, not a literal cube. It is already the right shape family;
it is untextured and identical for both damage types.

`projectile_render_mesh` is a `TurretSectionConfig` field, so it is
**per-turret, not per-round-type**. Both PDC prototypes leave it `None`. Since
the fired round's type comes from the runtime `LoadedBullet` slot (`firing.rs`:
`loaded.map(|l| (l.kind, l.damage))`) and NOT from the authored config, a turret
that swaps ammo types would keep whatever mesh its config named. **Per-type
models cannot be expressed by the current field.**

### What per-`DamageType` models would take

The projectile already carries `ProjectileDamage { kind, .. }` at spawn, so the
observer has the type in hand with no new plumbing - `insert_projectile_render`
would just query it alongside `BulletProjectileRenderMesh`.

Two viable shapes, and the choice turns on one hard constraint:

**The performance constraint is real and load-bearing.** `DefaultProjectileRender`
exists specifically because the `None` arm is the shipped path at 100 rounds/s
per muzzle, and a test pins it: `default_projectile_render_allocates_no_assets_per_shot`
(`render.rs:358`) asserts that 65 spawned bullets add **zero** mesh and material
assets and all share one handle. **Any per-type scheme must keep exactly one
mesh+material handle PER TYPE, built once, never per shot.** That test will
need extending, not deleting.

- **Option A - generated meshes (recommended, cheapest).** Widen
  `DefaultProjectileRender` from two handles to a small map or struct keyed by
  `DamageType`, built once in `FromWorld`. Kinetic keeps the stubby slug;
  Pierce gets a longer, thinner dart (e.g. `Cuboid::new(0.03, 0.03, 0.5)` or a
  `Cylinder`/`Capsule` primitive). Colours are already defined and shared:
  `damage_type_color(kind)` (`damage.rs:244`) returns amber for Kinetic and
  steel blue for Pierce, and the HUD ammo readout already uses it - so the
  round in flight would match the pip colour for free. No new assets, no new
  content-pipeline work, no `.content.ron` regeneration. Also note
  `nova_gameplay/src/mesh/builder.rs` exists for procedural meshes if
  primitives are not enough.
- **Option B - authored GLBs.** Add per-type `AssetRef<WorldAsset>` fields to
  `TurretSectionConfig` (or a `HashMap<DamageType, AssetRef<WorldAsset>>`),
  author them in `BaseContentAssets` beside `turret_yaw`/`turret_barrel`
  (`base_content/assets.rs:63-66`) as `self://gltf/...glb#Scene0`, model them
  in Blender under `art/blender/` (today: `hull-01.blend`, `turret-01.blend`,
  `torpedo-bay-01.blend`), export to `assets/base/gltf/`, then regenerate with
  `cargo run content -- gen` because `assets/base/**/*.content.ron` is
  generated and must never be hand-edited. `WorldAssetRoot` spawns a full scene
  per round, which is much heavier than a shared `Mesh3d` - at 100 rounds/s
  this needs measuring before it ships.

**Recommendation: Option A.** It gives the requested "a slug and a penetrator
read differently in flight" with no asset pipeline work, keeps the shared-handle
invariant intact, and reuses the colour vocabulary the HUD already speaks.
Option B is the right answer only if the rounds want real geometry and texture,
and it should be measured at the shipped fire rate first.

For reference, the torpedo has the same structure and the same default-primitive
fallback (`torpedo_section/render.rs:55`, `Cuboid::new(1.0, 1.0, 1.0)` - a true
unit cube, 10 m). It also authors `projectile_render_mesh: None` in the catalog,
so shipped torpedoes fly as 10 m cubes too. Out of scope here, same fix shape.

---

## 11. Which levers interact - the summary for the diagram

```
                       LIFETIME (the chosen lever)
                              |
                              v
             reach = muzzle_speed * lifetime
                   |                    |
                   v                    v
    AI fire gate (x0.9)          player reach (NO HUD readout)
                   |
      +------------+-------------+----------------+
      v            v             v                v
 AI_STANDOFF   pd_range    AI_ENGAGE_RANGE   balance.rs audit
  + BAND       (400)          (800)          threat envelope
  (250+60)                                   -> CI gate + acks
      |            |
      |            v
      |     rounds fired per intercept = fire_rate * pd_range / (100 + v_torp)
      |            |
      |            v
      |     AMMUNITION ECONOMY  <---- SectionAmmo(500) / reload(3.0s)
      |
      v
 whether the AI can shoot at all

              REFERENCE_CLOSING_SPEED (100.0)
                    ^                ^
                    |                |
             muzzle_speed      ship + torpedo speeds
             (LOCKED)          (move the argument, not the anchor)
```

| Lever | Reach | Where fights happen | Ammo cost | Per-hit damage | CI |
|---|---|---|---|---|---|
| `projectile_lifetime` | **YES** | via the gate | via pd_range | **no** | balance audit |
| `muzzle_speed` | yes | via the gate | yes | **YES - global** | balance audit |
| `AI_STANDOFF_RANGE` + BAND | no | **YES** | no | no | no |
| `pd_range` | no | no | **YES - linear** | no | no |
| `AI_ENGAGE_RANGE` | no | approach length | no | no | no |
| `fire_rate` | no | no | **YES - linear** | no | dps in audit |
| `ammo_capacity` / `reload` | no | no | **YES** | no | no |
| `projectile_health` (ordnance) | no | no | ~2 rounds either way | no | no |
| ship / torpedo speeds | **YES (0.75-1.7x)** | yes | yes | **YES** | no |

---

## 12. What surprised me

1. **There is no turret `range` field.** Reach is entirely
   `muzzle_speed * projectile_lifetime`, recomputed at three separate call
   sites (`guns.rs` fire gate, `balance.rs:173` audit, and the acquisition doc
   comment's hardcoded "450 m"). Changing lifetime changes a number that three
   modules independently re-derive and one CI gate grades.
2. **`AI_STANDOFF_RANGE = 250` is the real engagement-range knob** and it is
   absent from the task brief. It, not the fire gate, decides where ships sit.
3. **An intercept costs ~296 rounds, not 2.** The gun cannot know its target is
   dead until one time-of-flight later, and point defence explicitly bypasses
   the burst cadence. 99.3% of point-defence ammunition is spent on already-dead
   torpedoes.
4. **Shortening range does not help torpedoes survive.** Window and flight time
   shrink together; the ratio is pinned near `(100 + v_torp)/v_torp`.
5. **Ordnance hp is the wrong lever by two orders of magnitude.** A standard
   torpedo needs ~2000 hp, not 10 or 20, to survive one PDC at 200 u.
6. **A ship defends against one torpedo at a time**, however many turrets it
   has - `AIPointDefenseTarget` is a single `Option<Entity>`. Adding PDC mounts
   does not add intercepts.
7. **A torpedo dies at 10 hp, not 20.** Both body sections carry
   `projectile_health` and killing EITHER kills the whole torpedo
   (`bay.rs:16` `on_torpedo_body_destroyed`).
8. **The pierce PDC is already a hard anti-salvo weapon on paper** - 6 layers
   at 2.0 flat, so one round can gut three aligned torpedoes for ~7.4 power out
   of 300 - and nothing in the game ever lines a salvo up to show it.
9. **The light turret already breaks the reference invariant.** At 60 u/s its
   authored 3.825 lands as 2.295 in a station-keeping duel.
10. **A PDC round cannot miss.** No dispersion, exact lead solve, 180 deg/s
    traverse, and point defence is exempt from the line-of-fire ray. Every
    "how tough should ordnance be" question is downstream of that.
11. **Finite ammo is already the default everywhere except the webmods.** The
    whole base campaign ships `infinite_ammo: false`; all six ledger chapters
    and the gauntlet ship `true`. The engine change to demote it is one
    condition; the real cost is seven hand-written scenario RONs that have
    never been played with a magazine.

---

## 13. Uncertainty - stated, not estimated

- **No live measurement was taken.** Everything here is read from source or
  derived by the quoted formulas. Nothing was run; no temporary test was
  written or deleted. The next step before any retune should be a live run.
- **Torpedo cruise speed.** All torpedo arithmetic uses `max_speed` (35 / 70)
  as the closing speed. `linear_damping` is 0.8 / 0.4 and `thrust_headroom`
  tapers thrust over the last 5 u/s, so true cruise is **at or slightly below**
  `max_speed`, and acceleration from the 1.0 u/s launch push takes an
  unmeasured time. Exposure windows are therefore slightly OPTIMISTIC for the
  defender (a slower torpedo is exposed longer) and the intercept ToF slightly
  PESSIMISTIC. Magnitude unknown without a run.
- **Slew time between point-defence targets is ignored.** At 180 deg/s and the
  small angular separations of a salvo it should be well under 0.1 s, but the
  sequential-kill count in 5c does not include it, nor does it include the
  `AI_FIRE_ALIGNMENT = 0.95` settle.
- **Gravity.** `gravity_well_system` applies a mass-INDEPENDENT acceleration to
  bullets (`damage.rs:136`), so near a well a round's path bends and its reach
  against a target shortens by an unmeasured amount. Every reach figure assumes
  a straight line.
- **`TempEntity` ticks in `Update` (frame delta) while the round integrates in
  `FixedUpdate`.** Both are wall-clock, so reach should not vary with frame
  rate, but the expiry lands on a frame boundary rather than a physics tick.
  Not measured.
- **The "rounds fired per intercept" figure assumes the trigger is already
  held** when the torpedo crosses `pd_range`. A turret that has to slew onto a
  new PD target loses a fraction of a second of that, making the real figure
  slightly lower.
- **Whether both torpedo body sections are equally likely to be hit** was not
  determined. Both are unit cubes (`BaseSectionConfig::collider: None` ->
  unit cube, `base_section.rs:79`) at local Z = 0 and Z = 1, so a head-on round
  meets the controller first. This matters only for the pierce case.
- **Option B round rendering (GLB scenes at 100 rounds/s) is unmeasured.**
  `WorldAssetRoot` spawns a scene per round; whether that is affordable at the
  shipped fire rate is exactly the kind of thing that needs a run, not a guess.

---

## 14. What the IMPLEMENTATION changed about this document (2026-08-15)

The engagement-range half of the spike is now landed. Two of its
recommendations did not survive contact, and one uncertainty is measured.

### 14a. A single global lifetime is not just "a grade difference" - it is a
### silent kill switch on most shipped hostiles

Section 2 states the 60 u/s guns keep 60% of the reach at the same lifetime and
calls the result "a defensible grade difference". That is true of the reach and
FALSE of the fight. The standoff band is GLOBAL, so at lifetime 2.0 the
scavenger gun's fire gate (108 u) falls INSIDE the retuned orbit band
(75-125 u): the ship flies its envelope correctly and holds fire over the outer
half of it. The catalog check that matters is which gun the hostiles carry, and
`cargoa_turret_*_light` (60 u/s) is what the shakedown pirate, the lifeline
raiders, the final_tally hostiles and ledger_ch3 all fly - i.e. MOST armed AI
in the game.

Landed instead: lifetime is authored PER GRADE. 2.0 s for the 100 u/s guns
(200 u reach, 180 u gate), 3.0 s for the 60 u/s guns (180 u reach, 162 u gate).
The grade difference stays in reach (180 vs 200) and where it always was - fire
rate (25 vs 100), applied per-hit (2.295 vs 4.0 station-keeping), magazine
(150 vs 500).

The standing constraint is now a test, not a comment:
`every_authored_turret_reaches_past_the_standoff_band` (nova_authoring
base_content) grades every authored turret's
`muzzle_speed * lifetime * AI_FIRE_RANGE_FACTOR` against the exported
`AI_STANDOFF_OUTER_EDGE`, and fails when a gun cannot reach the band its own AI
flies. Confirmed fail-first by setting the light turret back to 2.0.

### 14b. `AI_FIRE_RANGE_FACTOR` stays at 0.9

Section 7 says ~0.75 would be strictly safe against a target fleeing at 25 u/s.
It would also cost a quarter of the envelope in the closing case a fight is
decided in, and it tightens the margin over the standoff band - the failure
mode this whole change exists to avoid. The error the factor fails to absorb is
PROPORTIONAL to reach, so cutting lifetime 2.5x already shrank it 2.5x: 75 u of
overshoot at the old 5.0 s, 30 u now. Kept, with the reasoning recorded at the
constant.

### 14c. `AI_ENGAGE_RANGE` is safe to halve because combat states ignore it

Section 3 worried that shrinking it strands the far spawns (ledger_ch2 at
500/800 u, lifeline and final_tally at 700 u). It does not: `AIBehaviorState`
defaults to `Engage`, and combat states hold on ANY acquired target out to
`AI_TARGET_MAX_RANGE` (2000 u). The gate only governs PASSIVE states, and every
far-spawned hostile in shipped content is either freshly spawned (Engage) or on
a patrol route that leads into the player's area. 400 u, giving
`(400 - 180) / 20 = 11 s` of silent closing against 17.5 s before.

### 14d. Live measurement (the spike took none - Section 13)

`menu_duel` backdrop, headless on Xvfb, twice, with the fire gate instrumented
to log the distance at which each turret's trigger opens:

| measure | run 1 | run 2 |
|---|---|---|
| trigger-open events, act one | 79 | 35 |
| first shot of the fight | 179 u | 179 u |
| closest engagement | 82 u | 73 u |
| mean engagement | 119 u | 123 u |
| point-defense opens (vs the siege torpedo) | 148 u, 96 u | inside 150 u |
| outcome | `duel_rival` neutralized, then `duel_victor` | same |

So fights open at the gate (180 u / 1.8 km) and settle at the standoff
(~100 u / 1.0 km), the intended shape. Both acts of the backdrop played
through: the AI closed, opened fire, killed every section of the loser, and the
winner's point defense fought the siege torpedo before losing to it. No shot
was fired from outside reach, and no ship sat outside its own gate.

### 14e. Still true, and still not implemented

Sections 4, 5 and 9 (torpedo survivability, the ammunition economy, demoting
infinite ammo) are UNTOUCHED by this change and their arithmetic still holds -
with one number improved for free: at the new 150 u `pd_range` an intercept
costs ~111 rounds instead of ~296, so a 500-round magazine now covers ~4.5
intercepts instead of 1.69. Section 10 (round rendering) is untouched.
