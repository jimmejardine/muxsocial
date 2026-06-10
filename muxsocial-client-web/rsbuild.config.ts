import { defineConfig } from "@rsbuild/core";
import { pluginBasicSsl } from "@rsbuild/plugin-basic-ssl";
import { pluginReact } from "@rsbuild/plugin-react";

export default defineConfig({
	plugins: [pluginReact(), pluginBasicSsl()],

	dev: {
		hmr: true,
		liveReload: true,
	},

	output: {
		target: "web",
	},

	html: {
		template: "./public/index.html",
	},
});
