# Combat balance: engagement range, ammunition economy, ordnance survivability

- STATUS: IN_PROGRESS
- PRIORITY: 74
- TAGS: v0.11.0, balance, combat, spike

## Goal

Combat mechanically works after the v0.11.0 damage pass - ships die, and the
kill splits roughly 50/50 between neutralization and destruction. The NUMBERS
are wrong. Owner's read: "the fighting is COOL but right now it's not balanced
at all".

Three separate problems, one balance pass.

## 1. Engagement happens too far out

Nova's scale is 1 unit = 10 m, so 1 km = 100 units. Ships currently open fire
around 400-500 units, which is 4-5 km. That reads as long range for a game at
this scale.

Target: PDCs effective at roughly 1-2 km (100-200 units). The Expanse reference
the game leans on has PDCs at ~5 km and torpedoes at ~1000 km, but Nova's world
is far smaller, so the ratio matters more than the absolute figures.

Known knobs: turret `range`, `muzzle_speed`, projectile lifetime, AI
`engage_range` (default 800) and `pd_range` (default 400).

CAUTION: `REFERENCE_CLOSING_SPEED = 100.0` is the shipped PDC muzzle speed, and
both speed-driven damage curves read exactly 1.0 at it. Shortening range by
slowing the round moves the reference and rebalances all weapon damage as a side
effect. Shortening it via lifetime or turret range does not. Decide this
deliberately.

## 2. Point defense has no cost

Turret ammo is effectively unlimited (the ledger grants infinite), so spamming
PDC fire is free and an enemy can shoot down every torpedo without a decision.

Wanted: ordinary finite ammunition on the normal path, with infinite ammo
demoted to a DEBUG-only cheat for examples and harness runs. Owner: "it's not
really something I would use unless it's an example and I am testing something".

## 3. Torpedoes are too easy to kill

They fly straight and die to point defense. The Expanse answer is evasion; the
owner explicitly does not want that complexity yet. Cheaper levers to weigh:
slower PDC reload, tighter PDC tracking, fewer effective intercept seconds from
the shorter engagement range, or ordnance toughness.

Note what already changed this release and compounds here: ordnance went 1 -> 10
hp, turret DPS halved to its authored value when the double-pay bug was fixed,
and a piercing round can kill one warhead and carry on into a second.

## 4. Rounds are untextured cubes

Turret projectiles render as cubes. They want a smaller, better-looking model,
and a distinct one per damage type so a kinetic slug and a penetrator read
differently in flight.

## Owner decisions

- **Range comes down via projectile LIFETIME, not muzzle speed.**
  `REFERENCE_CLOSING_SPEED = 100.0` is the shipped muzzle speed and both damage
  curves read exactly 1.0 there, so slowing the round would silently rebalance
  every weapon in the game. The spike confirms lifetime moves nothing else.
- **Infinite ammo goes false EVERYWHERE, including the ledger campaign.** The
  seven hand-written webmod RONs that ship `infinite_ammo: true` (ledger ch1,
  ch2, ch2b, ch3, ch4, ch5, and gauntlet) all become false. The owner is aware
  this means each of those scenarios is played with finite ammunition for the
  first time.
- **Gauntlet flies the RACER, unarmed.** With no weapons the ammo question does
  not arise there at all, and the scenario becomes a survival run rather than a
  shooting one.

## Approach

Spike first, implement second. The spike must produce actual numbers, not
adjectives:

- the full inventory of range knobs and what each currently reads
- time-of-flight and intercept-window arithmetic at candidate ranges
- how many PDC rounds a torpedo intercept costs now, and at each candidate
- what a magazine has to hold for point defense to be a decision rather than a
  reflex
- prior art worth borrowing from (Nebulous: Fleet Command and Children of a Dead
  Earth are the closest analogues; real CIWS engagement envelopes for the shape
  of the problem)

Record it as `SPIKE.md` beside this task, with a diagram the owner can read.

## Definition of done

- Engagement opens inside the intended envelope and PDCs are effective at 1-2 km
- Turret ammunition is finite on the normal path; infinite is debug-gated
- A torpedo salvo is a threat that costs something to answer, without evasion AI
- Rounds render as per-type models rather than cubes
- Numbers are justified in `SPIKE.md`, not tuned by feel alone
- A live playtest run, with rendered evidence, not just passing tests
