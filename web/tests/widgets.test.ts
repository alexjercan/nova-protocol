// The corridor scope's walk against what the game measured, and the unit
// contract the rest of the widgets rest on. Every expected number in the
// corridor block is examples/systems/system_railgun_lance.rs's stand bank:
// 200 hp reinforced cells on the build lattice, one 1800-power slug at
// 15 000 m/s down the centre, and the cells-per-layer profile the probe
// recorded for each stand. Run with `npm test`.
import { strict as assert } from "node:assert";
import {
    engineKilometers,
    engineMeters,
    engineMetersPerSec,
    engineMetersPerSec2,
    GUNSHIP_ARM_U,
    GUNSHIP_CELLS,
    GUNSHIP_COMPUTERS,
    GUNSHIP_MATES,
    hullState,
    kilometers,
    kineticDamageMultiplier,
    lanceCorridor,
    LANCE_RAKE_RADIUS_CELLS,
    meters,
    metersPerSec,
    metersPerSec2,
    METERS_PER_UNIT,
    reachLadder,
    severedParts,
    structuralCeiling,
    wavePath,
    wavePeaks,
    weaveFade,
    arrivalPark,
    ASTEROID_KINDS,
    asteroidKindFromMix,
    budgetedRcsDeltaV,
    CARRIER_CELLS,
    collapseBudget,
    COMMAND_CLASSES,
    COMMAND_ROWS,
    commandAllowed,
    CUTTER_CELLS,
    distanceAttenuation,
    GRAMMAR_ZONES,
    halfColumn,
    hullRadiusMeters,
    panCompensation,
    panGains,
    railgunRecoil,
    rcsPush,
    seededDraws,
    segmentClearance,
    soundAtEars,
    soundCueUrl,
    zoneAllows,
    ZONE_PARTS,
    zonePlacements,
} from "../src/widgets";

const HP = 200;

// ---- units ----------------------------------------------------------------

// The plain formatters take SI, because the catalog is authored in SI. They
// must NOT apply the engine scale to what they are handed.
{
    assert.equal(meters(300), "300 m");
    assert.equal(meters(999), "999 m");
    assert.equal(meters(1000), "1 km", "a kilometer up, the reading is km");
    assert.equal(meters(2000), "2 km");
    assert.equal(kilometers(18000), "18 km");
    assert.equal(metersPerSec(1000), "1,000 m/s");
    assert.equal(metersPerSec2(78.48, 0), "78 m/s^2");
}

// The engine formatters are the only place the world-unit scale is applied,
// and one world unit is ten meters.
{
    assert.equal(METERS_PER_UNIT, 10);
    assert.equal(engineMeters(30), "300 m", "30 u is a 300 m blast radius");
    assert.equal(engineMeters(5.52, 1), "55.2 m", "the gunship's arm");
    assert.equal(engineKilometers(328.6, 1), "3.3 km", "a planetoid SOI");
    assert.equal(engineMetersPerSec(100), "1,000 m/s", "the reference speed");
    assert.equal(engineMetersPerSec2(64, 0), "640 m/s^2", "one bare drive");
}

// The attitude model's one crossing: the arm arrives in world units off the
// collider boxes and the 8 G limit is SI, so 5.52 u is 55.2 m and the ceiling
// is 78.48 / 55.2 - the same rad/s^2 the game reads.
{
    const ceiling = structuralCeiling(GUNSHIP_ARM_U);
    assert.ok(
        Math.abs(ceiling - (8 * 9.81) / 55.2) < 1e-9,
        `the gunship's structural ceiling, got ${ceiling}`
    );
    assert.ok(
        Math.abs(ceiling - 1.422) < 5e-3,
        `~1.42 rad/s^2 as the widget prints it, got ${ceiling}`
    );
    assert.equal(structuralCeiling(0), Infinity, "a point mass has no arm");
}

// ---- the Patrol Gunship ---------------------------------------------------

// `GUNSHIP_ARM_U` is a constant because the GOTO scope needs it before the cell
// table exists. It has to be the arm that table derives, or the two widgets
// fly different ships.
{
    const state = hullState(GUNSHIP_CELLS);
    assert.ok(
        Math.abs(state.arm - GUNSHIP_ARM_U) < 5e-3,
        `the constant against the derived arm, got ${state.arm}`
    );
    // assets/base/ships/base.content.ron, block_gunship: 53 sections and the
    // volume of their boxes, with nothing anywhere authoring a mass.
    assert.equal(GUNSHIP_CELLS.length, 53, "block_gunship's section count");
    assert.ok(
        Math.abs(state.mass - 64.75) < 1e-9,
        `block_gunship's mass, got ${state.mass}`
    );
}

