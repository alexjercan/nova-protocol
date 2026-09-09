const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const {
    validateScript,
    compileScript,
    scriptMarkdown,
    transcript,
} = require("../comic-script-build");
const { compileComics } = require("../comic-sources");
const panel = {
    id: "one",
    action: "A person waits by a blank display.",
    art: "cover",
    dialogue: [
        { id: "report", speaker: "Worker", text: "Everything is ready." },
    ],
    lettering: {
        report: {
            at: [20, 20],
            width: 300,
            tail: [200, 130],
            side: "bottom",
            breakAfter: [],
        },
    },
    labels: [],
    cards: [],
};
const script = {
    heading: "Fixture",
    footer: "Fixture only",
    pages: [
        {
            kind: "illustrated",
            id: "one",
            title: "A page",
            purpose: "Validate the shared engine.",
            panels: [panel],
            layout: { one: { at: [42, 86], size: [1416, 857] } },
        },
    ],
};
validateScript(script, 1);
for (const mutate of [
    (s) => delete s.pages[0].panels[0].action,
    (s) => (s.pages[0].panels[0].unknown = true),
    (s) =>
        (s.pages[0].panels[0].lettering.wrong =
            s.pages[0].panels[0].lettering.report),
    (s) => delete s.pages[0].panels[0].lettering.report,
    (s) => (s.pages[0].panels[0].lettering.report.breakAfter = [99]),
    (s) => s.pages[0].panels[0].dialogue.push({ ...panel.dialogue[0] }),
    (s) => (s.pages[0].layout.wrong = s.pages[0].layout.one),
    (s) => (s.pages[0].layout.one.at = [1400, 900]),
    (s) => (s.pages[0].panels[0].art = "../escape"),
]) {
    const invalid = structuredClone(script);
    mutate(invalid);
    assert.throws(() => validateScript(invalid, 1));
}
assert.throws(() => validateScript(script, 2), /page count/);
assert(transcript(script.pages[0]).includes("Worker: Everything is ready."));
assert(
    scriptMarkdown(script, "Fixture").includes(
        "**Worker:** Everything is ready."
    )
);
const silent = {
    kind: "script",
    id: "future",
    title: "Later",
    purpose: "A script can precede illustration.",
    panels: [
        { id: "future-panel", action: "FUTURE PRIVATE CANARY", dialogue: [] },
    ],
};
assert(transcript(silent).includes("No dialogue."));
const nonsequential = { ...script, pages: [silent, ...script.pages] };
validateScript(nonsequential, 2);

const temp = fs.mkdtempSync(path.join(os.tmpdir(), "comic-dsl-"));
const root = path.join(temp, "source"),
    output = path.join(temp, "generated");
