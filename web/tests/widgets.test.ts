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
    weaveFade,
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
