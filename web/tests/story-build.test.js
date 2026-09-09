const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const web = path.resolve(__dirname, "..");
const root = path.dirname(web);
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "comic-deployment-"));
const fixture = path.join(temporary, "web");
const put = (name, content) => {
    const file = path.join(fixture, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(
        file,
        typeof content === "string" ? content : JSON.stringify(content)
    );
};
const walk = (directory) =>
    fs
        .readdirSync(directory, { withFileTypes: true })
        .flatMap((p) =>
            p.isDirectory()
                ? walk(path.join(directory, p.name))
                : [path.join(directory, p.name)]
        );
const build = (target, prefix = "/") =>
    execFileSync(
        process.execPath,
        [
            path.join(web, "node_modules/webpack-cli/bin/cli.js"),
            "--config",
            "webpack.config.js",
            "--env",
            `target=${target}`,
            "--mode",
            "production",
        ],
        {
            cwd: fixture,
            env: { ...process.env, PUBLIC_PATH: prefix },
            stdio: "pipe",
            encoding: "utf8",
            maxBuffer: 8_000_000,
        }
    );
try {
    fs.cpSync(web, fixture, {
        recursive: true,
        filter: (source) => {
            const relative = path.relative(web, source).split(path.sep);
            if (
                [
                    "node_modules",
                    "dist",
                    "dist-story",
                    ".cache",
                    ".test-out",
                    "tests",
                ].includes(relative[0])
            )
                return false;
            if (
                relative[0] === "src" &&
                relative[1] === "assets" &&
                relative.length > 2
            )
                return relative[2] === "promptfont.woff2";
            return true;
        },
    });
    fs.symlinkSync(
        path.join(web, "node_modules"),
        path.join(fixture, "node_modules"),
        "dir"
    );
    fs.cpSync(
        path.join(root, "scripts/nova_illustration"),
        path.join(temporary, "scripts/nova_illustration"),
        { recursive: true, filter: (p) => !p.includes("__pycache__") }
    );
    fs.mkdirSync(path.join(temporary, "assets/input-prompts"), {
        recursive: true,
    });
    fs.writeFileSync(
        path.join(temporary, "assets/input-prompts/fixture.svg"),
        '<svg xmlns="http://www.w3.org/2000/svg"/>'
    );
    const episode = "src/comics/season-1/episode-1/";
    fs.renameSync(
        path.join(fixture, episode, "episode.ts"),
        path.join(fixture, episode, "full-episode.ts")
    );
    put(
        episode + "episode.ts",
        'import script from "./full-episode"; export default { ...script, pages: script.pages.slice(0, 7) };'
    );
    const metadata = JSON.parse(
        fs.readFileSync(path.join(fixture, episode, "episode.json"))
    );
    put(episode + "episode.json", {
        ...metadata,
        publication: "released",
        pageCount: 7,
    });
    const season = JSON.parse(
        fs.readFileSync(path.join(fixture, "src/comics/season-1/comic.json"))
    );
    put("src/comics/season-1/comic.json", {
        ...season,
        episodes: ["episode-1", "episode-2"],
    });
    put("src/comics/season-1/episode-2/episode.json", {
        ...metadata,
        publication: "draft",
        pageCount: 1,
        source: "episode.ts",
        art: ["art/scenes.py"],
        title: "FUTURE_PRIVATE_CANARY",
    });
    put(
        "src/comics/season-1/episode-2/episode.ts",
        'throw new Error("Excluded TS executed");'
    );
    put(
        "src/comics/season-1/episode-2/art/scenes.py",
        'raise Exception("Excluded Python executed")'
    );
    const isolated = ["markdown.js", "src/docs-manifest.js", "src/news.ts"];
    const originals = isolated.map((name) =>
        fs.readFileSync(path.join(fixture, name))
    );
    isolated.forEach((name) =>
        put(name, 'throw new Error("Unrelated website code executed");')
    );
    for (const prefix of ["/", "/nova-protocol/"]) {
        build("story", prefix);
        const output = path.join(fixture, "dist-story");
        assert.deepEqual(fs.readdirSync(output), ["story"]);
        assert(fs.existsSync(path.join(output, "story/reader.js")));
        assert(
            fs.existsSync(path.join(output, "story/assets/promptfont.woff2"))
        );
        assert(fs.existsSync(path.join(output, "story/favicon.svg")));
        assert(
            fs.existsSync(
                path.join(output, "story/season-1/episode-1/index.html")
            )
        );
        assert(!fs.existsSync(path.join(output, "story/season-1/episode-2")));
        for (const file of walk(output).filter((p) =>
            /\.(html|js|map|json)$/.test(p)
        )) {
            const text = fs.readFileSync(file, "utf8");
            for (const secret of [
                "FUTURE_PRIVATE_CANARY",
                "Excluded TS executed",
                "Excluded Python executed",
                "Unrelated website code executed",
                "funding-refusal",
            ])
                assert(!text.includes(secret), `${secret} in ${file}`);
            if (file.endsWith(".html")) {
                for (const [, resource] of text.matchAll(
                    /(?:src|rel="icon" href)="([^"]+)"/g
                )) {
                    assert(resource.startsWith(prefix + "story/"), resource);
                    assert(
                        fs.existsSync(
                            path.join(output, resource.slice(prefix.length))
                        ),
                        resource
                    );
                }
            }
        }
        if (process.env.NOVA_COMIC_TEST_OUTPUT)
            fs.cpSync(
                output,
                path.resolve(
                    process.env.NOVA_COMIC_TEST_OUTPUT,
                    prefix === "/" ? "root" : "prefix"
                ),
                { recursive: true }
            );
    }
    isolated.forEach((name, i) => put(name, originals[i].toString()));
    put(
        episode + "episode.ts",
        'throw new Error("Site build executed the comic");'
    );
    build("site");
    const site = path.join(fixture, "dist");
    assert(fs.existsSync(path.join(site, "index.html")));
    assert(!fs.existsSync(path.join(site, "story")));
    assert(!fs.existsSync(path.join(site, "assets/story")));
    assert(!walk(site).some((p) => /reader\.js|story\.js/.test(p)));
    const config = require("../webpack.config");
    void (async () => {
        await assert.rejects(
            config({ target: "unknown" }, {}),
            /Unknown website build target/
        );
        await assert.rejects(
            config({ target: "story", WEBPACK_SERVE: true }, {}),
            /normal serving/
        );
    })();
    console.log(
        "Comic-only builds own /story/ at root and prefix; unrelated pages and excluded TS/Python never execute. Site-only builds never execute comics."
    );
} finally {
    fs.rmSync(temporary, { recursive: true, force: true });
}