// The structural graph is derived off the grid rather than authored, so every
// cell has to reach a flight computer on the intact hull - a cell that mates
// to nothing would silently sever the moment the widget is touched.
{
    const whole = severedParts(
        GUNSHIP_CELLS,
        GUNSHIP_MATES,
        new Set<string>(),
        GUNSHIP_COMPUTERS
    );
    assert.equal(whole.held.length, 53, "the intact hull is one body");
    assert.equal(whole.adrift.length, 0, "and nothing is adrift on it");
}

// Losing one of the two computers keeps the ship; losing both ends it, and
// every section left is a wreck rather than a hull with a long arm.
{
    const one = severedParts(
        GUNSHIP_CELLS,
        GUNSHIP_MATES,
        new Set(["bridge"]),
        GUNSHIP_COMPUTERS
    );
    assert.equal(one.held.length, 52, "the aft computer still flies it");
    const none = severedParts(
        GUNSHIP_CELLS,
        GUNSHIP_MATES,
        new Set(GUNSHIP_COMPUTERS),
        GUNSHIP_COMPUTERS
    );
    assert.equal(none.held.length, 0, "no computer, no ship");
    assert.equal(none.adrift.length, 51, "all of it is adrift");
}

// Shooting the bow spur off shortens the arm and RAISES the turn ceiling,
// which is the whole claim the controller-arm widget makes.
{
    const bow = GUNSHIP_CELLS.filter((part) => part.group === "bow");
    const cut = severedParts(
        GUNSHIP_CELLS,
        GUNSHIP_MATES,
        new Set(bow.map((part) => part.id)),
        GUNSHIP_COMPUTERS
    );
    assert.equal(cut.adrift.length, 0, "the bow is the last thing out there");
    assert.ok(
        hullState(cut.held).arm < GUNSHIP_ARM_U,
        "the arm shortens without the bow"
    );
}

// The damage curves stay in world units per second, because damage.rs does:
// the reference speed reads exactly 1.0, and it is the same speed as the PDC's
// authored 1 000 m/s muzzle.
{
    assert.equal(kineticDamageMultiplier(100), 1);
    assert.equal(engineMetersPerSec(100), metersPerSec(1000));
    assert.equal(kineticDamageMultiplier(200), 2, "the head-on ceiling");
    assert.equal(kineticDamageMultiplier(10), 0.25, "the stern-chase floor");
}

// The weave taper is a ratio of blast radii, so it moves with the authored
// 300 m warhead: full three radii out, gone half a radius out.
{
    assert.equal(weaveFade(1000, 300), 1, "full weave at range");
    assert.equal(weaveFade(900, 300), 1, "full weave at three radii");
    assert.equal(weaveFade(150, 300), 0, "none at the terminal band");
    assert.equal(weaveFade(525, 300), 0.5, "half way between the two");
}

// ---- corridor -------------------------------------------------------------

// The rake is authored as 10 m and the lattice counts cells, so the shipped
// radius is exactly one cell.
assert.equal(LANCE_RAKE_RADIUS_CELLS, 1, "10 m of rake is one build cell");

// The shipped radius against the 5 x 5 x 4 wall: nine cells a layer, and a
// 28th crossing on the far side because the f32 budget walk lands it.
{
    const wall = lanceCorridor(LANCE_RAKE_RADIUS_CELLS, HP, 5, 5, 4);
    assert.deepEqual(wall.profile, [9, 9, 9, 1], "raked_wall profile");
    assert.equal(wall.taken, 28, "raked_wall cells");
    assert.equal(wall.removed, 5600, "raked_wall removed");
}

// A four-cell (40 m) blast seed against the same wall spends the same budget
// on the entry face and stops one layer in.
{
    const wide = lanceCorridor(4.0, HP, 5, 5, 4);
    assert.deepEqual(wide.profile, [25, 3, 0, 0], "wide_wall profile");
    assert.equal(wide.taken, 28, "wide_wall cells");
    assert.equal(wide.removed, 5600, "wider is not more");
}

