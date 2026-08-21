import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const tauriDir = join(root, "src-tauri");
const workerDir = join(tauriDir, "fixture-worker");
const triple = execFileSync("rustc", ["--print", "host-tuple"], { encoding: "utf8" }).trim();
const release = process.argv.includes("--release");
const profile = release ? "release" : "debug";
const cargoTargetDir = resolve(root, process.env.CARGO_TARGET_DIR ?? join("src-tauri", "fixture-worker", "target"));

execFileSync("cargo", ["build", "--manifest-path", join(workerDir, "Cargo.toml"), ...(release ? ["--release"] : [])], {
  stdio: "inherit"
});

const extension = process.platform === "win32" ? ".exe" : "";
const source = join(cargoTargetDir, profile, `fixture-worker${extension}`);
const destination = join(tauriDir, "binaries", `fixture-worker-${triple}${extension}`);
mkdirSync(dirname(destination), { recursive: true });
copyFileSync(source, destination);
console.log(`Sidecar ready: ${destination}`);
