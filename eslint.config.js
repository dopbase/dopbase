import pluginVue from "eslint-plugin-vue";
import globals from "globals";
import eslintConfigPrettier from "eslint-config-prettier";
import tseslint from "typescript-eslint";

export default [
  {
    ignores: [
      "android/app/build/",
      "android/app/src/main/assets/web/",
      "coverage/",
      "dist/",
      "ios/Web/",
    ],
  },
  ...tseslint.configs.recommended,
  ...pluginVue.configs["flat/recommended"],
  {
    files: ["**/*.vue"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    rules: {
      // override/add rules settings here, such as:
      "vue/multi-word-component-names": "off",
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["../*", "../**"],
              message:
                "Parent directory imports are forbidden. Use local files (./) or absolute aliases (~/).",
            },
          ],
        },
      ],
    },
    languageOptions: {
      sourceType: "module",
      globals: {
        ...globals.browser,
      },
    },
  },
  {
    files: ["src/components/ui/**"],
    rules: {
      // UI atoms must stay dumb: no business logic, stores, services, or
      // page-level imports may leak into the design system layer.
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["../*", "../**"],
              message:
                "Parent directory imports are forbidden. Use local files (./) or absolute aliases (~/).",
            },
            {
              group: [
                "~/stores",
                "~/stores/*",
                "~/services",
                "~/services/*",
                "~/pages",
                "~/pages/*",
                "~/router",
              ],
              message:
                "UI components must be presentation-only and cannot import business logic.",
            },
          ],
        },
      ],
    },
  },
  eslintConfigPrettier,
];