// A hull line three across and one tall: the needle takes the column,
// the shipped rake takes the pods beside it, and neither binds the budget.
{
    const needle = lanceCorridor(0, HP, 3, 1, 4);
    assert.deepEqual(needle.profile, [1, 1, 1, 1], "narrow_line profile");
    assert.equal(needle.taken, 4);
    const raked = lanceCorridor(LANCE_RAKE_RADIUS_CELLS, HP, 3, 1, 4);
    assert.deepEqual(raked.profile, [3, 3, 3, 3], "raked_line profile");
    assert.equal(raked.removed, 3 * needle.removed, "three times the needle");
    assert.ok(raked.spent < 1800, "the line never binds the budget");
}

// A needle against the wall is the same four cells: the width is the rake's.
assert.deepEqual(lanceCorridor(0, HP, 5, 5, 4).profile, [1, 1, 1, 1]);

// The sphere never reaches ahead of the tip: nothing beside the bore is
// reached before the tip has entered that cell's layer, and the second ring
// of the build lattice sits outside the shipped radius.
{
    const wall = lanceCorridor(LANCE_RAKE_RADIUS_CELLS, HP, 5, 5, 4);
    for (const cell of wall.cells) {
        if (cell.offset === 0 || cell.reach === Infinity) continue;
        assert.ok(cell.reach > cell.layer, "reached from behind the tip");
        assert.ok(cell.offset <= LANCE_RAKE_RADIUS_CELLS, "inside the radius");
    }
    assert.equal(
        wall.cells.filter((c) => c.reach !== Infinity).length,
        36,
        "the nine-cell footprint over four layers"
    );
}

// A drive is hit, not deleted: 300 into a 480 hp cell removes 300 of it.
{
    const drive = lanceCorridor(LANCE_RAKE_RADIUS_CELLS, 480, 1, 1, 1);
    assert.equal(drive.taken, 1);
    assert.equal(drive.removed, 300);
}

// ---- the engagement ladder ------------------------------------------------

// Every reach is a METER figure derived from authored speeds and lifetimes:
// 1 000 m/s over 2 s of PDC round, 15 000 m/s over 1.2 s of slug, and the
// harness's along-the-line torpedo speeds over the bay's 100 s.
{
    const rungs = reachLadder(0);
    assert.equal(rungs[0].reach, 2000, "PDC reach, 1 000 m/s x 2 s");
    assert.equal(rungs[1].reach, 18000, "lance reach, 15 000 m/s x 1.2 s");
    assert.equal(rungs[2].reach, 29140, "Serpent reach");
    assert.equal(rungs[3].reach, 31300, "Lance torpedo reach");
}

// The ladder: a target at 10 km is inside the lance and outside the PDC, and
// the slug's flight there is under a second.
{
    const rungs = reachLadder(10000);
    assert.equal(rungs[0].flightSecs, Infinity, "PDC out of reach");
    assert.ok(rungs[1].flightSecs < 1, "lance arrives inside a second");
    assert.ok(rungs[2].flightSecs > 30, "a Serpent takes half a minute");
    assert.equal(reachLadder(2000)[0].flightSecs, 2, "PDC at its reach");
    assert.equal(reachLadder(18001)[1].flightSecs, Infinity, "past the lance");
}

// eslint-disable-next-line no-console
console.log("widgets: the corridor scope reproduces the stand bank");

// ---- sound board ----------------------------------------------------------

// The waveform is a peak envelope normalised to the cue's loudest sample, so
// a quiet cue still fills its row and a silent one draws a hairline.
{
    const samples = new Float32Array([0, 0.5, -0.25, 0, 0, -1, 0.125, 0]);
    assert.deepEqual(
        wavePeaks(samples, 4),
        [0.5, 0.25, 1, 0.125],
        "each bin is the loudest absolute sample in it, over the loudest overall"
    );
    assert.deepEqual(wavePeaks(new Float32Array(0), 4), []);
    assert.deepEqual(
        wavePeaks(new Float32Array([0, 0, 0, 0]), 2),
        [0, 0],
        "silence does not divide by zero"
    );
    assert.equal(
        wavePath([], 240, 36),
        "M0,18 L240,18",
        "an undecoded cue is a hairline"
    );
    const d = wavePath([1, 0], 20, 10);
    assert.ok(
        d.startsWith("M0,5 L0.0,0.0"),
        `full peak reaches the top edge: ${d}`
    );
    assert.ok(d.endsWith("Z"), "the envelope is closed");
    assert.ok(
        d.includes("L10.0,4.4"),
        `a zero peak keeps a 0.6 hairline: ${d}`
    );
}

