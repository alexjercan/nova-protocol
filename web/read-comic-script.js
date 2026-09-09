const path = require("node:path");
require("ts-node").register({
    transpileOnly: true,
    compilerOptions: { module: "CommonJS", moduleResolution: "Node" },
});
// A fresh process evaluates only a build-selected entry and its explicit imports.
const script = require(path.resolve(process.argv[2])).default;
process.stdout.write(JSON.stringify(script));
