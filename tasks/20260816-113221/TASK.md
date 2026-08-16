# Torpedo types: how smart a torpedo is, as a choice

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.11.0, combat, balance, content, modding

## Goal

Owner: "can we have a tunable parameter on the torpedo bay that is like 'how
smart my torpedoes are?' so 2 different torpedo bays one is less evassive and one
is more evassive ... same as the different ammo types in a way; so it should be
more like torpedo TYPE or something; they both deal same blast damage but one
type is more evasive than the other, and for a good reason, if you just want to
blast something really long range away that you know doesn't fight back you would
use the dumb one, but for high intense combat use the evasive one".

## The mechanism already exists

`weave_angle` and `weave_rate` are per-bay authored fields
(`crates/nova_ship/src/sections/torpedo_section/projectile.rs:1046`;
`weave_angle` of zero flies the bare intercept). Shipped content ALREADY carries
two different settings - 0.44 and 0.22 in
`crates/nova_authoring/src/base_content/sections/standard.rs:593` and `:690`.

So the physics is done. What is missing is that these are two numbers on two
bays: no identity, no player-visible difference, and no reason to pick one over
the other. This task turns a parameter into a CHOICE.

## The trade is already in the physics - do not invent one

Blast damage stays equal, per the owner. So the cost of evasion has to come from
somewhere else, and it already does: **a weave lengthens the flight path.** A
weaving torpedo covers more distance to reach the same point, so it arrives later
and has less effective reach for a given lifetime.

That is exactly the split the owner described - dumb for long-range bombardment
of something that will not fight back, evasive for a close fight. Verify the
effect is large enough to feel before adding any second lever. If it is not,
report the measurement rather than inventing an artificial penalty.

## Bay property or loadable type?

The owner compares it to ammo types. Turrets already swap ammo at runtime through
the `LoadedBullet` slot, and the built-in round art keys off the FIRED damage
type rather than the authored config, so a turret that swaps ammo swaps what its
rounds look like (`turret_section/render.rs:105-125`).

Decide deliberately whether a torpedo type is:

- an authored property of the bay (simplest; a ship's loadout is fixed at build)
- a loadable the bay carries, mirroring `LoadedBullet` (richer; matches the
  owner's ammo-type analogy, and lets one bay carry both)

Prefer whichever adds less machinery. State the choice and the reasoning.

Whichever it is, the two types want distinct names, distinct catalog entries, and
enough visible difference in flight that a player can tell which one they fired.

## Campaign easing, and why it is the same change

Owner: "in the mainline campaign let's make the first encounter with a torpedo
ship 'easier' because it is a bit hard".

Find the FIRST mainline encounter that fields a torpedo bay - do not guess it.
The obvious lever makes the easing and the feature one change: give that
encounter the DUMB type. A straight-flying torpedo is what point defence is good
at, so the first time a player meets ordnance it is a threat they can actually
answer, and the evasive type is what escalates later.

Check the encounter is genuinely too hard before retuning it, and say what you
measured. Note also that the ledger ch4 Auditor balance ack is STALE: it claims
"the light mook turret (downgraded for exactly this spawn)" but no `_light`
variant exists, and the ack rests on a playtest taken when ammunition was
infinite.

## Definition of done

- two torpedo types a ship can be authored with, differing in evasion, equal in
  blast damage, and distinguishable in flight
- the trade measured, not asserted - time to target and effective reach for each
- a mod can author its own type
- the first mainline torpedo encounter retuned, with the measurement that says it
  was too hard and the measurement that says it no longer is
- a live playtest with rendered evidence, not only passing tests

## Lane

sprout `torpedo-types`.
