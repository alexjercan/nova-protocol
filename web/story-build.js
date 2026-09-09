const fs = require("node:fs");
const path = require("node:path");
const { compileComics, SOURCE_ROOT } = require("./comic-sources");

function files(directory) {
    return fs
        .readdirSync(directory, { withFileTypes: true })
        .flatMap((entry) => {
            const file = path.join(directory, entry.name);
            if (entry.name === "__pycache__") return [];
            return entry.isDirectory() ? files(file) : [file];
        });
}

/** Build one selected catalog; normal serving includes drafts, ordinary builds never do. */
function storyBuild({ serving, mode, outputPath }) {
    if (serving && outputPath)
        throw new Error(
            "The development server owns its isolated output directory"
        );
    const includeDrafts = serving && mode !== "production";
    const cache = path.join(
        __dirname,
        ".cache/story",
        includeDrafts ? "development" : "release"
    );
    const generated = path.join(cache, "generated");
    let current;
    function generate() {
        current = compileComics({ output: generated, includeDrafts });
    }
    generate();
    return {
        site: path.join(__dirname, ".cache/story/development/site"),
        comics: () => current.comics,
        plugin: {
            apply(compiler) {
                compiler.hooks.beforeCompile.tap("StoryBuild", generate);
                compiler.hooks.thisCompilation.tap(
                    "StoryBuild",
                    (compilation) => {
                        compilation.hooks.fullHash.tap("StoryBuild", (hash) => {
                            hash.update(JSON.stringify(current.comics));
                            for (const [name, file] of current.assets)
                                hash.update(name).update(fs.readFileSync(file));
                        });
                        compilation.hooks.processAssets.tap(
                            {
                                name: "StoryBuild",
                                stage: compiler.webpack.Compilation
                                    .PROCESS_ASSETS_STAGE_ADDITIONAL,
                            },
                            () => {
                                for (const [name, file] of current.assets) {
                                    compilation.emitAsset(
                                        name,
                                        new compiler.webpack.sources.RawSource(
                                            fs.readFileSync(file)
                                        )
                                    );
                                    compilation.fileDependencies.add(file);
                                }
                            }
                        );
                        for (const directory of [
                            SOURCE_ROOT,
                            path.resolve(
                                __dirname,
                                "../scripts/nova_illustration"
                            ),
                        ]) {
                            compilation.contextDependencies.add(directory);
                            for (const file of files(directory))
                                compilation.fileDependencies.add(file);
                        }
                        for (const name of [
                            "build-comics.py",
                            "read-comic-script.js",
                        ])
                            compilation.fileDependencies.add(
                                path.join(__dirname, name)
                            );
                    }
                );
            },
        },
    };
}
module.exports = { storyBuild };
