const fs = require("fs");
const path = require("path");

// Injects shared header/footer partials into each generated page, replacing the
// <%= basePath %> placeholder so links resolve under the deploy subpath. A page
// opts in by including <div id="header"></div> / <div id="footer"></div>.
class HtmlPartialsPlugin {
    constructor(options) {
        this.options = options || {};
    }

    apply(compiler) {
        compiler.hooks.compilation.tap("HtmlPartialsPlugin", (compilation) => {
            const HtmlWebpackPlugin = require("html-webpack-plugin");
            const hooks = HtmlWebpackPlugin.getHooks(compilation);

            hooks.beforeEmit.tapAsync(
                "HtmlPartialsPlugin",
                (data, callback) => {
                    const basePath =
                        data.plugin.options.basePath ||
                        this.options.basePath ||
                        "/";

                    const headerPath = path.join(__dirname, "src/_header.html");
                    const footerPath = path.join(__dirname, "src/_footer.html");

                    let header = fs.existsSync(headerPath)
                        ? fs.readFileSync(headerPath, "utf8")
                        : "";
                    let footer = fs.existsSync(footerPath)
                        ? fs.readFileSync(footerPath, "utf8")
                        : "";

                    header = header.replace(/<%=\s*basePath\s*%>/g, basePath);
                    footer = footer.replace(/<%=\s*basePath\s*%>/g, basePath);

                    data.html = data.html
                        .replace('<div id="header"></div>', header)
                        .replace('<div id="footer"></div>', footer);

                    callback(null, data);
                }
            );
        });
    }
}

module.exports = HtmlPartialsPlugin;
