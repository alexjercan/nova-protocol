const assert = require("node:assert/strict");
const fs = require("node:fs");
const { spawnSync } = require("node:child_process");
const os = require("node:os");
const path = require("node:path");
const { discoverComics, compileComics } = require("../comic-sources");
const {
    comicIndexPage,
    comicSeasonPage,
    comicReaderPage,
    comicRoutes,
    releasedEpisodes,
} = require("../comic-build");
const { validatePageDefinition } = require("../comic-definitions");
const { storyBuild } = require("../story-build");

assert.throws(
    () =>
        storyBuild({ serving: true, mode: "development", outputPath: "dist" }),
    /isolated/
);
const releasedCatalog = storyBuild({
    serving: false,
    mode: "development",
}).comics();
assert(
    releasedCatalog.every((season) =>
        season.episodes.every((episode) => episode.publication === "released")
    )
);
assert.deepEqual(
    storyBuild({ serving: true, mode: "production" }).comics(),
    releasedCatalog
);
const html = (p) => p.userOptions.templateContent;

function fixture(run) {
    const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "comic-sources-"));
    const root = path.join(temporary, "sources"),
        output = path.join(temporary, "generated");
    const season = {
        sequence: 1,
        title: "Fixture season",
        summary: "Fixture only.",
        episodes: ["episode-1", "episode-2"],
    };
    const episode = {
        title: "First",
        summary: "Fixture episode.",
        publication: "released",
        pageCount: 1,
        source: "episode.ts",
        art: ["art/scenes.py"],
        cover: "cover",
        coverAlt: "Fixture cover.",
    };
    const put = (name, data) => {
        const file = path.join(root, name);
        fs.mkdirSync(path.dirname(file), { recursive: true });
        fs.writeFileSync(
            file,
            typeof data === "string" ? data : JSON.stringify(data)
        );
    };
    put("season-1/comic.json", season);
    put("season-1/episode-1/episode.json", episode);
    put("season-1/episode-2/episode.json", {
        ...episode,
        title: "Private canary",
        publication: "draft",
        pageCount: 2,
    });
    for (const id of season.episodes) {
        put(
            `season-1/${id}/episode.ts`,
            "throw new Error('An excluded draft must not execute'); export default {};\n"
        );
        put(
            `season-1/${id}/art/scenes.py`,
            "raise Exception('An excluded draft must not execute')\n"
        );
    }
    const called = [];
    const generate = (episode, directory) => {
        called.push(episode.id);
        fs.mkdirSync(directory, { recursive: true });
        fs.writeFileSync(path.join(directory, "cover.svg"), "<svg/>");
        fs.writeFileSync(
            path.join(directory, "scene.svg"),
            `<svg><title>${episode.title}</title></svg>`
        );
        fs.writeFileSync(
            path.join(directory, "unreferenced.svg"),
            "Never copied"
        );
        fs.writeFileSync(
            path.join(directory, "pages.json"),
            JSON.stringify([
                {
                    id: "one",
                    title: "A page",
                    transcript: "Literal <b>words</b>.",
                    definition: {
                        layout: "art",
                        image: "scene.svg",
                        alt: "Fixture art.",
                    },
                },
            ])
        );
    };
    const compile = (includeDrafts = false) =>
        compileComics({ root, output, includeDrafts, generate });
    try {
        run({ root, output, season, episode, put, called, generate, compile });
    } finally {
        fs.rmSync(temporary, { recursive: true, force: true });
    }
}

