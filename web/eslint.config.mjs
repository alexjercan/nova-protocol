import eslint from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
    eslint.configs.recommended,
    ...tseslint.configs.recommendedTypeChecked,
    {
        languageOptions: {
            parserOptions: {
                projectService: true,
                tsconfigRootDir: import.meta.dirname,
            },
        },
    },
    {
        rules: {
            "@typescript-eslint/no-unused-vars": [
                "warn",
                {
                    argsIgnorePattern: "^_",
                    varsIgnorePattern: "^_",
                },
            ],
            "@typescript-eslint/no-explicit-any": "warn",
            "no-console": ["warn", { allow: ["warn", "error"] }],
            "@typescript-eslint/no-floating-promises": "off",
            "@typescript-eslint/no-require-imports": "off",
        },
    },
    {
        ignores: [
            "dist/",
            "node_modules/",
            "webpack.config.js",
            "webpack-partials.js",
            "eslint.config.mjs",
        ],
    }
);