// A cue's v0.12.0 recording lives beside the board's copy, under one
// subdirectory, so the two keys on a row differ by path alone.
{
    assert.equal(
        soundCueUrl("/", "impact", false),
        "/assets/sounds/impact.wav"
    );
    assert.equal(
        soundCueUrl("/nova/", "impact", true),
        "/nova/assets/sounds/v0120/impact.wav"
    );
}

// ---- v0.13.0: the news post's scopes --------------------------------------

// Recoil: the impulse lands at the muzzle, so a lance on the axis only
// pushes, and one off the axis yaws the bow toward its own side.
{
    const onAxis = railgunRecoil(0);
    assert.ok(Math.abs(onAxis.yaw) < 1e-9, `no yaw on the axis: ${onAxis.yaw}`);
    assert.ok(onAxis.push > 0, "the shove is aft");
    assert.ok(railgunRecoil(2).yaw > 0, "a starboard mount yaws to starboard");
    assert.ok(railgunRecoil(-2).yaw < 0, "a port mount yaws to port");
    assert.ok(
        Math.abs(railgunRecoil(3).yaw) > Math.abs(railgunRecoil(1).yaw),
        "a wider offset yaws harder"
    );
    const heavy = railgunRecoil(2, 2);
    const light = railgunRecoil(2, 1);
    assert.ok(
        Math.abs(heavy.push * 2 - light.push) < 1e-9,
        "twice the mass halves the push"
    );
    assert.ok(
        Math.abs(heavy.yaw * 2 - light.yaw) < 1e-9,
        "twice the mass halves the yaw"
    );
    assert.equal(
        heavy.leverArm,
        light.leverArm,
        "the arm is geometry, not mass"
    );
}

// Occlusion: one solid ray from the scanner to the contact, blocked by a
// rock whose surface the SEGMENT crosses.
{
    assert.ok(
        segmentClearance(0, 0, 4000, 0, 2000, 0, 500) <= 0,
        "a rock on the line blocks"
    );
    assert.ok(
        segmentClearance(0, 0, 4000, 0, 2000, 3000, 500) > 0,
        "a rock well across does not"
    );
    assert.ok(
        segmentClearance(0, 0, 4000, 0, 6000, 0, 500) > 0,
        "a rock beyond the contact does not"
    );
    assert.ok(
        segmentClearance(0, 0, 4000, 0, 0, 0, 500) <= 0,
        "a scanner inside a rock sees nothing"
    );
    assert.equal(
        segmentClearance(0, 0, 4000, 0, 2000, 700, 500),
        200,
        "clearance is distance minus radius"
    );
}

// Zones: thirds along the hull, above or below the keel row, the outboard
// half across; a placement is legal only if every cell passes.
{
    assert.equal(GRAMMAR_ZONES.length, 6);
    assert.ok(
        zoneAllows("Bow", 0, 0, 3) && !zoneAllows("Bow", 0, 0, 4),
        "Bow is z 0..3"
    );
    assert.ok(
        zoneAllows("Amidships", 0, 0, 4) &&
            zoneAllows("Amidships", 0, 0, 7) &&
            !zoneAllows("Amidships", 0, 0, 8),
        "Amidships is z 4..7"
    );
    assert.ok(
        zoneAllows("Stern", 0, 0, 8) && !zoneAllows("Stern", 0, 0, 7),
        "Stern is z 8..10"
    );
    assert.ok(
        zoneAllows("Dorsal", 0, 3, 0) && !zoneAllows("Dorsal", 0, 2, 0),
        "Dorsal is above the keel row"
    );
    assert.ok(
        zoneAllows("Ventral", 0, 1, 0) && !zoneAllows("Ventral", 0, 2, 0),
        "Ventral is below the keel row"
    );
    assert.ok(
        zoneAllows("Flank", 2, 0, 0) && !zoneAllows("Flank", 1, 0, 0),
        "Flank is the outboard two columns"
    );
    assert.deepEqual(
        [0, 3, 4, 7].map(halfColumn),
        [3, 0, 0, 3],
        "columns mirror about the centreline"
    );
    assert.equal(ZONE_PARTS.length, 5);
    const free = zonePlacements([], [1, 1, 1]);
    assert.equal(free.cells, 220, "the STARBOARD HALF is 4 x 5 x 11 cells");
    assert.equal(free.placements, 220);
    assert.equal(
        zonePlacements(["Bow", "Stern"], [1, 1, 1]).placements,
        0,
        "two thirds never overlap"
    );
    // The headline the widget used to get wrong: the collapse runs on the
    // half, so a part wider than the half has nowhere to stand at ALL - not
    // 36 places on an 8-wide ship it is never judged against.
    assert.equal(
        zonePlacements([], [5, 5, 3]).placements,
        0,
        "a 5-wide capital drive does not fit the 4-wide half, zone or none"
    );
    assert.equal(
        zonePlacements(["Dorsal"], [3, 3, 2]).placements,
        0,
        "a vector drive does not fit in the two rows above the keel"
    );
    assert.equal(
        zonePlacements([], [3, 3, 2]).placements,
        60,
        "a vector drive has 2 x 3 x 10 places on the unzoned half"
    );
    const lance = zonePlacements(["Bow"], [1, 1, 3]);
    assert.equal(
        lance.placements,
        40,
        "a lance has 4 x 5 x 2 places in the bow of the half"
    );
    assert.ok(
        lance.lit[0][0][3] && !lance.lit[0][0][4],
        "lit cells stop at the third"
    );
}

