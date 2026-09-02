// The corridor scope's walk against what the game measured. Every expected
// number here is examples/systems/system_railgun_lance.rs's stand bank: 200 hp
// reinforced cells on a unit lattice, one 1800-power slug at 1500 u/s down
// the centre, and the cells-per-layer profile the probe recorded for each
// stand. Run with `npm test`.
import { strict as assert } from "node:assert";
import { lanceCorridor, reachLadder } from "../src/widgets";

const HP = 200;

// The shipped radius against the 5 x 5 x 4 wall: nine cells a layer, and a
// 28th crossing on the far side because the f32 budget walk lands it.
{
    const wall = lanceCorridor(1.0, HP, 5, 5, 4);
    assert.deepEqual(wall.profile, [9, 9, 9, 1], "raked_wall profile");
    assert.equal(wall.taken, 28, "raked_wall cells");
    assert.equal(wall.removed, 5600, "raked_wall removed");
}

// The 4.0 blast seed against the same wall spends the same budget on the
// entry face and stops one layer in.
{
    const wide = lanceCorridor(4.0, HP, 5, 5, 4);
    assert.deepEqual(wide.profile, [25, 3, 0, 0], "wide_wall profile");
    assert.equal(wide.taken, 28, "wide_wall cells");
    assert.equal(wide.removed, 5600, "wider is not more");
}

// The corvette line, three across and one tall: the needle takes the column,
// the shipped rake takes the pods beside it, and neither binds the budget.
{
    const needle = lanceCorridor(0, HP, 3, 1, 4);
    assert.deepEqual(needle.profile, [1, 1, 1, 1], "narrow_line profile");
    assert.equal(needle.taken, 4);
    const raked = lanceCorridor(1.0, HP, 3, 1, 4);
    assert.deepEqual(raked.profile, [3, 3, 3, 3], "raked_line profile");
    assert.equal(raked.removed, 3 * needle.removed, "three times the needle");
    assert.ok(raked.spent < 1800, "the line never binds the budget");
}

// A needle against the wall is the same four cells: the width is the rake's.
assert.deepEqual(lanceCorridor(0, HP, 5, 5, 4).profile, [1, 1, 1, 1]);

// The sphere never reaches ahead of the tip: nothing beside the bore is
// reached before the tip has entered that cell's layer, and the second ring
// of a unit lattice sits outside the shipped radius.
{
    const wall = lanceCorridor(1.0, HP, 5, 5, 4);
    for (const cell of wall.cells) {
        if (cell.offset === 0 || cell.reach === Infinity) continue;
        assert.ok(cell.reach > cell.layer, "reached from behind the tip");
        assert.ok(cell.offset <= 1.0, "inside the radius");
    }
    assert.equal(
        wall.cells.filter((c) => c.reach !== Infinity).length,
        36,
        "the nine-cell footprint over four layers"
    );
}

// A drive is hit, not deleted: 300 into a 480 hp cell removes 300 of it.
{
    const drive = lanceCorridor(1.0, 480, 1, 1, 1);
    assert.equal(drive.taken, 1);
    assert.equal(drive.removed, 300);
}

// The ladder: a target at 1000 u is inside the lance and outside the PDC, and
// the slug's flight there is under a second.
{
    const rungs = reachLadder(1000);
    assert.equal(rungs[0].flightSecs, Infinity, "PDC out of reach");
    assert.ok(rungs[1].flightSecs < 1, "lance arrives inside a second");
    assert.ok(rungs[2].flightSecs > 30, "a Serpent takes half a minute");
    assert.equal(reachLadder(200)[0].flightSecs, 2, "PDC at its reach");
    assert.equal(reachLadder(1801)[1].flightSecs, Infinity, "past the lance");
}

// eslint-disable-next-line no-console
console.log("widgets: the corridor scope reproduces the stand bank");
