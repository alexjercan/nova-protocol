const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const {
    CHAPTER_STATES,
    chapterCounts,
    discoverComics,
    loadComic,
    pagesOf,
    referencedAssets,
    validateComic,
} = require("../comic-build");

// Every comic under test lives in a temporary root, so the assertions hold
// with no comic shipped from `src/comics`.
function temporaryRoots() {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "comic-test-"));
    return {
        roots: {
            comics: path.join(root, "comics"),
            assets: path.join(root, "assets"),
        },
        cleanup: () => fs.rmSync(root, { recursive: true, force: true }),
    };
}

// A root that holds only engine modules and no comic directories.
{
    const { roots, cleanup } = temporaryRoots();
    fs.mkdirSync(roots.comics, { recursive: true });
    fs.writeFileSync(path.join(roots.comics, "comic-page.ts"), "export {};\n");
    try {
        assert.deepEqual(
            discoverComics(roots),
            [],
            "a comics root with no comic directories is an empty archive"
        );
    } finally {
        cleanup();
    }
}

function fixture(mutate) {
    const { roots, cleanup } = temporaryRoots();
    const comicDir = path.join(roots.comics, "fixture");
    const assetDir = path.join(roots.assets, "fixture");
    fs.mkdirSync(path.join(comicDir, "pages"), { recursive: true });
    fs.mkdirSync(assetDir, { recursive: true });
    fs.writeFileSync(path.join(assetDir, "cover.svg"), "<svg/>");
    fs.writeFileSync(path.join(assetDir, "board.svg"), "<svg/>");
    fs.writeFileSync(
        path.join(comicDir, "pages", "one.ts"),
        'export default svgAsset("board.svg", { alt: "A board." });\n'
    );
    fs.writeFileSync(path.join(comicDir, "pages", "two.ts"), "export {};\n");
    const comic = {
        title: "Fixture",
        summary: "A fixture comic.",
        status: "Draft",
        cover: "cover.svg",
        coverAlt: "A cover.",
        chapters: [
            {
                id: "one",
                title: "One",
                state: "playable",
                pages: [{ id: "one", title: "One", source: "pages/one" }],
            },
            {
                id: "two",
                title: "Two",
                state: "planned",
                pages: [{ id: "two", title: "Two", source: "pages/two" }],
            },
        ],
    };
    if (mutate) mutate({ comic, comicDir, assetDir });
    fs.writeFileSync(path.join(comicDir, "comic.json"), JSON.stringify(comic));
    return { comic, roots, cleanup };
}

function rejects(mutate, pattern, message) {
    const { comic, roots, cleanup } = fixture(mutate);
    try {
        assert.throws(
            () => validateComic("fixture", comic, roots),
            pattern,
            message
        );
    } finally {
        cleanup();
    }
}

{
    const { roots, cleanup } = fixture();
    try {
        const loaded = loadComic("fixture", roots);
        assert.equal(loaded.path, "fixture", "a valid manifest loads");
        assert.equal(
            "slug" in loaded,
            false,
            "the directory is the only index"
        );
        const pages = pagesOf(loaded);
        assert.deepEqual(
            pages.map((page) => [page.id, page.chapterId, page.state]),
            [
                ["one", "one", "playable"],
                ["two", "two", "planned"],
            ],
            "pages carry their chapter id and inherit its state"
        );
        assert.ok(
            pages.every((page) => CHAPTER_STATES.includes(page.state)),
            "every page state is a known chapter state"
        );
        assert.deepEqual(
            chapterCounts(loaded),
            { playable: 1, planned: 1, frame: 0 },
            "chapters are counted by state"
        );
        assert.deepEqual(
            discoverComics(roots).map((comic) => comic.path),
            ["fixture"],
            "discovery walks the comics root"
        );
        assert.deepEqual(
            referencedAssets(path.join(roots.comics, "fixture/pages/one.ts")),
            ["board.svg"],
            "string-literal asset names are collected from a module"
        );
        assert.throws(
            () => validateComic("Not An Id", loaded, roots),
            /is not a valid id/,
            "a directory name that is not a valid public id is rejected"
        );
    } finally {
        cleanup();
    }
}

// The validator is the whole guarantee `docs/development.md` advertises, so
// what it REFUSES matters more than what it accepts.
rejects(
    ({ comic }) => delete comic.chapters[0].id,
    /invalid\/duplicate chapter id/,
    "a chapter without an id is rejected"
);
rejects(
    ({ comic }) => (comic.chapters[1].id = "one"),
    /invalid\/duplicate chapter id/,
    "a repeated chapter id is rejected"
);
rejects(
    ({ comic }) => delete comic.chapters[1].state,
    /needs a state/,
    "a chapter without a state is rejected"
);
rejects(
    ({ comic }) => (comic.chapters[1].state = "someday"),
    /needs a state/,
    "an unknown chapter state is rejected"
);
rejects(
    ({ comic }) => delete comic.coverAlt,
    /has no coverAlt/,
    "a cover without alt text is rejected"
);
rejects(
    ({ comic }) => delete comic.chapters[0].pages[0].id,
    /invalid\/duplicate page id/,
    "a page without an id is rejected"
);
rejects(
    ({ comic }) => (comic.chapters[1].pages[0].id = "one"),
    /invalid\/duplicate page id/,
    "a page id repeated across chapters is rejected"
);
rejects(
    ({ comic }) => (comic.chapters[0].pages[0].source = "one"),
    /invalid source/,
    "a page source outside pages/ is rejected"
);
rejects(
    ({ comic }) => (comic.chapters[0].pages[0].source = "pages/deep/one"),
    /invalid source/,
    "a page source nested under pages/ is rejected"
);
rejects(
    ({ comic }) => (comic.chapters[0].pages[0].source = "pages/missing"),
    /page source is missing/,
    "a page without its module is rejected"
);
rejects(
    ({ comic }) => (comic.cover = "nowhere.svg"),
    /cover asset is missing/,
    "a missing cover asset is rejected"
);
rejects(
    ({ comic }) => (comic.cover = "../cover.svg"),
    /cover asset is missing/,
    "a cover outside the comic's asset folder is rejected"
);
rejects(
    ({ comicDir }) =>
        fs.writeFileSync(
            path.join(comicDir, "pages", "two.ts"),
            'export default svgAsset("ghost.svg", { alt: "Missing." });\n'
        ),
    /two\.ts asset is missing: ghost\.svg/,
    "a page module that names a missing asset is rejected"
);

console.log("comics.test.js: all assertions passed");