// Kinds: the weighted draw, exactly as the scatter does it.
{
    assert.equal(ASTEROID_KINDS.length, 5);
    const mix: [string, number][] = [
        ["rock", 6],
        ["ice", 3],
        ["metal", 1],
    ];
    assert.equal(asteroidKindFromMix(mix, 0), "rock");
    assert.equal(asteroidKindFromMix(mix, 0.59), "rock");
    assert.equal(asteroidKindFromMix(mix, 0.61), "ice");
    assert.equal(asteroidKindFromMix(mix, 0.89), "ice");
    assert.equal(asteroidKindFromMix(mix, 0.91), "metal");
    assert.equal(
        asteroidKindFromMix(mix, 1),
        "metal",
        "a draw of 1 is the last ticket"
    );
    assert.equal(asteroidKindFromMix(mix, 2), "metal", "the draw is clamped");
    assert.equal(asteroidKindFromMix(mix, -1), "rock");
    assert.equal(
        asteroidKindFromMix([["rock", 0]], 0.5),
        null,
        "no weight picks nothing"
    );
    const a = seededDraws(7, 5);
    assert.deepEqual(a, seededDraws(7, 5), "a seed is a belt");
    assert.ok(
        a.every((d) => d >= 0 && d < 1),
        "draws lie in [0, 1)"
    );
    assert.notDeepEqual(a, seededDraws(8, 5));
}

// Sound: the rolloff band, the pan law and its compensation, the routes.
{
    assert.equal(distanceAttenuation(0), 1);
    assert.equal(distanceAttenuation(20), 1, "full inside NEAR");
    assert.equal(distanceAttenuation(320), 0, "silent at FAR");
    assert.equal(distanceAttenuation(420), 0);
    assert.ok(
        distanceAttenuation(100) > distanceAttenuation(200),
        "monotone between"
    );
    assert.ok(
        distanceAttenuation(319) > 0 && distanceAttenuation(319) < 0.01,
        "reaches zero, not the floor"
    );
    const [l, r] = panGains(-1, 0, 0);
    assert.ok(l > 2 * r, `hard to port favours the left ear: ${l} vs ${r}`);
    const ahead = panGains(0, 0, -1);
    assert.ok(Math.abs(ahead[0] - ahead[1]) < 1e-9, "dead ahead is centred");
    const c = panCompensation(-1, 0, 0);
    const rms = Math.sqrt(((l * c) ** 2 + (r * c) ** 2) / 2);
    assert.ok(
        Math.abs(rms - 1) < 1e-9,
        `compensation restores unit RMS: ${rms}`
    );
    assert.deepEqual(
        soundAtEars("Hull", 5000, 90),
        { left: 1, right: 1, level: 1 },
        "Hull is never attenuated or panned"
    );
    assert.deepEqual(soundAtEars("Interface", 5000, 90), {
        left: 1,
        right: 1,
        level: 1,
    });
    const world = soundAtEars("Exterior", 3500, 0);
    assert.equal(world.level, 0, "past the far ring the world is silent");
    const abeam = soundAtEars("Exterior", 0, 90);
    assert.ok(abeam.right > abeam.left, "a source to starboard leans right");
}