fixture((f) => {
    const source = discoverComics(f.root);
    assert.equal(source[0].episodes[1].publication, "draft");
    assert.deepEqual(f.called, [], "Discovery never executes story code");
    const production = f.compile();
    assert.deepEqual(f.called, ["episode-1"]);
    assert.equal(production.comics[0].episodes.length, 1);
    assert.equal(production.assets.size, 2);
    assert(!JSON.stringify(production.comics).includes("Private canary"));
    assert(!JSON.stringify(production.comics).includes("episode.ts"));
    assert(!JSON.stringify(production.comics).includes("scenes.py"));
    assert(
        ![...production.assets.keys()].some((n) =>
            /episode-2|unreferenced/.test(n)
        )
    );
    const local = f.compile(true);
    assert.equal(local.comics[0].episodes.length, 2);
    assert.equal(local.comics[0].episodes[1].number, 2);
    assert.equal(local.comics[0].cover, "episode-1/cover.svg");
    assert.deepEqual(
        releasedEpisodes(local.comics).map((e) => e.episode.id),
        ["episode-1"]
    );
    for (const prefix of ["/", "/nova-protocol/"]) {
        const index = html(comicIndexPage(local.comics, prefix));
        assert.equal(
            index.split(`href="${prefix}story/season-1/"`).length - 1,
            1
        );
        assert(!index.includes('class="story-episode"'));
        assert(!index.includes("Private canary"));
        assert(
            index.includes(
                '2 episodes / <span class="story-draft-label">1 draft</span>'
            )
        );
        assert.equal((index.match(/<img /g) || []).length, 1);
        const page = comicSeasonPage(local.comics[0], prefix);
        const season = html(page);
        assert.deepEqual(page.userOptions.chunks, ["story"]);
        assert(!season.includes('http-equiv="refresh"'));
        assert(
            season.includes(
                `class="story-back" href="${prefix}story/">Story</a>`
            )
        );
        assert(season.includes("Private canary"));
        assert(
            season.includes(
                'class="story-draft-label">Draft</span> - 1 of 2 pages illustrated'
            )
        );
        assert.equal(
            (season.match(/<img /g) || []).length,
            2,
            "Only episode thumbnails appear on the season page"
        );
        for (const { id } of local.comics[0].episodes) {
            const link = `href="${prefix}story/season-1/${id}/"`;
            assert(
                !index.includes(link),
                "The library lists seasons, not episodes"
            );
            assert.equal(
                season.split(link).length - 1,
                1,
                "Each episode has one entry point on its season page"
            );
        }
        assert(season.indexOf("Episode 01") < season.indexOf("Episode 02"));
        for (const document of [index, season])
            for (const removed of [
                "Latest episode",
                "Earlier seasons",
                "demonstration",
                "Preview episode",
                "Start reading",
                "Season contents",
                "story-season__cover",
                "Read &gt;",
            ])
                assert(!document.includes(removed));
        const [released, draft] = local.comics[0].episodes;
        assert(
            !html(comicReaderPage(local.comics[0], released, prefix)).includes(
                "data-layout-toggle"
            )
        );
        const reader = html(comicReaderPage(local.comics[0], draft, prefix));
        assert(!reader.includes("data-layout-toggle"));
        assert(reader.includes("data-art-toggle"));
        assert(reader.includes('data-reader-dialog="panels"'));
        assert(reader.includes('data-reader-dialog="transcript"'));
        assert(reader.includes('<dialog class="comic-reader-dialog"'));
        assert(!reader.includes('<details class="story-page-text"'));
        assert(!reader.includes('<details class="story-panel-images"'));
        assert(reader.includes("Previous episode: First"));
        assert(
            reader.includes(
                `href="${prefix}story/season-1/">Fixture season</a>`
            )
        );
        assert(reader.includes('aria-expanded="false">Pages</button>'));
        assert(!reader.includes("Season contents"));
        assert(reader.includes("Literal &lt;b&gt;words&lt;/b&gt;."));
        const publicIndex = html(comicIndexPage(production.comics, prefix));
        assert(!publicIndex.includes("Private canary"));
        assert(!publicIndex.includes("story-draft-label"));
        assert(publicIndex.includes("1 episode"));
        const publicSeason = html(
            comicSeasonPage(production.comics[0], prefix)
        );
        assert(!publicSeason.includes("Private canary"));
        assert(!publicSeason.includes("episode-2"));
        assert(!publicSeason.includes("story-draft-label"));
    }
    assert.deepEqual(comicRoutes(local.comics), [
        "story/season-1/episode-1",
        "story/season-1/episode-2",
        "story/season-1",
    ]);
    f.compile();
    assert(
        !fs.existsSync(path.join(f.output, "season-1/episode-2")),
        "Removed selection has no stale generated assets"
    );
    const metadata = { ...f.episode, publication: "draft" };
    f.put("season-1/episode-1/episode.json", metadata);
    const empty = f.compile();
    assert.deepEqual(empty.comics, []);
    assert.equal(empty.assets.size, 0);
    assert(
        html(comicIndexPage([], "/")).includes(
            "No story episodes have been released"
        )
    );
    assert(
        html(comicIndexPage(f.compile(true).comics, "/")).includes(
            'class="story-draft-label">2 drafts</span>'
        )
    );
    assert.throws(
        () =>
            compileComics({
                root: f.root,
                output: f.root,
                generate: f.generate,
            }),
        /isolated/
    );
});

for (const field of ["sequence", "title", "summary", "episodes"])
    fixture((f) => {
        const invalid = { ...f.season };
        delete invalid[field];
        f.put("season-1/comic.json", invalid);
        assert.throws(() => discoverComics(f.root), /has no/);
    });
for (const field of [
    "title",
    "summary",
    "publication",
    "pageCount",
    "source",
    "art",
    "cover",
    "coverAlt",
])
    fixture((f) => {
        const invalid = { ...f.episode };
        delete invalid[field];
        f.put("season-1/episode-1/episode.json", invalid);
        assert.throws(() => discoverComics(f.root), /has no/);
    });
