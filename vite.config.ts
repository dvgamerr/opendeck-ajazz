import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";

const starterpackAssets = fileURLToPath(new URL("./plugins/com.amansprojects.starterpack.sdPlugin/assets", import.meta.url));

export default defineConfig({
	plugins: [sveltekit(), tailwindcss()],
	css: {
		transformer: "lightningcss",
	},
	clearScreen: false,
	server: {
		fs: {
			allow: [starterpackAssets],
		},
		watch: {
			ignored: ["**/src-tauri/**", "**/target/**"],
		},
	},
});