// RCS: free braking, full push inside the sphere, a turn on its surface.
{
    assert.deepEqual(
        budgetedRcsDeltaV([0, 0], [1, 0], 100, 20),
        [1, 0],
        "inside the sphere the push lands whole"
    );
    assert.deepEqual(
        budgetedRcsDeltaV([100, 0], [-1, 0], 100, 20),
        [-1, 0],
        "braking is free at the cap"
    );
    const turn = budgetedRcsDeltaV([100, 0], [0, 1], 100, 20);
    const held = Math.hypot(100 + turn[0], turn[1]);
    assert.ok(
        Math.abs(held - 100) < 1e-6,
        `the sphere holds the speed: ${held}`
    );
    assert.ok(turn[1] > 0.9, "and the push turns the vector");
    const rest = rcsPush(0, 0, 0, 1);
    assert.ok(
        Math.abs(Math.hypot(...rest.after) - 49.05) < 1e-6,
        "one second from rest is 49.05 m/s"
    );
    assert.ok(!rest.clamped && !rest.tapered && !rest.free);
    const capped = rcsPush(100, 0, 90, 1);
    assert.ok(capped.clamped, "pushing across at the cap is clamped");
    assert.ok(
        Math.abs(Math.hypot(...capped.after) - 100) < 1e-6,
        "and stays at the cap"
    );
    const braking = rcsPush(100, 0, 180, 1);
    assert.ok(braking.free, "pushing back is free");
    assert.ok(Math.abs(Math.hypot(...braking.after) - 50.95) < 1e-6);
}

// Arrival: hull radii from the cell plans, and the park rule.
{
    const gunship = hullRadiusMeters(GUNSHIP_CELLS);
    assert.ok(
        Math.abs(gunship - GUNSHIP_ARM_U * METERS_PER_UNIT) < 0.1,
        `the gunship's radius is its pinned structural arm: ${gunship}`
    );
    const cutter = hullRadiusMeters(CUTTER_CELLS);
    const carrier = hullRadiusMeters(CARRIER_CELLS);
    assert.ok(
        cutter < gunship && gunship < carrier,
        `cutter ${cutter} < gunship ${gunship} < carrier ${carrier}`
    );
    assert.equal(CUTTER_CELLS.length, 26, "the cutter's plan is 26 cells");
    assert.ok(
        CARRIER_CELLS.length > 800,
        `the carrier's plan is hundreds of cells: ${CARRIER_CELLS.length}`
    );
    const park = arrivalPark(950, 100, 500);
    assert.equal(park.centreDistance, 1550);
    assert.equal(park.gap, 500, "the margin is the gap");
    assert.equal(park.oldCentreDistance, 500);
    assert.equal(park.oldGap, -550, "the old rule parked inside the planetoid");
}

// Commands: 27 rows, four classes, and one gate.
{
    assert.equal(COMMAND_CLASSES.length, 4);
    assert.equal(COMMAND_ROWS.length, 27);
    const count = (cls: string): number =>
        COMMAND_ROWS.filter((row) => row.cls === cls).length;
    assert.deepEqual(
        [count("Utility"), count("ReadOnly"), count("Setting"), count("Cheat")],
        [5, 12, 5, 5]
    );
    assert.ok(commandAllowed("status", "ReadOnly", false));
    assert.ok(commandAllowed("graphics", "Setting", false));
    assert.ok(
        !commandAllowed("ammo refill", "Cheat", false),
        "a cheat is refused unarmed"
    );
    assert.ok(
        commandAllowed("cheats enable", "Cheat", false),
        "except the one that arms"
    );
    assert.ok(commandAllowed("ammo refill", "Cheat", true));
}

// Ceilings: the walk once, 128 chips, 24 pieces a frame.
{
    const b = collapseBudget(720, 600);
    assert.equal(b.walksOld, 720);
    assert.equal(b.walksNew, 1);
    assert.equal(b.chipsOld, 4200);
    assert.equal(b.chipsNew, 128);
    assert.equal(b.unchipped, 582);
    assert.equal(b.piecesOld, 720);
    assert.equal(b.piecesNew, 24);
    assert.equal(b.shedFrames, 30, "720 pieces take 30 frames");
    assert.ok(
        b.walkMsOld > 30 && b.walkMsNew < 1,
        `${b.walkMsOld} ms to ${b.walkMsNew} ms`
    );
    const small = collapseBudget(1, 1);
    assert.equal(small.chipsOld, 7);
    assert.equal(small.chipsNew, 7, "one crater is untouched");
    assert.equal(small.unchipped, 0);
    assert.equal(small.piecesNew, 1);
    assert.equal(small.shedFrames, 1);
}

// eslint-disable-next-line no-console
console.log("widgets: the v0.13.0 scopes follow the game's rules");