for (const change of [
    { publication: "planned" },
    { pageCount: 0 },
    { pageCount: 1.5 },
    { source: "../episode.ts" },
    { source: "absent.ts" },
    { art: ["../scenes.py"] },
    { art: ["art/scenes.py", "art/scenes.py"] },
    { art: [] },
    { cover: "../cover" },
    { title: " " },
    { unknown: true },
])
    fixture((f) => {
        f.put("season-1/episode-1/episode.json", { ...f.episode, ...change });
        assert.throws(() => discoverComics(f.root));
    });
fixture((f) => {
    f.put("season-1/comic.json", {
        ...f.season,
        episodes: ["episode-1", "episode-1"],
    });
    assert.throws(() => discoverComics(f.root), /duplicate/);
});
fixture((f) => {
    f.put("season-1/comic.json", { ...f.season, episodes: ["../escape"] });
    assert.throws(() => discoverComics(f.root), /episode id/);
});
fixture((f) => {
    f.put("season-1/comic.json", { ...f.season, episodes: ["episode-1"] });
    assert.throws(() => discoverComics(f.root), /Unregistered/);
});
fixture((f) => {
    f.put("season-1/episode-1/episode.json", { ...f.episode, pageCount: 2 });
    assert.throws(() => f.compile(), /fully illustrated/);
});
fixture((f) => {
    f.put("season-1/episode-2/episode.json", { ...f.episode, pageCount: 2 });
    assert.throws(() => f.compile(), /fully illustrated/);
});
fixture((f) => {
    f.put("season-1/episode-1/episode.json", {
        ...f.episode,
        publication: "draft",
    });
    f.put("season-1/episode-2/episode.json", f.episode);
    assert.throws(() => discoverComics(f.root), /precede drafts/);
});
fixture((f) => {
    f.put("season-2/comic.json", { ...f.season, episodes: ["episode-1"] });
    f.put("season-2/episode-1/episode.json", f.episode);
    f.put("season-2/episode-1/episode.ts", "export default {};");
    f.put("season-2/episode-1/art/scenes.py", "pass");
    assert.throws(() => discoverComics(f.root), /Duplicate season sequence/);
});
fixture((f) => {
    f.put("season-1/episode-2/episode.json", { ...f.episode, title: "Second" });
    f.put("season-2/comic.json", {
        ...f.season,
        sequence: 2,
        episodes: ["episode-1"],
    });
    f.put("season-2/episode-1/episode.json", { ...f.episode, title: "Third" });
    f.put("season-2/episode-1/episode.ts", "export default {};");
    f.put("season-2/episode-1/art/scenes.py", "pass");
    const { comics } = f.compile();
    const index = html(comicIndexPage(comics, "/nova-protocol/"));
    assert(!index.includes("Earlier seasons"));
    assert(!index.includes("Latest episode"));
    assert(index.includes('href="/nova-protocol/story/season-2/"'));
    assert(
        index.indexOf('href="/nova-protocol/story/season-1/"') <
            index.indexOf('href="/nova-protocol/story/season-2/"')
    );
    assert.equal((index.match(/class="story-season"/g) || []).length, 2);
    assert(!index.includes('class="story-episode"'));
    assert.equal(
        (index.match(/<img /g) || []).length,
        2,
        "One image per season"
    );
    const first = html(comicSeasonPage(comics[0], "/nova-protocol/"));
    const second = html(comicSeasonPage(comics[1], "/nova-protocol/"));
    assert.equal((first.match(/class="story-episode"/g) || []).length, 2);
    assert.equal((second.match(/class="story-episode"/g) || []).length, 1);
    assert(
        first.indexOf(">First</strong>") < first.indexOf(">Second</strong>")
    );
    assert(!first.includes(">Third</strong>"));
    assert(!second.includes(">First</strong>"));
    assert.equal(
        html(comicIndexPage([...comics].reverse(), "/nova-protocol/")),
        index,
        "Season order is numeric, not discovery order"
    );
    assert(
        html(
            comicReaderPage(comics[0], comics[0].episodes[1], "/", comics)
        ).includes("Next episode: Third")
    );
    assert(
        html(
            comicReaderPage(comics[1], comics[1].episodes[0], "/", comics)
        ).includes("Previous episode: Second")
    );
    f.put("season-2/episode-1/episode.json", {
        ...f.episode,
        publication: "draft",
        title: "Private third",
    });
    const publicSelection = f.compile().comics;
    assert.equal(
        publicSelection.length,
        1,
        "A draft-only season stays out of public browsing"
    );
    assert(
        !html(comicIndexPage(publicSelection, "/")).includes(
            'href="/story/season-2/"'
        )
    );
    assert(
        !comicRoutes(publicSelection).some((route) =>
            route.startsWith("story/season-2")
        )
    );
    const localSelection = f.compile(true).comics;
    assert.equal(localSelection.length, 2);
    assert(
        html(comicIndexPage(localSelection, "/")).includes(
            'class="story-draft-label">1 draft</span>'
        )
    );
    assert(
        html(comicSeasonPage(localSelection[1], "/")).includes("Private third")
    );
});
fixture((f) => {
    f.put("season-1/episode-2/episode.json", { ...f.episode, title: "Second" });
    f.put("season-1/comic.json", {
        ...f.season,
        episodes: ["episode-2", "episode-1"],
    });
    const { comics } = f.compile();
    const season = html(comicSeasonPage(comics[0], "/"));
    assert(
        season.indexOf(">Second</strong>") < season.indexOf(">First</strong>"),
        "Episode order comes from metadata, not ids"
    );
    assert.equal(comics[0].cover, "episode-2/cover.svg");
});
fixture((f) => {
    f.put("season-1/episode-1/episode.json", {
        ...f.episode,
        title: '</script><script>alert("x")</script>',
    });
    const { comics } = f.compile();
    const reader = html(comicReaderPage(comics[0], comics[0].episodes[0], "/"));
    assert(!reader.includes('<script>alert("x")</script>'));
    assert(reader.includes("\\u003c/script>"));
});

