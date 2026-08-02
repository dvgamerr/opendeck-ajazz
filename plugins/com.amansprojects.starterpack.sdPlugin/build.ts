import { cp, rm } from "node:fs/promises";
import { join } from "node:path";

const [outDir, target] = process.argv.slice(2);

if (!outDir || !target) {
	console.error("Usage: bun run build.ts <output-directory> <rust-target>");
	process.exit(1);
}

await rm(outDir, { recursive: true, force: true });
await cp("assets", outDir, { recursive: true });

const platform = process.platform === "win32" ? "windows" : process.platform;
const cargo = Bun.spawn(["cargo", "install", "--force", "--path", ".", "--target", target, "--root", join(outDir, platform)], {
	stdin: "inherit",
	stdout: "inherit",
	stderr: "inherit",
});

process.exit(await cargo.exited);
