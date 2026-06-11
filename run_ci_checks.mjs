#!/usr/bin/env node
// Runs the CI build/test/lint pipeline. Single source of truth shared by
// .github/workflows/ci.yml and local pre-commit runs (`node run_ci_checks.mjs`).
// Mirrors the CI "build" job only — environment setup, tag versioning, artifact
// upload, deploy, and the separate check-translations gate live in the workflow,
// not here.
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const rust = join(root, "muxsocial-rust");
const web = join(root, "muxsocial-client-web");

// Order matters: wasm is built before the web steps because the web client imports
// muxsocial-client-wasm/pkg (gitignored) — same ordering as the GitHub workflow.
const steps = [
	{ name: "Rust format check", cmd: "cargo fmt --all --check", cwd: rust },
	{ name: "Rust clippy", cmd: "cargo clippy -p muxsocial-lib --all-targets -- -D warnings", cwd: rust },
	{ name: "Rust tests", cmd: "cargo test -p muxsocial-lib", cwd: rust },
	{ name: "Build wasm", cmd: "wasm-pack build muxsocial-client-wasm --release --target bundler", cwd: rust },
	{ name: "Install web dependencies", cmd: "npm ci", cwd: web },
	{ name: "Web lint (Biome)", cmd: "npm run check:ci", cwd: web },
	{ name: "Web tests (vitest)", cmd: "npm test", cwd: web },
	{ name: "Build web client", cmd: "npm run build", cwd: web },
];

for (const step of steps) {
	console.log(`\n=== ${step.name} ===\n$ ${step.cmd}  (in ${step.cwd})`);
	// shell: true so cargo/npm/wasm-pack resolve cross-platform (e.g. npm.cmd on Windows).
	const result = spawnSync(step.cmd, { cwd: step.cwd, stdio: "inherit", shell: true });
	if (result.status !== 0) {
		console.error(`\n✗ CI step failed: ${step.name} (exit ${result.status ?? `signal ${result.signal}`})`);
		process.exit(result.status ?? 1);
	}
}

console.log("\n✓ All CI checks passed.");
