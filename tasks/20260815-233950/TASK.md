# Torpedo attrition: evasive ordnance, finite salvos, per-turret point defence

- STATUS: CLOSED
- PRIORITY: 72
- TAGS: v0.11.0, combat, balance, ai, ship

## Goal

Turn the torpedo-versus-point-defence exchange from a BINARY outcome into an
ATTRITION fight. A salvo should cost the defender ammunition whether or not it
connects, and both sides should be spending a finite resource.

Owner's framing: "use torpedoes to drain the PDCs... it's more of an attrition
fight". And on why the realistic model is not the goal: "realistic is not
necessarily fun".

Depends on `20260815-224512` (range retune and finite ammunition). Both change
what a round is worth, so tuning before they land is tuning against a moving
target.

## The design argument, so nobody re-derives it

Point defence really does dominate ordnance in reality - that is exactly why
navies worry about saturation rather than about single missiles. Simulate it
faithfully and you get Cosmoteer's diagnosis of its own missiles: "either
over-powered if the enemy didn't have enough point defenses, or near-useless if
the enemy had a decent amount... many battles won-or-lost before the battle even
began."

Distant Worlds 2 solves this by having point defence SUBTRACT damage rather than
kill. **The owner rejected that** - it is an accounting abstraction, not
something a player can watch happen.

EVASION is the concrete form of the same idea. A weaving torpedo makes point
defence spend rounds on where it was going to be. PD fire is never wholly
wasted, a torpedo is never trivially negated, and every intercept is still a
real kill or a real miss.

**It works here because the lead solution is real.** `lead_intercept_point`
computes a genuine intercept prediction for AI turrets, pinned by
`fire_aligns_with_the_leaded_aim_point_not_the_anchor`. Perturb the target's
path and the firing solution is wrong.

**It self-balances with no tuning.** Weaving lengthens the path, so an evading
torpedo spends MORE seconds inside the point-defence envelope. Evade harder,
survive each burst better, eat more bursts. The geometry prices it.

## Work, cheapest and highest value first

### 1. Terminal weave

An open-loop perturbation on top of the existing guidance - no awareness of
individual incoming rounds needed. Torpedoes have RCS on their controller; use
it.

Two things that decide whether this reads as evasive ordnance or as a drunk:

- **Amplitude.** Too much and it looks silly and misses; too little and the lead
  solution still works.
- **It must PERTURB the guidance solution, not replace it**, and decay on final
  approach, or torpedoes start missing stationary targets.

### 2. Limited torpedo reloads

Bays regenerate +1 every 4 s to a cap of 6, which is infinite torpedoes given
time. A hard magazine completes the exchange: the attacker spends torpedoes, the
defender spends rounds, and whoever runs dry first loses. Nearly free to
implement, and it is what makes the attrition economy real rather than notional.

### 3. Per-turret point-defence assignment

`AIPointDefenseTarget` is per-SHIP, so every turret on a hull dogpiles one
torpedo. That is the exact bug Sins of a Solar Empire II patched out in August
2024, and FTL still has it.

**The real defect is not overkill, it is idleness.** Owner: "some PDCs cannot
even reach the target all the time, so its bad, but it can reach other targets,
so it can split". A turret assigned a target outside its arc contributes nothing
while a torpedo it COULD have hit goes unengaged.

So the rule is: never assign a turret a target it cannot engage. Splitting falls
out of that for free. Each PD-capable turret picks its own target - reachable
given mount limits and own-hull occlusion, in range, most imminent first,
preferring a threat no other turret has claimed.

**Dwell is mandatory.** Reassign every tick and turrets swing between targets and
hit nothing, because slew time is real. Hold a target until it dies, leaves the
arc, or something far more urgent appears. Nebulous does exactly this.

**No micro-management.** Owner: "I don't want to micro manage the PDCs, maybe an
auto mode which scans for danger and shoots at everything". Point defence is
autonomous for player and AI alike. The ONE control worth exposing is an
auto-engage toggle - which is a real decision only because ammunition is now
finite, and would have been a pointless switch before.

### 4. Torpedoes intercepting torpedoes

Viable, because head-on you do not need to CATCH anything - closing speed does
the work. A new targeting mode, so land 1-3 first and see whether the attrition
fight already delivers what the owner wants.

### 5. Reactive dodging

Dodging individual observed rounds. Probably unnecessary: if open-loop weave
already breaks the lead solution, awareness buys little for a lot of complexity.
Evaluate, and do not build it unless 1 demonstrably falls short.

## Sequencing inside the task

3 makes point defence STRONGER; 1 makes it weaker. They pull opposite ways, so
they land together and are tuned together. Doing either alone means tuning
against a moving target.

## Ordnance durability - deliberately unresolved

Two independent derivations disagree, and the gap is real:

- prior-art analysis (Starsector's unguided Reaper: 500 HP, survives light point
  defence, dies to two interceptors) suggests ~400-500 hp plus a terminal sprint
- Nova's own spike computes ~2054 hp to survive one PDC at a 200 u `pd_range`

Exposure is `pd_range / torpedo_speed`, so the answer swings 2-4x on where
`pd_range` lands - which `20260815-224512` is deciding. Do not fix ordnance
health before that number is fixed.

## Rejected, with reasons

- **Damage subtraction** (Distant Worlds 2) - owner rejected it as abstract.
- **Nebulous wall thickness / AP thresholds** - the cleanest anti-HP-inflation
  model found, but a later lever, not this pass.
- **Cosmoteer's nerf-both-sides** - works, but requires retuning everything.
- **Priced PD-defeat catalogues** (Nebulous) - right idea, far too many knobs.

## Definition of done

- A torpedo salvo costs the defender ammunition whether or not it connects
- Turrets never sit idle on a target they cannot reach, and never all dogpile one
- Point defence is autonomous; the player's only control is one toggle
- Torpedo magazines are finite
- Evasion reads as evasive, not drunk, and torpedoes still hit what they aim at
- Numbers justified against the landed range and ammunition figures
- A live playtest with rendered evidence, not only passing tests

## Closing note

Five of the seven done-conditions were met as written. The other two:

- **"Torpedo magazines are finite" is SUPERSEDED, not unmet.** The owner's later
  call was that bays regenerate like every other weapon: "torpedo bays should
  regenerate the same way as the PDCs once they are out they regen". Landed as
  one torpedo per 10 s. The attrition economy survives because the attacker wins
  by out-carrying through the 6-round rack, never by outlasting - two bays run
  two point-defense mounts at 59% of capacity.
- **"Point defence is autonomous; the player's only control is one toggle" is
  MOVED**, not dropped. All point-defense code sits under
  `crates/nova_ship/src/input/ai/`, so it is a behaviour of the AI controller and
  a player hull gets none of it. That is a feature in its own right and is now
  tracked separately.

The second one leaves a live balance hole worth stating plainly: every figure in
this task assumes a defender that tracks perfectly and never stops firing - 369
rounds an intercept, one mount answering 0.17 torpedoes/s, bay regeneration set
against that. A human answering by hand is far worse than that, so a player
currently fights an attrition economy balanced against point defence they do not
have. Finite ammunition sharpens it.
