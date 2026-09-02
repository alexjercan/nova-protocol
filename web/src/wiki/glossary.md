# Glossary

Short definitions for the terms and units that recur across the wiki, grouped the way the book reads: units, then flight, ships, targeting, combat, the interface and the world. Each entry points at the page that owns it.

## Units

- **m / km** - the distance units. Ranges and radii read in metres below a kilometre (e.g. a standoff of about 500 m) and in kilometres above it (e.g. `1.24 km`). Every distance readout on the HUD and in the NOVA OS map uses them.
- **m/s** - the speed unit, metres per second. The speed chip beside the velocity sphere reads in `m/s`, and a target's closing speed reads as a signed `m/s`.
- **u** - the game's own distance unit, ten metres. The section catalogs and the scopes on these pages count in it: a hull cell is one u on a side, a PDC round reaches 200 u, a railgun slug 1800 u. `u/s` is the matching speed and `u/s2` the matching acceleration.
- **hp** - hit points, the health a section or a torpedo carries. Every hit subtracts from it and the part dies at zero; each section page lists its variants' figures under [Variants](../sections/).

## Flight

- **Newtonian** - momentum persists and nothing dampens you: a burn adds speed, only an opposite burn takes it away, and turning the nose does not turn your velocity. There is no flight-assist toggle - see [Flight & autopilot](../flight-autopilot/).
- **Prograde / retrograde** - prograde is the direction you are moving; retrograde is the opposite. STOP flips you to retrograde and burns to kill your speed.
- **Burn** - a held main-drive throttle. The main drive is the sum of every thruster that points forward, spooled smoothly up and down rather than snapped. See [Manual flight](../flight-autopilot/#manual-flight).
- **Speed cap** - a soft ceiling a scenario can put on a hull, like the Shakedown's 250 m/s governor. The burn tapers to zero along its own axis over the last stretch; turning and braking are never capped.
- **GOTO / ORBIT / STOP** - the autopilot verbs. Each flies a whole maneuver on the real controller and thrusters and hands the ship back, and any manual input disengages it. GOTO burns to the nav lock and settles at the standoff, ORBIT holds a ring around the dominant well until you break away, STOP flips retrograde and burns to rest. See [The autopilot flies the hull](../flight-autopilot/#the-autopilot-flies-the-hull).
- **Arrival envelope** - the autopilot's one rule: at any distance it caps closing speed at what a flip-and-brake from there can still cancel, so the braking ramp always lands on the standoff.
- **Flip** - the swing to retrograde that puts the drive against your velocity before a braking burn. The autopilot schedules it a swing early, because the swing itself takes time.
- **Standoff** - the resting distance GOTO stops at, just off a target (about 500 m plus the target's radius, measured from the surface) rather than ramming it.
- **RCS** - reaction-control fine translation for docking: hold <kbd>Shift</kbd> and steer with the mouse and scroll wheel to nudge the hull straight along its own axes, no rotation, capped at a gentle per-axis speed. A trim a scenario grants rather than standard flight - see [RCS](../flight-autopilot/#rcs-fine-docking-thrusters).
- **Centre of mass** - the point the flight computer balances every burn through. A section weighs the box it is hit on, so the centre moves as the hull is built and as it is shot apart, and the computer rebalances live. See [Balancing thrust through the hull](../sections/thruster/#balancing-thrust-through-the-hull).
- **Gravity well** - a large asteroid's or planetoid's pull: inverse-square, authored per body, felt by piloted ships and by rounds and torpedoes in flight, and always escapable under main drive. See [Gravity wells](../gravity-wells/).
- **Sphere of influence** - the reach of a [gravity well](../gravity-wells/#sphere-of-influence), set by the body's authored mass alone: the distance where the pull decays to a fixed cutoff. Outside it the well does not pull on you; only the dominant well inside it matters.
- **Dominant well** - the one well that counts where you are. ORBIT circularises around it and STOP budgets for its pull. See [The dominant well](../gravity-wells/#the-dominant-well).
- **Hysteresis** - a bit of stickiness that stops a state from flickering at its edge. A lock, a fine-lock section, and the dominant well all hold a little past their switch point so they do not chatter.

## Ships

- **Section** - one part of a hull: a hull block, a controller, a thruster, a turret, a torpedo bay or a railgun. Each is its own body with its own mass, its own health and one job. See [Ship sections](../sections/).
- **Cell** - one unit cube of the build grid. Sections are sized in cells - a torpedo tube is two, a railgun three - and a hull's mass is the sum of its boxes.
- **Hull section** - the passive structural block the other sections mount to, and where most of a ship's mass is. See [Hull](../sections/hull/).
- **Controller** - the flight computer. It turns the ship, flies the autopilot verbs and works point defense; lose the last one and the ship is neutralized. See [Controller](../sections/controller/).
- **Socket** - a face a section bolts on through. A thruster offers only its forward face, so it always seats nose-in with the plume clear of the ship. See [Thruster](../sections/thruster/).
- **Variant** - one shipped prototype of a section kind, listed at the foot of each section page: the Kinetic and Pierce PDCs on their two mounts, the Serpent and Lance bays, the base ships' own engines and pods.
- **Cladding** - decorative plating and fixtures. They take damage and sections can shield them, but they neither shield anything nor slow a blast's pressure. See [What damage looks like](../ships/#what-damage-looks-like).
- **Structural depth** - how far into a hull a blast reaches. A section that survives its pressure stops the wave; a destroyed one passes 65 percent on. See [What a warhead does to a hull](../sections/torpedo-bay/#what-a-warhead-does-to-a-hull).
- **Wreck** - a healthy piece of hull that a cut disconnected from the controller. It keeps drifting, keeps its shape and still takes damage, but nothing aboard flies or fires it. See [Taking a ship apart](../ships/#taking-a-ship-apart).
- **Neutralized** - out of the fight for good. A ship that loses its last working gun, or the flight computer it had, stops fighting even though its hull survives: the [viewfinder](../hud/#target-viewfinder) tags the wreck NEUTRALIZED, its mounts stop answering, and it keeps drifting rather than despawning. See [Taking a ship apart](../ships/#taking-a-ship-apart).

## Targeting

- **Radar sweep** - hold <kbd>Ctrl</kbd> to run the radar: it tracks the best body inside an 18-degree cone around your aim and re-targets as you sweep. See [Holding to sweep](../targeting-radar/#holding-to-sweep).
- **Lock-on dwell** - the short hold a lock has to be earned through. Pointing at something does not lock it; holding steady on it does.
- **Nav lock / combat lock** - the two lock slots. A lowered (white) stance sweeps a nav lock the autopilot flies to; a raised (red) stance sweeps a combat lock the guns aim at. See [Stances and slots](../targeting-radar/#stances-and-slots).
- **Stance** - whether your weapons are lowered or raised. It picks which slot a sweep fills, and it is one of the two things that make the weapons hot.
- **Fine-lock** - drilling a combat lock into one specific [section](../sections/) of an enemy hull, so turrets and the viewfinder focus that part. See [Per-section fine-lock](../targeting-radar/#per-section-fine-lock).
- **"Hot" weapons** - your weapons are hot while you hold a raised (red) combat stance or while a combat lock exists. Hot means turret lead pips and the viewfinder frame go red and you can fire.
- **Weapons safety** - the lowered state, the opposite of hot: no trigger fires, a bay keeps its iris shut and a railgun will not start a charge. Raise the weapons or take a combat lock and it is off.
- **Lock range** - how far out the radar will take and hold a lock. See [Lock ranges](../targeting-radar/#lock-ranges).
- **Clearing** - dropping locks, in stages rather than all at once. See [Clearing locks](../targeting-radar/#clearing-locks).

## Combat

- **Reach** - how far a round, slug or torpedo gets before it expires: muzzle speed times lifetime, never an authored number. A PDC round reaches 200 u, a railgun slug 1800 u, a torpedo about 3000 u. See [Three reaches](../combat-weapons/#three-reaches).
- **Engagement ladder** - the three reaches on one axis, and the shape of every fight: torpedoes at the long end, the railgun in the middle, guns close in.
- **Closing speed** - how fast you and your target converge along the round's line of flight. It scales Kinetic damage and Pierce depth in both directions and moves the reach with them. See [Closing speed](../combat-weapons/#closing-speed).
- **Cover** - a round expends itself on the first tangible thing it meets, so a rock between you and a hostile eats the burst that was meant for it. See [Combat](../combat-weapons/).
- **Kinetic / Pierce** - the two bullet behaviors, told apart by tracer colour. A Kinetic round punches: it stops at the first section it fails to destroy. A Pierce round rakes: it crosses several sections and damages every one on the way through. See [Damage types](../combat-weapons/#damage-types).
- **Explosive** - the torpedo's damage type: area pressure falling off from the centre of the blast, with no speed term, worked through the hull by structural depth.
- **Power** - a Pierce round's depth budget. Every section it crosses spends some of it, the tougher the section the more, and the round stops when it is gone. A railgun slug carries 1800. See [How far a round travels](../combat-weapons/#how-far-a-round-travels).
- **Intercept lead** - aiming where the target will be when the round arrives, computed in the shooter's own frame. Every turret does it, and the lead pip shows it. See [Aiming with lead](../sections/turret/#aiming-with-lead).
- **Barrel discipline** - a gun fires only while its barrel is actually on the aim point, so it holds through a slew and a mount that cannot bear simply waits. See [Barrel discipline](../sections/turret/#barrel-discipline).
- **Blind cone** - the sky under a turret's own keel that its barrel cannot depress into: ten degrees below level and everything under that. See [What it can bear on](../sections/turret/#what-it-can-bear-on).
- **Point defense** - your turrets' automatic close-in fire against inbound torpedoes. Each mount picks and holds its own torpedo, and your flight computer works the mounts you are not using; every intercept costs real ammunition. See [Turret](../sections/turret/#point-defense) for the mount and [Combat](../combat-weapons/#point-defense) for the battery.
- **Saturation** - firing at a facing from more directions than it has mounts to answer. Torpedoes arriving from one side, or from under a hull, meet only the guns that can train on them.
- **Magazine** - what a weapon holds and how it gets it back: a batch lands after a quiet delay, all at once or not at all, and any shot restarts the clock. See [Combat](../combat-weapons/).
- **Lance / Serpent / Breaker** - the [torpedo](../sections/torpedo-bay/#the-two-run-ins) types, each named and tinted on the viewfinder when you lock one. The Lance runs straight and fast, so point defense can answer it; the Serpent corkscrews on approach and costs far more rounds to stop; the Breaker is the huge crimson siege warhead scripted batteries fire. (The railgun's catalog entry is named Railgun Lance; these pages call it the railgun.)
- **Weave** - the Serpent's terminal corkscrew, laid over its guidance so a lead solution is never quite right, and tapering out on the doorstep.
- **Arming gate** - the short time or distance after launch before a torpedo's guidance and fuze go live, so it cannot go off in your lap.
- **Fuze** - a torpedo bursts about thirty metres from the nearest part of its target's skin; with nothing left to reach it bursts a half-radius short. See [How a torpedo runs in](../sections/torpedo-bay/#how-a-torpedo-runs-in).
- **Blast** - the warhead's pressure: full at the centre, zero at the edge of its radius, and worked through a hull by structural depth. See [What a warhead does to a hull](../sections/torpedo-bay/#what-a-warhead-does-to-a-hull).
- **Iris** - the six-petal muzzle door on a torpedo bay. It opens for a launch and for nothing else, so an open iris on another hull means a torpedo is coming. See [Shut until it fires](../sections/torpedo-bay/#shut-until-it-fires).
- **Railgun** - the [spinal gun](../sections/railgun/): three cells of rails with no traverse, so the hull is the aim. A tap commits a 1.5 s charge that only the weapons safety can stop, and the slug that leaves rakes a corridor through everything in the line. One shell, a twelve-second reload.
- **Charge** - the 1.5 seconds between the trigger tap and the shot, committed: the slug leaves whether or not the nose is still on the target. See [Committing the shot](../sections/railgun/#committing-the-shot).
- **Bore sight** - the [HUD](../hud/#bore-sight) line a railgun-carrying hull draws from its muzzle to where the slug would stop, with a ring on every section the shot would destroy. Up while weapons are hot; dimmed through the reload.
- **Corridor** - what a railgun slug leaves: the bore column the slug's tip cut, widened by a sphere trailing the tip to about three cells across, entry to exit. Every section in it pays out of the one power budget, so a wider rake is never more damage, only a differently shaped hole - see [What one shot takes out](../sections/railgun/#what-one-shot-takes-out).
- **Rake** - the sphere a railgun slug drags behind its tip. It widens the cut into the corridor and only ever touches what the tip has already reached.
- **Recoil** - the shove a railgun gives the ship that fired it, landing at the muzzle: a gun off the ship's axis yaws it as well as pushing it back. See [The recoil](../sections/railgun/#the-recoil).
- **Carve** - what a hit does to a rock: it takes real material out of the asteroid's shape, so the silhouette and the thing you can fly into both change. Only rocks carve - a ship's parts keep the shape they were built in until they die. See [Shooting rock](../combat-weapons/#shooting-rock).
- **Crater** - the hole a carve leaves. It stays, and fire landing near it deepens the same hole instead of opening a new one.

## Interface

- **HUD** - the heads-up display: diegetic, because the instruments read the ship's real state, and contextual, because elements surface while their moment is live and settle back when it passes. See [HUD](../hud/).
- **Diegetic** - part of the world rather than an overlay bolted on top. The HUD instruments read the ship's real state and the autopilot flies the same real thrusters you do, so what you see is what the ship is actually doing.
- **Velocity sphere** - the flight readout that shows where the ship is going, with the speed chip beside it. It turns violet while RCS is held. See [Flight readouts](../hud/#flight-readouts).
- **Viewfinder** - the target frame: what you have locked, how far it is and how fast it is closing, and the NEUTRALIZED tag on a wreck. See [Target viewfinder](../hud/#target-viewfinder).
- **Lead pip** - the marker a turret draws on the point it is actually shooting at, ahead of a moving target. Red while the weapons are hot. See [Locks and reticles](../hud/#locks-and-reticles).
- **Allegiance markers** - the always-on ship markers, tinted by how each ship stands to you. See [Allegiance markers](../hud/#allegiance-markers).
- **Keybind dock** - the strip of chips listing the verbs your ship has right now. A chip appears only when the ship has the verb, RCS included. See [Keybinds](../keybinds/).
- **NOVA OS** - the ship computer. <kbd>Tab</kbd> in flight brings up a CRT terminal that freezes the world while you read, type and click, with the MAP and SHIP apps and section rebinding. See [NOVA OS](../nova-os/).
- **Comms** - the story panel where a scenario talks to you and lists its objectives. See [Comms and objectives](../hud/#comms-and-objectives).

## World

- **Allegiance** - Player, Enemy or Neutral: the one side every ship carries. Any two things resolve to Own, Hostile or Neutral, and that relation drives acquisition, whose rounds hurt whom, and reticle tint. See [Factions](../factions/).
- **Raider** - an enemy gunship. It flies the same catalog guns as you on scavenger-grade mounts, closes to about a kilometre and fights there. See [Turret](../sections/turret/#variants).
- **Scenario** - a placed world and the events, filters and actions wired over it: the same machinery for a five-minute tutorial and a combat sandbox. See [Scenarios](../scenarios/).
- **Objective** - what a scenario asks of you, listed on the comms panel and advanced by the scenario's events.
- **Shakedown Run** - the first campaign leg, the training flight [Your first flight](../getting-started/) walks through beat by beat under a 250 m/s governor.
- **Rock** - an asteroid: solid, carvable cover that eats rounds, and a gravity well if it is big enough to carry one. See [Shooting rock](../combat-weapons/#shooting-rock).
