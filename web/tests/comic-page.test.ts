import { strict as assert } from "node:assert";
import { balloonOutline, wrapDialogue } from "../src/comics/comic-lettering";

assert.deepEqual(
    wrapDialogue("Keep the protective covers on.", 20, (text) => text.length),
    ["Keep the protective", "covers on."]
);
assert.deepEqual(
    wrapDialogue("Keep the\ncovers on.", 40, (text) => text.length),
    ["Keep the", "covers on."]
);
assert.deepEqual(
    wrapDialogue("unsplittable", 4, (text) => text.length),
    ["unsplittable"],
    "An oversized word remains intact for the overflow diagnostic"
);
import {
    ComicNode,
    SvgNode,
    bleedPage,
    comicPage,
    grid,
    group,
    hud,
    narration,
    panel,
    speech,
    svgText,
    terminal,
    transcript,
    widePanel,
} from "../src/comics/comic-page";
import { ShipKind, debris, ship, starfield } from "../src/comics/comic-art";

function node<K extends ComicNode["kind"]>(
    value: ComicNode,
    kind: K
): Extract<ComicNode, { kind: K }> {
    assert.equal(value.kind, kind, `node is a ${kind}`);
    return value as Extract<ComicNode, { kind: K }>;
}

function shape<K extends SvgNode["kind"]>(
    value: SvgNode,
    kind: K
): Extract<SvgNode, { kind: K }> {
    assert.equal(value.kind, kind, `shape is a ${kind}`);
    return value as Extract<SvgNode, { kind: K }>;
}

// Speech and narration default to the corners a reader expects.
{
    const plain = node(speech("Pilot", "Copy."), "speech");
    assert.equal(plain.at, "bottom-right", "speech sits bottom right");
    assert.equal(plain.tone, "default");
    assert.equal(plain.channel, undefined, "no channel unless given");
    const tagged = node(
        speech("Control", "Nobody.", {
            tone: "amber",
            at: "top-left",
            channel: "rec",
        }),
        "speech"
    );
    assert.equal(tagged.at, "top-left");
    assert.equal(tagged.tone, "amber");
    assert.equal(tagged.channel, "rec");
    assert.equal(node(narration("Later."), "narration").at, "top-left");
}

// Grid and panel defaults.
{
    const layout = node(
        grid({}, panel({}), widePanel({ variant: "dark" })),
        "grid"
    );
    assert.equal(layout.columns, 2, "two columns by default");
    assert.deepEqual(layout.rows, [2, 1], "a tall row over a short row");
    const first = node(layout.children[0], "panel");
    assert.equal(first.span, 1);
    assert.equal(first.rowSpan, 1);
    assert.equal(first.variant, "default");
    assert.equal(first.frame, "default");
    const wide = node(layout.children[1], "panel");
    assert.equal(
        wide.span,
        Number.POSITIVE_INFINITY,
        "a wide panel spans every column"
    );
    assert.equal(wide.variant, "dark", "a wide panel keeps its variant");
}

// Tuple helpers map to typed records.
{
    const lines = node(
        transcript(
            [
                ["Control", "Hold."],
                ["Captain", "Holding.", "amber"],
            ],
            { title: "Open channel" }
        ),
        "transcript"
    );
    assert.equal(lines.title, "Open channel");
    assert.deepEqual(lines.lines, [
        { speaker: "Control", text: "Hold.", tone: "default" },
        { speaker: "Captain", text: "Holding.", tone: "amber" },
    ]);
    const chips = node(
        hud([
            ["RANGE", "1200 m"],
            ["FUEL", "12%", "danger"],
        ]),
        "hud"
    );
    assert.equal(chips.at, "top-right", "hud chips sit top right");
    assert.deepEqual(chips.items, [
        { label: "RANGE", value: "1200 m", tone: "default" },
        { label: "FUEL", value: "12%", tone: "danger" },
    ]);
    const prompt = node(
        terminal("Choose.", [
            ["1", "Hold", "Wait for the navy."],
            ["2", "Go", "Break the hold.", "amber"],
        ]),
        "terminal"
    );
    assert.equal(prompt.prompt, "Choose.");
    assert.deepEqual(prompt.options, [
        {
            key: "1",
            label: "Hold",
            detail: "Wait for the navy.",
            tone: "default",
        },
        { key: "2", label: "Go", detail: "Break the hold.", tone: "amber" },
    ]);
}

// Page layouts.
{
    const standard = comicPage(narration("x"));
    assert.equal(standard.layout, "standard");
    const bleed = bleedPage({ image: "board.svg", alt: "A board." });
    assert.equal(bleed.layout, "bleed");
    if (bleed.layout === "bleed") {
        assert.equal(bleed.focus, "center", "bleed images centre by default");
        assert.equal(bleed.image, "board.svg");
    }
}

// Inline SVG helper defaults.
{
    const label = shape(svgText("MAST", { at: [10, 20] }), "svgText");
    assert.equal(label.size, 14);
    assert.equal(label.anchor, "middle");
    assert.equal(label.spacing, 0);
    const wrapper = shape(group({}), "group");
    assert.deepEqual(wrapper.translate, [0, 0]);
    assert.equal(wrapper.rotate, 0);
    assert.equal(wrapper.scale, 1);
    assert.equal(wrapper.opacity, 1);
}