function put(name, contents) {
    const file = path.join(root, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(
        file,
        typeof contents === "string" ? contents : JSON.stringify(contents)
    );
}
const episode = {
    title: "Selected fixture",
    summary: "Fixture only.",
    publication: "released",
    pageCount: 1,
    source: "episode.ts",
    art: ["art/scenes.py"],
    cover: "cover",
    coverAlt: "Fixture art.",
};
const art =
    "from nova_illustration.scenes import Scene\ndef unused():\n    raise Exception('Unreferenced scene must not be drawn')\nSCENES = {'cover': lambda: Scene(1416,857,'<rect width=\"1416\" height=\"857\" fill=\"#123456\"/>'), 'unused': unused}\n";
const compile = (includeDrafts = false) =>
    compileComics({ root, output, includeDrafts });
try {
    put("season-1/comic.json", {
        sequence: 1,
        title: "Fixture",
        summary: "Fixture only.",
        episodes: ["episode-1", "episode-2"],
    });
    put("season-1/episode-1/episode.json", episode);
    put(
        "season-1/episode-1/episode.ts",
        "export default " + JSON.stringify(script) + ";"
    );
    put("season-1/episode-1/art/scenes.py", art);
    put("season-1/episode-2/episode.json", {
        ...episode,
        publication: "draft",
    });
    put(
        "season-1/episode-2/episode.ts",
        "throw new Error('Excluded TS must not execute'); export default {}; "
    );
    put(
        "season-1/episode-2/art/scenes.py",
        "raise Exception('Excluded Python must not execute')\n"
    );
    const released = compile();
    assert.equal(released.assets.size, 1);
    assert.equal(released.comics[0].episodes.length, 1);
    assert(!JSON.stringify(released.comics).includes("episode.ts"));
    assert(!JSON.stringify(released.comics).includes("scenes.py"));
    const directory = path.join(output, "season-1/episode-1");
    const before = fs
        .readdirSync(directory)
        .map((name) => [
            name,
            fs.readFileSync(path.join(directory, name)).toString(),
            fs.statSync(path.join(directory, name)).mtimeMs,
        ]);
    compile();
    assert.deepEqual(
        fs
            .readdirSync(directory)
            .map((name) => [
                name,
                fs.readFileSync(path.join(directory, name)).toString(),
                fs.statSync(path.join(directory, name)).mtimeMs,
            ]),
        before,
        "Deterministic files preserve mtimes"
    );
    assert.throws(() => compile(true), /Excluded TS/);
    put(
        "season-1/episode-2/episode.ts",
        "export default " + JSON.stringify(script) + ";"
    );
    assert.throws(() => compile(true), /Excluded Python/);
    put("season-1/episode-2/art/scenes.py", art);
    put("season-1/episode-2/episode.json", {
        ...episode,
        publication: "draft",
        pageCount: 2,
    });
    put(
        "season-1/episode-2/episode.ts",
        "export default " +
            JSON.stringify({ ...script, pages: [...script.pages, silent] }) +
            ";"
    );
    const local = compile(true);
    assert.equal(local.comics[0].episodes[1].pages.length, 1);
    assert(!JSON.stringify(local.comics).includes("FUTURE PRIVATE CANARY"));
    assert(
        fs
            .readFileSync(
                path.join(output, "season-1/episode-2/SCRIPT.md"),
                "utf8"
            )
            .includes("FUTURE PRIVATE CANARY")
    );
    assert(![...local.assets.keys()].some((k) => /json|md|unused/.test(k)));
    const palette = JSON.parse(
        fs.readFileSync(path.join(directory, "palette.json"), "utf8")
    );
    assert.throws(() => compileScript(script, {}, palette), /Unknown scene/);
    const scenes = {
        cover: { file: "cover.svg", size: [1416, 857], slots: [] },
    };
    assert.equal(
        compileScript(nonsequential, scenes, palette)[0].definition.number,
        2
    );
    const labelled = structuredClone(script);
    labelled.pages[0].panels[0].labels = [
        {
            id: "screen",
            text: "READY",
            slot: "screen",
            at: [20, 180],
            size: 20,
            color: "mint",
            weight: "normal",
            spacing: 0,
            anchor: "start",
        },
    ];
    assert.throws(
        () => compileScript(labelled, scenes, palette),
        /Unknown or repeated text slot/
    );
    assert.throws(
        () =>
            compileScript(
                script,
                { cover: { ...scenes.cover, slots: ["unlettered"] } },
                palette
            ),
        /Unlettered/
    );
    scenes.cover.slots = ["screen"];
    assert.equal(
        compileScript(labelled, scenes, palette)[0].definition.panels[0]
            .labels[0].text,
        "READY"
    );
    labelled.pages[0].panels[0].labels[0].color = "missing";
    assert.throws(
        () => compileScript(labelled, scenes, palette),
        /Unknown label color/
    );
    compile();
    assert(!fs.existsSync(path.join(output, "season-1/episode-2")));
    put("season-1/episode-2/episode.json", { ...episode, pageCount: 2 });
    assert.throws(() => compile(), /not fully illustrated/);
} finally {
    fs.rmSync(temp, { recursive: true, force: true });
}
console.log(
    "Comic DSL validation, selected TS/Python execution, script derivation, and cache isolation passed."
);
