import js from "@eslint/js";
import boundaries from "eslint-plugin-boundaries";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

/**
 * Feature-Sliced Design layers, lowest first.
 *
 * A layer may depend only on itself and on the layers declared before it. The
 * order is the single source of truth for the import rules below.
 */
const FSD_LAYERS = ["shared", "entities", "features", "widgets", "pages", "app"];

/** Layers a layer of the given index may import. */
function layersUpTo(layer) {
	return FSD_LAYERS.slice(0, FSD_LAYERS.indexOf(layer) + 1);
}

/**
 * Element descriptors.
 *
 * Each slice on a layer (`src/widgets/system-status`) is an element of that
 * layer's type with the slice name captured, which lets the rules restrict
 * cross-slice imports to a slice's public API.
 */
const elements = [
	...FSD_LAYERS.map((layer) => ({
		type: layer,
		pattern: `src/${layer}/*`,
		capture: ["slice"],
	})),
	{ type: "test", pattern: "src/test/*" },
];

export default tseslint.config(
	{
		ignores: [
			"dist",
			"node_modules",
			"src/shared/api/generated/schema.d.ts",
			"src/shared/api/generated/openapi.json",
		],
	},
	{
		files: ["**/*.{ts,tsx}"],
		extends: [
			js.configs.recommended,
			...tseslint.configs.recommendedTypeChecked,
			...tseslint.configs.stylisticTypeChecked,
		],
		languageOptions: {
			ecmaVersion: 2023,
			globals: { ...globals.browser, ...globals.node },
			parserOptions: {
				projectService: true,
				tsconfigRootDir: import.meta.dirname,
			},
		},
		plugins: {
			"react-hooks": reactHooks,
			"react-refresh": reactRefresh,
		},
		rules: {
			...reactHooks.configs.recommended.rules,
			"react-refresh/only-export-components": ["warn", { allowConstantExport: true }],
			"@typescript-eslint/consistent-type-imports": [
				"error",
				{ prefer: "type-imports", fixStyle: "inline-type-imports" },
			],
			"@typescript-eslint/no-unused-vars": [
				"error",
				{ argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
			],
			"@typescript-eslint/no-misused-promises": [
				"error",
				{ checksVoidReturn: { attributes: false } },
			],
			"@typescript-eslint/switch-exhaustiveness-check": "error",
			eqeqeq: ["error", "always"],
			"no-console": ["warn", { allow: ["warn", "error"] }],
		},
	},
	{
		// Architectural boundaries.
		files: ["src/**/*.{ts,tsx}"],
		plugins: { boundaries },
		settings: {
			"boundaries/elements": elements,
			"boundaries/include": ["src/**/*.{ts,tsx}"],
			// Without a resolver the plugin cannot follow the `@/` alias and
			// silently classifies aliased imports as external, which would disable
			// the layer checks entirely.
			"import/resolver": {
				typescript: {
					alwaysTryTypes: true,
					project: "./tsconfig.json",
				},
			},
		},
		rules: {
			"boundaries/dependencies": [
				"error",
				{
					default: "disallow",
					policies: [
						// Layer direction: a layer imports itself and lower layers only.
						...FSD_LAYERS.map((layer) => ({
							allow: {
								from: { element: { type: layer } },
								to: { element: { types: layersUpTo(layer) } },
							},
						})),
						// A slice exposes a public API through `index.ts`. Importing
						// `@/entities/system-status/model` from another slice is
						// therefore rejected even when the layer direction is allowed.
						{
							disallow: {
								to: {
									element: {
										type: FSD_LAYERS,
										internalPath: "!(index.ts)",
										captured: { slice: "!{{ from.element.captured.slice }}" },
									},
								},
							},
						},
						// Tests may reach whatever they verify.
						{ allow: { from: { element: { type: "test" } } } },
						{ allow: { to: { element: { type: "test" } } } },
					],
				},
			],
		},
	},
	{
		files: ["src/**/*.test.{ts,tsx}", "src/test/**/*.{ts,tsx}"],
		rules: {
			"@typescript-eslint/no-unsafe-assignment": "off",
			"@typescript-eslint/no-unsafe-member-access": "off",
			"@typescript-eslint/no-unsafe-call": "off",
			"@typescript-eslint/no-unsafe-argument": "off",
			"@typescript-eslint/no-non-null-assertion": "off",
		},
	},
	{
		files: ["vite.config.ts"],
		languageOptions: {
			globals: { ...globals.node },
		},
		rules: {
			"@typescript-eslint/no-unsafe-assignment": "off",
		},
	},
);