fixture((f) => {
    f.put("season-1/comic.json", {
        ...f.season,
        title: '<Season & "one">',
        summary: "<b>Not markup</b>",
    });
    const { comics } = f.compile();
    for (const document of [
        html(comicIndexPage(comics, "/")),
        html(comicSeasonPage(comics[0], "/")),
        html(comicReaderPage(comics[0], comics[0].episodes[0], "/")),
    ]) {
        assert(document.includes("&lt;Season &amp; &quot;one&quot;&gt;"));
        assert(!document.includes("<Season"));
        assert(!document.includes("<b>Not markup</b>"));
    }
});

const page = {
    layout: "lettered",
    image: "scene.svg",
    alt: "A scene",
    width: 1500,
    height: 1000,
    palette: { ink: "#101010", balloon: "#ffffff", speaker: "#008844" },
    balloons: [
        {
            x: 20,
            y: 30,
            width: 240,
            speaker: "Crew",
            text: "Hold position.",
            tip: [90, 180],
            side: "bottom",
        },
    ],
};
const asset = (value) => assert.equal(value, "scene.svg");
validatePageDefinition(page, asset);
validatePageDefinition({ ...page, balloons: [] }, asset);
for (const mutate of [
    (p) => delete p.width,
    (p) => (p.extra = true),
    (p) => (p.width = Infinity),
    (p) => (p.height = 0),
    (p) => (p.palette.ink = ["#101010"]),
    (p) => (p.palette.balloon = "red"),
    (p) => (p.balloons[0].x = -1),
    (p) => (p.balloons[0].width = 2000),
    (p) => (p.balloons[0].tip = [0]),
    (p) => (p.balloons[0].side = "auto"),
    (p) => (p.balloons[0].text = " "),
    (p) => (p.balloons[0].speaker = null),
]) {
    const invalid = structuredClone(page);
    mutate(invalid);
    assert.throws(() => validatePageDefinition(invalid, asset));
}
// A scene id reaches Python only after the script has been read, so a typo or
// a renamed scene surfaces THERE. It must arrive as an authoring error naming
// every id it could not find and every id it has, not a bare KeyError on the
// first one. Nothing renders, so this costs a process start.
{
    const rejected = path.join(os.tmpdir(), "nova-unreachable-scene");
    fs.rmSync(rejected, { recursive: true, force: true });
    const generator = spawnSync(
        "python3",
        [
            path.join(__dirname, "../build-comics.py"),
            "--source",
            path.join(
                __dirname,
                "../src/comics/season-1/episode-1/art/opening.py"
            ),
            "--scenes",
            JSON.stringify(["coffee-break", "nope", "gone"]),
            "--output",
            rejected,
        ],
        { encoding: "utf8" }
    );
    assert.equal(generator.status, 1, "an unknown scene fails the generation");
    assert.match(
        generator.stderr,
        /ValueError: Unregistered scene ids: nope, gone\. Registered: assignment-window, /,
        "the error names what is missing and what is on offer"
    );
    assert(!fs.existsSync(rejected), "a rejected request renders nothing");
}

assert(
    discoverComics().length > 0,
    "Repository metadata can be discovered independently of publication state"
);
assert(
    !fs
        .readFileSync(
            path.join(__dirname, "../src/comics/comic-catalog.ts"),
            "utf8"
        )
        .includes("require.context")
);
console.log(
    "comics.test.js: metadata, publication selection, incomplete-release rejection, mixed seasons, routing, escaping, assets, and stale cleanup passed"
);
