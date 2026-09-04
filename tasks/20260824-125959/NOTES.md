# Nova Protocol campaign notes

Working story and lore notes for task `20260824-125959`. These notes separate
accepted direction, dialogue drafts, and open decisions. They are not shipped
player documentation yet.

## Accepted direction

- Keep the mainline human and grounded. There are no aliens or hidden
  supernatural elements in the current campaign premise.
- Earth supports civilian industry and a separate navy.
- Pirates are outlaw crews using stolen, salvaged, and rebuilt ships.
- Earth naval ships patrol, enforce policy, protect routes, and project force.
  Their presentation draws from naval fiction such as The Expanse without
  copying its setting.
- The player works with an experienced industrial crew mining and recovering
  salvage in the belt.
- The player is an experienced engineer taking a first shift at the helm, not a
  novice learning their trade.
- The player's ship type is an **industrial cutter**. Its radio identity is
  **Cutter One**. The runtime id remains `cutter`.
- **Earthworks Industrial** is the current candidate for the Earth-owned
  industrial company. Meridian is one of its large industrial carriers.
- Scene 1 presents an ordinary workday. It does not explain pirates or the navy.
  The later attack is stronger if normal working life is credible first.

## Lore and faction documentation

The current Factions wiki page documents only the engine's three-state combat
relation model: Player, Enemy, and Neutral. Narrative factions must not be
collapsed into those runtime allegiances.

Future lore support should let each campaign or mod define its own setting
entries. The design should account for:

- organizations and political factions;
- employers, teams, and ship crews;
- named people, ships, fleets, and places;
- campaign ownership and namespaced ids;
- campaign-specific lore that does not claim to apply to every campaign;
- a wiki or codex view that groups entries by campaign and category;
- mod-defined lore without requiring edits to Nova Protocol's base lore.

The durable wiki should eventually distinguish mechanical **Allegiance** from
narrative **Factions and Organizations**. Do not implement the data format until
First Shift establishes what information the campaign actually needs.

## First Shift, scene 1

### Purpose

The opening conversation should:

1. Establish Meridian as the crew's workplace and home.
2. Establish Cutter One as one industrial workboat among Meridian's craft.
3. Show that this is an experienced crew trusted with difficult belt work.
4. Brief three recoveries, including one unusual unmanifested object.
5. Establish a departure deadline.
6. Hand the helm to the player without making spoken dialogue read like a key
   binding tutorial.

The camera starts in a safe conversation hold and establishes Cutter One beside
Meridian. It returns to the player before the work mark activates.

### Dialogue draft

**MERIDIAN CONTROL**

"Cutter One, Meridian Control. Bay is clear. You are released."

**COPILOT**

"Clamps are open. Drive and thrusters read green."

**DECK CHIEF**

"Plate Seven has three recovery crates waiting. Two manifested. The third
turned up after the break."

**ENGINEER**

"Turned up how?"

**MERIDIAN CONTROL**

"Loose in the debris. It is marked for recovery."

**ENGINEER**

"Mass?"

**MERIDIAN CONTROL**

"Unknown."

**ENGINEER**

"Of course it is."

**YOU**

"Cutter One copies. Three crates off Seven, one unknown."

**MERIDIAN CONTROL**

"Correct. Meridian gets under way in fifty-six minutes. Do not make me come
looking for you."

**COPILOT - CABIN CHANNEL**

"She says that like they would leave without us."

**ENGINEER**

"They would leave without you."

**COPILOT**

"Cruel."

**ENGINEER**

"Put us alongside. I will handle the crates."

Camera returns to the player.

**OBJECTIVE**

"Burn to the work mark."

**COPILOT**

"Mark ahead, seventeen hundred metres. Easy on the drive - we are still inside
Meridian's paint budget."

### Open dialogue questions

- Decide whether the Deck Chief and Meridian Control are separate people in
  this exchange. Too many external voices may weaken the crew introduction.
- Decide whether the third object is already known to be a crate. Calling it a
  tagged return, recovery object, or contact would preserve more uncertainty.
- Decide how the private cabin channel is presented in the speaker header.
- Confirm whether fifty-six minutes matches the later scenario pacing as an
  operational deadline rather than literal elapsed play time.

## RCS check rationale

An experienced crew should not fly four arbitrary training beacons. The route
works better as a post-maintenance handling check before Cutter One enters the
tight plate.

Accepted setup:

- Cutter One has just completed scheduled maintenance in which its port RCS
  manifold was replaced.
- Automated diagnostics prove that valves and controllers answer. They do not
  prove the cutter's real translation under its current mass, thrust balance,
  and human helm input.
- The four-mark box checks lateral and vertical translation in both directions:
  right, up, left, and down.
- STOP must first remove main-drive motion so the crew can distinguish drift
  from bad trim.
- The later GOTO legs close out the repaired cutter's guidance and automatic
  braking checks while also carrying the crew toward real work.
- ORBIT is not on the handling card. Once GOTO behaves correctly, the crew uses
  the planetoid as an unauthorized chance to test whether the integrated
  guidance and RCS hold a ring under gravity. This gives the "donut" both a
  technical excuse and a playful motive.
- The senior team is assigned because Plate Seven is difficult work and the
  third recovery is poorly characterized. The check is due to the ship's
  maintenance state, not the crew's inexperience.

The copilot's caution comes from Prospector Six, a cutter operated by another
company. Its computer reported green before a port manifold locked open in the
belt. News first blamed pilot error; the recovered flight recorder contradicted
that account. Nobody aboard survived. This is background industrial history,
not a mystery tied to First Shift's third recovery.

The scene 2 conversation is now production dialogue. It starts ambiguously with
the copilot asking for a full stop and a handling check, reveals the scheduled
repair and Prospector Six during the wide four-mark shot, and returns control
with explicit Shift-plus-mouse guidance. The objective and control dock teach
the input; the crew explains why the maneuver matters.