// Every ship kind and faction mark renders as a placed group with shapes.
{
    const kinds: ShipKind[] = [
        "cutter",
        "tender",
        "carrier",
        "corvette",
        "warship",
        "picket",
        "claw",
        "skiff",
        "tug",
        "leader",
        "boat",
    ];
    for (const kind of kinds) {
        const hull = shape(ship(kind, { at: [10, 20], rotate: 15 }), "group");
        assert.deepEqual(hull.translate, [10, 20], `${kind} is placed`);
        assert.equal(hull.rotate, 15, `${kind} is rotated`);
        assert.ok(hull.children.length > 0, `${kind} has a hull`);
    }
}

// Scattered art is deterministic per seed.
{
    const first = debris({ center: [100, 100], radius: 40 });
    const second = debris({ center: [100, 100], radius: 40 });
    assert.equal(first.length, 14, "fourteen pieces by default");
    assert.deepEqual(first, second, "the same seed scatters the same way");
    assert.notDeepEqual(
        first,
        debris({ center: [100, 100], radius: 40, seed: 99 }),
        "a new seed scatters differently"
    );
    const stars = starfield({ size: [400, 300] });
    assert.equal(stars.length, 24, "twenty-four stars by default");
    assert.deepEqual(stars, starfield({ size: [400, 300] }));
    for (const star of stars) {
        const dot = shape(star, "circle");
        assert.ok(dot.fill && dot.width === 0, "stars are solid dots");
        assert.ok(
            dot.center[0] >= 0 &&
                dot.center[0] <= 400 &&
                dot.center[1] >= 0 &&
                dot.center[1] <= 300,
            "stars stay inside the box"
        );
    }
}

// A tailed balloon's outline never doubles back on itself.
//
// The failure this pins does not throw and does not look wrong in any data
// assertion: when a tail's notch is wider than the straight run it cuts into,
// the path reverses and the stroke draws a spur across the balloon. A balloon
// is `42 + 27 * lines` tall, so the SHORT ones are exactly the ones at risk,
// and one- and two-line balloons are most of what an episode letters.
{
    // The outline is drawn clockwise from the top-left corner, one corner
    // quadratic per turn. So the straight runs between turn N and turn N+1
    // belong to one edge, and each edge has one direction it may travel:
    // right along the top, down the right, left along the bottom, up the
    // left. A tail's `L` excursions to the tip are not part of the edge and
    // are skipped; what must hold is that the edge itself never backs up.
    const EDGE = [
        { axis: "H", sign: 1, name: "top" },
        { axis: "V", sign: 1, name: "right" },
        { axis: "H", sign: -1, name: "bottom" },
        { axis: "V", sign: -1, name: "left" },
    ] as const;

    function edgeFaults(d: string): string[] {
        const steps = d.match(/[MHVLQZ][^MHVLQZ]*/g) ?? [];
        let [x, y] = [0, 0];
        let edge = 0;
        const faults: string[] = [];
        for (const step of steps) {
            const command = step[0];
            const numbers = (step.slice(1).match(/-?\d+(?:\.\d+)?/g) ?? []).map(
                Number
            );
            if (command === "M" || command === "L") {
                [x, y] = [numbers[0], numbers[1]];
            } else if (command === "Q") {
                [x, y] = [numbers[2], numbers[3]];
                edge += 1;
            } else if (command === "H" || command === "V") {
                const here = EDGE[Math.min(edge, EDGE.length - 1)];
                const from = command === "H" ? x : y;
                const to = numbers[0];
                if (command === "H") x = to;
                else y = to;
                if (here.axis !== command) continue;
                if ((to - from) * here.sign < -1e-9)
                    faults.push(
                        `the ${here.name} edge runs ${from} -> ${to}, backwards`
                    );
            }
        }
        return faults;
    }

    for (const side of ["top", "right", "bottom", "left"] as const) {
        for (const lines of [1, 2, 3, 4]) {
            const h = 42 + 27 * lines;
            const w = 260;
            const tip: [number, number] =
                side === "left"
                    ? [-40, h / 2]
                    : side === "right"
                      ? [w + 40, h / 2]
                      : side === "top"
                        ? [120, -40]
                        : [120, h + 40];
            const faults = edgeFaults(balloonOutline(0, 0, w, h, side, tip).d);
            assert.deepEqual(
                faults,
                [],
                `a ${side} tail on a ${lines}-line balloon: ${faults.join("; ")}`
            );
        }
    }

    // A tail near a corner is clamped along the edge, not allowed off it.
    for (const tip of [
        [120, -400],
        [-400, 4],
        [900, 4],
    ] as [number, number][]) {
        for (const side of ["top", "right", "bottom", "left"] as const)
            assert.deepEqual(
                edgeFaults(balloonOutline(0, 0, 260, 69, side, tip).d),
                [],
                `a ${side} tail aimed at ${tip.join(",")} stays on its edge`
            );
    }
}

// eslint-disable-next-line no-console
console.log("comic-page.test.ts: all assertions passed");
